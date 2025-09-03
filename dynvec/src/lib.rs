#![feature(ptr_metadata, ptr_alignment_type)]
#![feature(ptr_as_ref_unchecked)]
#![feature(assert_matches)]
#![feature(type_alias_impl_trait)]

#![warn(missing_docs)]
//! Type-erased, single-type vector.
//!
//! DynVec is a growable vector whose element type is picked at runtime, yet
//! all elements share that same concrete type. This enables dynamic component
//! storage while still providing fast typed access through zero-overhead views.
//!
//! - Construction: `DynVec::new::<T>()` or `DynVec::new_with_meta(DynVecMetadata::new::<T>())`.
//! - Untyped access: `get`/`get_mut` yield `&dyn Any`/`&mut dyn Any`.
//! - Typed views: `typed::<T>()`/`typed_mut::<T>()` return a `Result` with a typed view
//!   and provide fast typed methods without re-checking the type on every call.
//! - Mutation:
//!   - Untyped: `push`/`set` take `Box<dyn Any>`; removals `swap_remove`/`pop` return an owning
//!     guard (OwnedDynVecValue) that can be consumed (e.g., `into_typed`, `push_into`, `set_into`).
//!   - Typed: use the typed views for `push`/`set`/`swap_remove`/`pop` with concrete types.
//! - Iteration/collection: supports `Extend` and `FromIterator` so you can
//!   `collect::<DynVec>()` from an iterator of `T` and `extend` typed views.
//!
//! ## Example
//!
//! ```
//! use portal_dynvec::DynVec;
//!
//! // Choose the element type at runtime
//! let mut v = DynVec::new::<i32>();
//!
//! // Fast typed access via a view
//! let mut t = v.typed_mut::<i32>().unwrap();
//! t.push(1);
//! t.push(2);
//! assert_eq!(t.len(), 2);
//! assert_eq!(*t.get(1).unwrap(), 2);
//! ```
//!
//! ## Safety
//!
//! Internally, memory is managed manually via `std::alloc`. Drop glue for elements is
//! reconstructed using stored `dyn Any` metadata. The following invariants are upheld:
//! - The backing allocation has capacity for `capacity * layout.size()` bytes, aligned to `layout.align()`.
//! - The initialized prefix `[0, len)` contains valid values of the element type.
//! - Elements are dropped exactly once on `clear`/`set`/`swap_remove`/`drop`.

use std::alloc::{ alloc, dealloc, realloc, Layout };
use std::any::{ type_name, Any, TypeId };
use std::ops::{ Deref, DerefMut };
use std::ptr::{ self, from_raw_parts, from_raw_parts_mut, NonNull };
use std::slice;
use std::ptr::Alignment;

use utils::prelude::MaybeDefault;

#[cfg(test)]
mod tests;

/// Error returned when there is mismatched between expected and received types.
#[derive(thiserror::Error, Debug)]
#[error("Type mismatch, expected '{}' but received '{}'", self.display_expected(), self.display_received())]
pub struct IncorrectTypeError {
    /// The type_name of the expected type (the one of the DynVec)
    pub expected_name: Option<&'static str>,
    /// The `TypeId` of the expected type (the one of the DynVec)
    pub expected_typeid: TypeId,
    /// The type_name of the type that was given to the function
    pub received_name: Option<&'static str>,
    /// The `TypeId` of the type that was given to the function
    pub received_typeid: TypeId,
}

impl IncorrectTypeError {
    fn display_expected(&self) -> String {
        self.expected_name.map(String::from)
            .unwrap_or_else(|| format!("{:?}", self.expected_typeid))
    }

    fn display_received(&self) -> String {
        self.received_name.map(String::from)
            .unwrap_or_else(|| format!("{:?}", self.received_typeid))
    }
}

/// Error returned when it is attempted to access an index that is out of bound.
#[derive(thiserror::Error, Debug)]
#[error("Index out of bound, {index} should be less than {len}")]
pub struct IndexOutOfBoundError {
    /// The index that was attempted
    pub index: usize,
    /// The length of the DynVec
    pub len: usize,
}

/// Error returned when trying to call [`DynVec::push_default`] but the type
/// doesn't implement Default
#[derive(thiserror::Error, Debug)]
#[error("The type in this DynVec doesn't implement Default")]
pub struct NoDefaultConstructorError;

/// Error returned for [`DynVec::set`] and [`OwnedDynVecValue::set_into`]
#[derive(thiserror::Error, Debug)]
pub enum InsertionError {
    /// See [`IncorrectTypeError`]
    #[error(transparent)]
    IncorrectType(#[from] IncorrectTypeError),
    /// See [`IndexOutOfBoundError`]
    #[error(transparent)]
    IndexOutOfBound(#[from] IndexOutOfBoundError),
}

/// Error returned for [`DynVec::set_default`]
#[derive(thiserror::Error, Debug)]
pub enum DefaultInsertionError {
    /// See [`NoDefaultConstructorError`]
    #[error(transparent)]
    NoDefaultConstructor(#[from] NoDefaultConstructorError),
    /// See [`IndexOutOfBoundError`]
    #[error(transparent)]
    IndexOutOfBound(#[from] IndexOutOfBoundError),
}

#[inline]
fn dangling_with_layout(layout: Layout) -> NonNull<u8> {
    // layout.align() is guaranteed > 0 and a power of two.
    unsafe { NonNull::without_provenance(Alignment::new_unchecked(layout.align()).as_nonzero()) }
}

/// Metadata describing the element type stored in a `DynVec`.
///
/// Construct using [`DynVecMetadata::new`] and pass it to APIs like
/// [`DynVec::new_with_meta`] or [`DynVec::with_capacity`].
///
/// All fields are public for transparency and potential interop, but the private
/// marker field prevents external construction to keep values consistent.
#[derive(Clone)]
pub struct DynVecMetadata {
    /// The `TypeId` of the element type.
    pub type_id: TypeId,
    /// The type_name of the element type.
    pub type_name: &'static str,
    /// The vtable metadata for `dyn Any` corresponding to the element type.
    pub dyn_meta: std::ptr::DynMetadata<dyn Any>,
    /// Initialize the default for the type into the given pointer
    pub default_fn: Option<unsafe fn(*mut u8)>,
    // Private dummy field to prevent external construction while keeping field reads possible.
    _priv: (),
}

impl DynVecMetadata {
    /// Creates metadata for the element type `T`.
    pub fn new<T: 'static>() -> Self {
        // Casting a (null) data pointer of T to a trait object pointer produces
        // a fat pointer with valid metadata for `T: Any`. The data ptr can be null
        // since we only query metadata and never dereference the fat pointer.
        let obj: *const dyn Any = std::ptr::null::<T>() as *const T as *const dyn Any;
        let meta = std::ptr::metadata(obj);

        unsafe fn default_fn<T: 'static>(into: *mut u8) {
            let maybe_default = <T as MaybeDefault>::maybe_default()
                .expect("Should exist");
            unsafe { (into as *mut T).write(maybe_default()) };
        }

        Self {
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
            dyn_meta: meta,
            default_fn: <T as MaybeDefault>::maybe_default()
                .and(Some(default_fn::<T>)),
            _priv: ()
        }
    }

    /// Returns the Layout from the pointer metadata
    pub fn layout(&self) -> Layout {
        self.dyn_meta.layout()
    }
}

impl std::fmt::Debug for DynVecMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let alternate = f.alternate();

        let mut ds = f.debug_struct("DynVecMetadata");
        ds.field("type_name", &self.type_name)
            .field("type_id", &self.type_id)
            .field("layout", &self.layout());

        if alternate {
            ds.field("default_fn", &self.default_fn);
        }
        else {
            ds.field("has_default_fn", &self.default_fn.is_some());
        }

        ds.finish()
    }
}

/// Type-erased vector storing a single runtime-selected element type.
///
/// See the crate-level docs for overview and safety guarantees.
pub struct DynVec {
    ptr: NonNull<u8>,
    len: usize,
    capacity: usize, // in elements
    /// Element type metadata (type id, layout, and vtable for `dyn Any`).
    meta: DynVecMetadata,
}

impl DynVec {
    /// Creates an empty `DynVec` for elements of the given runtime type.
    ///
    /// The vector starts empty and does not allocate until elements are pushed or
    /// capacity is reserved.
    ///
    /// Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut v = DynVec::new::<i32>();
    /// v.push(Box::new(1_i32)).unwrap();
    /// assert_eq!(v.len(), 1);
    /// ```
    pub fn new<T: 'static>() -> Self {
        Self::new_with_meta(DynVecMetadata::new::<T>())
    }

    /// Creates an empty `DynVec` using explicit metadata.
    ///
    /// Prefer `DynVec::new::<T>()` unless you need to pass metadata around.
    pub fn new_with_meta(meta: DynVecMetadata) -> Self {
        Self {
            // keep base pointer aligned to element type even with capacity == 0
            ptr: dangling_with_layout(meta.dyn_meta.layout()),
            len: 0,
            capacity: 0,
            meta,
        }
    }

    /// Creates a new `DynVec` with the given element type and capacity (in elements).
    ///
    /// Allocates enough space to hold at least `capacity` elements of the runtime type.
    pub fn with_capacity(meta: DynVecMetadata, capacity: usize) -> Self {
        let mut v = Self::new_with_meta(meta);
        if capacity > 0 {
            v.reserve(capacity);
        }
        v
    }

    #[inline]
    unsafe fn idx_ptr(&self, idx: usize) -> *mut u8 {
        debug_assert!(idx < self.len || idx == self.len && self.len <= self.capacity);
        unsafe { self.ptr.as_ptr().add(idx * self.meta.dyn_meta.layout().size()) }
    }

    #[inline]
    unsafe fn write_t<T: 'static>(&mut self, idx: usize, val: T) {
        debug_assert!(TypeId::of::<T>() == self.meta.type_id);
        debug_assert!(idx <= self.len && self.len <= self.capacity);
        unsafe { (self.idx_ptr(idx) as *mut T).write(val) };
    }

    #[inline]
    unsafe fn read_t<T: 'static>(&mut self, idx: usize) -> T {
        debug_assert!(TypeId::of::<T>() == self.meta.type_id);
        debug_assert!(idx < self.len);
        unsafe { (self.idx_ptr(idx) as *mut T).read() }
    }

    #[inline]
    unsafe fn copy_t<T: 'static>(&mut self, from: usize, to: usize, count: usize) {
        debug_assert!(TypeId::of::<T>() == self.meta.type_id);
        debug_assert!(from < self.len && to <= self.len && count <= self.len);
        if std::mem::size_of::<T>() == 0 || count == 0 || from == to {
            return;
        }
        unsafe {
            (self.idx_ptr(from) as *mut T)
                .copy_to_nonoverlapping(self.idx_ptr(to) as *mut T, count);
        }
    }

    #[inline]
    fn assert_type(&self, any: &dyn Any) -> Result<(), IncorrectTypeError> {
        let received_typeid = any.type_id();
        if received_typeid == self.meta.type_id {
            Ok(())
        }
        else {
            Err(IncorrectTypeError {
                expected_name: Some(self.meta.type_name),
                expected_typeid: self.meta.type_id,
                received_name: None,
                received_typeid,
            })
        }
    }

    #[inline]
    fn assert_type_t<T: 'static>(&self) -> Result<(), IncorrectTypeError> {
        let received_typeid = TypeId::of::<T>();
        if received_typeid == self.meta.type_id {
            Ok(())
        }
        else {
            Err(IncorrectTypeError {
                expected_name: Some(self.meta.type_name),
                expected_typeid: self.meta.type_id,
                received_name: Some(type_name::<T>()),
                received_typeid,
            })
        }
    }

    #[inline]
    fn assert_type_meta(&self, meta: &DynVecMetadata) -> Result<(), IncorrectTypeError> {
        if meta.type_id == self.meta.type_id {
            Ok(())
        }
        else {
            Err(IncorrectTypeError {
                expected_name: Some(self.meta.type_name),
                expected_typeid: self.meta.type_id,
                received_name: Some(meta.type_name),
                received_typeid: meta.type_id,
            })
        }
    }

    #[inline]
    fn assert_index(&self, idx: usize) -> Result<(), IndexOutOfBoundError> {
        if idx < self.len() {
            Ok(())
        }
        else {
            Err(IndexOutOfBoundError {
                index: idx,
                len: self.len(),
            })
        }
    }

    #[inline]
    unsafe fn drop_at(&mut self, idx: usize) {
        debug_assert!(idx < self.len);
        let meta = self.meta.dyn_meta;
        let data_ptr = unsafe { self.idx_ptr(idx) as *mut () };
        let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr, meta);
        unsafe { ptr::drop_in_place(fat) };
    }

    #[inline]
    unsafe fn as_dyn_ref_at<'a>(&self, idx: usize) -> &'a dyn Any {
        let meta = self.meta.dyn_meta;
        let data_ptr = unsafe { self.idx_ptr(idx) as *const () };
        let fat: *const dyn Any = from_raw_parts::<dyn Any>(data_ptr, meta);
        unsafe { &*fat }
    }

    #[inline]
    unsafe fn as_dyn_mut_at<'a>(&mut self, idx: usize) -> &'a mut dyn Any {
        let meta = self.meta.dyn_meta;
        let data_ptr = unsafe { self.idx_ptr(idx) as *mut () };
        let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr, meta);
        unsafe { &mut *fat }
    }

    /// Returns the number of elements currently stored.
    pub fn len(&self) -> usize { self.len }
    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool { self.len == 0 }
    /// Returns the number of elements the vector can hold without reallocating.
    pub fn capacity(&self) -> usize { self.capacity }

    /// Returns the element type metadata used by this vector.
    pub fn metadata(&self) -> &DynVecMetadata { &self.meta }

    /// Reserves capacity for at least `additional` more elements to be inserted.
    ///
    /// May reallocate. Uses a geometric growth strategy similar to `Vec`.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len.saturating_add(additional);
        if needed <= self.capacity { return; }
        // Choose next power of two for growth; minimum capacity of 4.
        let new_cap = needed.next_power_of_two().max(4);
        self.realloc_capacity(new_cap);
    }

    /// Shrinks the capacity as much as possible.
    ///
    /// If `len == 0`, frees the allocation. Otherwise, shrinks to exactly `len`.
    pub fn shrink_to_fit(&mut self) {
        if self.capacity == self.len { return; }
        self.realloc_capacity(self.len);
    }

    #[inline]
    fn realloc_capacity(&mut self, new_cap: usize) {
        debug_assert!(new_cap >= self.len, "new capacity cannot be less than len");
        let elem_size = self.meta.dyn_meta.layout().size();
        let align = self.meta.dyn_meta.layout().align();
        // ZST: no allocation required; just bump the logical capacity and keep aligned base.
        if elem_size == 0 {
            self.capacity = new_cap;
            self.ptr = dangling_with_layout(self.meta.dyn_meta.layout());
            return;
        }
        unsafe {
            if self.capacity == new_cap { return; }
            if self.capacity == 0 {
                if new_cap == 0 {
                    // Keep aligned base pointer for empty allocation
                    self.ptr = dangling_with_layout(self.meta.dyn_meta.layout());
                    self.capacity = 0;
                } else {
                    let size_bytes = new_cap.checked_mul(elem_size).expect("capacity overflow");
                    let layout = Layout::from_size_align(size_bytes, align).expect("invalid layout");
                    let new_ptr = alloc(layout);
                    if new_ptr.is_null() { std::alloc::handle_alloc_error(layout); }
                    self.ptr = NonNull::new_unchecked(new_ptr);
                    self.capacity = new_cap;
                }
            } else {
                let old_size = self.capacity.checked_mul(elem_size).expect("capacity overflow");
                let old_layout = Layout::from_size_align(old_size, align).expect("invalid layout");
                if new_cap == 0 {
                    dealloc(self.ptr.as_ptr(), old_layout);
                    self.ptr = dangling_with_layout(self.meta.dyn_meta.layout());
                    self.capacity = 0;
                } else {
                    let new_size = new_cap.checked_mul(elem_size).expect("capacity overflow");
                    let new_ptr = realloc(self.ptr.as_ptr(), old_layout, new_size);
                    if new_ptr.is_null() {
                        std::alloc::handle_alloc_error(
                            Layout::from_size_align(new_size, align).expect("invalid layout"),
                        );
                    }
                    self.ptr = NonNull::new_unchecked(new_ptr);
                    self.capacity = new_cap;
                }
            }
        }
    }

    /// Clears the vector, dropping all elements. Keeps capacity.
    pub fn clear(&mut self) {
        unsafe { for i in 0..self.len { self.drop_at(i); } }
        self.len = 0;
    }

    /// Creates a typed shared view for element type `T`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if `T` does not match the
    /// vector's element type.
    pub fn typed<T: 'static>(&self) -> Result<TypedDynVecRef<'_, T>, IncorrectTypeError> {
        TypedDynVecRef::<T>::new(self)
    }

    /// Creates a typed mutable view for element type `T`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if `T` does not match the
    /// vector's element type.
    pub fn typed_mut<T: 'static>(&mut self) -> Result<TypedDynVecRefMut<'_, T>, IncorrectTypeError> {
        TypedDynVecRefMut::<T>::new(self)
    }

    /// Returns a shared reference to the element at `idx` using a wrapper
    /// that delays chosing how to read the typed value later.
    /// See [`DynVecValueRef`].
    ///
    /// Return Err(IndexOutOfBoundError) if `idx` is out of bounds.
    pub fn get(&self, idx: usize) -> Result<DynVecValueRef<'_>, IndexOutOfBoundError> {
        self.assert_index(idx)?;
        Ok(DynVecValueRef {
            vec: self,
            idx,
        })
    }

    /// Returns a mutable reference to the element at `idx` using a wrapper
    /// that delays chosing how to read the typed value later.
    /// See [`DynVecValueRefMut`].
    ///
    /// Return Err(IndexOutOfBoundError) if `idx` is out of bounds.
    pub fn get_mut(&mut self, idx: usize) -> Result<DynVecValueRefMut<'_>, IndexOutOfBoundError> {
        self.assert_index(idx)?;
        Ok(DynVecValueRefMut {
            vec: self,
            idx,
        })
    }

    /// Appends an element to the back as `Box<dyn Any>`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if the boxed value's `TypeId`
    /// does not match the vector's element type.
    pub fn push(&mut self, val: Box<dyn Any>) -> Result<(), IncorrectTypeError> {
        self.assert_type(val.as_ref())?;

        if self.meta.dyn_meta.layout().size() == 0 {
            // ZST: no bytes to move; forget the box to defer drop to vector's lifecycle.
            core::mem::forget(val);
            self.reserve(1);
            self.len += 1;
            return Ok(());
        }

        let data_ptr = Box::into_raw(val) as *mut u8;
        self.reserve(1);
        // Safety: destination is within allocation; `data_ptr` points to a valid T value.
        unsafe { ptr::copy_nonoverlapping(data_ptr, self.idx_ptr(self.len), self.meta.dyn_meta.layout().size()) };
        unsafe { dealloc(data_ptr, self.meta.dyn_meta.layout()) };
        self.len += 1;

        Ok(())
    }

    /// If the dynvec's type implements [`Default`] this pushes a new value
    /// using its default method and returns Ok(()).
    /// 
    /// If it doesn't implement [`Default`] this return Err(NoDefaultConstructorError).
    pub fn push_default(&mut self) -> Result<(), NoDefaultConstructorError> {
        let Some(write_default) = self.meta.default_fn
        else { return Err(NoDefaultConstructorError) };

        self.reserve(1);
        unsafe { write_default(self.idx_ptr(self.len)) };
        self.len += 1;

        Ok(())
    }

    /// Pops the last element, if any, returning an owning guard over that value.
    ///
    /// The element is actually removed from the vector when the returned guard is dropped.
    pub fn pop(&mut self) -> Option<OwnedDynVecValue<'_>> {
        if self.len == 0 { return None; }
        Some(unsafe { self.swap_remove(self.len - 1).unwrap_unchecked() })
    }

    /// Replaces the element at `idx`, dropping the previous value in place.
    ///
    /// Returns an error if `idx` is out of bounds or the boxed value's `TypeId` mismatches.
    pub fn set(&mut self, idx: usize, val: Box<dyn Any>) -> Result<(), InsertionError> {
        self.assert_index(idx)?;
        self.assert_type(val.as_ref())?;

        if self.meta.dyn_meta.layout().size() == 0 {
            // ZST: drop the previous value's drop glue now; forget the new one to drop later.
            unsafe { self.drop_at(idx) };
            core::mem::forget(val);
            return Ok(());
        }

        let data_ptr = Box::into_raw(val) as *mut u8;
        unsafe { self.drop_at(idx) };
        unsafe { ptr::copy_nonoverlapping(data_ptr, self.idx_ptr(idx), self.meta.dyn_meta.layout().size()) };
        unsafe { dealloc(data_ptr, self.meta.dyn_meta.layout()) };

        Ok(())
    }
    
    /// Replaces the element at `idx`, dropping the previous value in place.
    ///
    /// Returns an error if `idx` is out of bounds or the boxed value's `TypeId` mismatches.
    pub fn set_default(&mut self, idx: usize) -> Result<(), DefaultInsertionError> {
        self.assert_index(idx)?;

        let Some(write_default) = self.meta.default_fn
        else { return Err(NoDefaultConstructorError.into()) };

        unsafe {
            self.drop_at(idx);
            write_default(self.idx_ptr(idx));
        }

        Ok(())
    }

    /// Removes and returns an owning guard for the element at `idx`.
    ///
    /// The element is logically owned by the returned guard. The backing vector is actually
    /// updated (swap in the last element and decrement `len`) when the guard is dropped.
    /// Returns `Err(IndexOutOfBoundError)` if out of bounds.
    ///
    /// # Examples
    ///
    /// Move a value into another `DynVec` with no allocation:
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut a = DynVec::new::<i32>();
    /// let mut b = DynVec::new::<i32>();
    /// a.typed_mut::<i32>().unwrap().extend([1, 2, 3]);
    /// a.swap_remove(1).unwrap().push_into(&mut b).unwrap();
    /// assert_eq!(a.typed::<i32>().unwrap().as_slice(), &[1, 3]);
    /// assert_eq!(b.typed::<i32>().unwrap().as_slice(), &[2]);
    /// ```
    ///
    /// Extract the removed value by type:
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut v = DynVec::new::<u64>();
    /// v.typed_mut::<u64>().unwrap().extend([10, 20]);
    /// let x: u64 = v.swap_remove(0).unwrap().into_typed().unwrap();
    /// assert_eq!(x, 10);
    /// assert_eq!(v.typed::<u64>().unwrap().as_slice(), &[20]);
    /// ```
    pub fn swap_remove(&mut self, idx: usize) -> Result<OwnedDynVecValue<'_>, IndexOutOfBoundError> {
        self.assert_index(idx)?;
        let last = self.len - 1;
        Ok(OwnedDynVecValue { vec: self, idx, last, consumed: false })
    }
}

impl Drop for DynVec {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.len {
                self.drop_at(i);
            }
            if self.capacity > 0 && self.meta.dyn_meta.layout().size() > 0 {
                let total_size = self.capacity.checked_mul(self.meta.dyn_meta.layout().size()).expect("capacity overflow");
                let layout = Layout::from_size_align(total_size, self.meta.dyn_meta.layout().align())
                    .expect("invalid layout");
                dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }
}

/// Shared reference guard for an element inside a `DynVec`.
///
/// The value can then be then be read as an Any trait object or casted into
/// its real type.
pub struct DynVecValueRef<'a> {
    vec: &'a DynVec,
    idx: usize,
}

impl<'a> DynVecValueRef<'a> {
    /// Returns a trait object reference to the underlying value.
    pub fn as_any(&self) -> &'a dyn Any {
        // SAFETY: Index checked before creating the struct
        unsafe { self.vec.as_dyn_ref_at(self.idx) }
    }

    /// If the given type is the same as the value's type a reference to the
    /// value is returned.
    /// Otherwise Err(IncorrectTypeError) is returned.
    pub fn as_typed<T: 'static>(&self) -> Result<&'a T, IncorrectTypeError> {
        self.vec.assert_type_t::<T>()?;
        Ok(unsafe { (self.vec.idx_ptr(self.idx) as *mut T).as_mut_unchecked() })
    }
}

/// Mutable reference guard for an element inside a `DynVec`.
///
/// The value can then be then be read as an Any trait object or casted into
/// its real type.
pub struct DynVecValueRefMut<'a> {
    vec: &'a mut DynVec,
    idx: usize,
}

impl<'a> DynVecValueRefMut<'a> {
    /// Returns a trait object mutable reference to the underlying value.
    pub fn as_any(&mut self) -> &'a mut dyn Any {
        unsafe { self.vec.as_dyn_mut_at(self.idx) }
    }

    /// If the given type is the same as the value's type a mutable reference
    /// to the value is returned.
    /// Otherwise Err(IncorrectTypeError) is returned.
    pub fn as_typed<T: 'static>(&mut self) -> Result<&'a mut T, IncorrectTypeError> {
        self.vec.assert_type_t::<T>()?;
        Ok(unsafe { (self.vec.idx_ptr(self.idx) as *mut T).as_mut_unchecked() })
    }
}

/// Owning guard for an element removed from a `DynVec`.
///
/// The value can be consumed via typed extraction or moved into another `DynVec`.
/// The source vector is actually updated on `Drop` of this guard.
///
/// Typical ways to consume the guard:
/// - Move into another `DynVec` of the same type using [`OwnedDynVecValue::push_into`]
/// - Overwrite a position in another `DynVec` using [`OwnedDynVecValue::set_into`]
/// - Extract the concrete value with [`OwnedDynVecValue::into_typed`]
/// - Box as `dyn Any` with [`OwnedDynVecValue::into_boxed_any`]
pub struct OwnedDynVecValue<'a> {
    vec: &'a mut DynVec,
    idx: usize,
    last: usize,
    consumed: bool,
}

impl<'a> OwnedDynVecValue<'a> {
    /// Consumes the guard and returns the value boxed as `dyn Any`.
    ///
    /// Allocates a new box and copies the bytes (ZST is handled without copying).
    ///
    /// # Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut v = DynVec::new::<String>();
    /// v.typed_mut::<String>().unwrap().extend(["a".to_string(), "b".to_string()]);
    /// let any = v.swap_remove(0).unwrap().into_boxed_any();
    /// assert!(any.downcast::<String>().is_ok());
    /// assert_eq!(v.typed::<String>().unwrap().as_slice(), &["b".to_string()]);
    /// ```
    pub fn into_boxed_any(mut self) -> Box<dyn Any> {
        let size = self.vec.meta.dyn_meta.layout().size();
        if size == 0 {
            // Fabricate a Box<dyn Any> for ZST using a proper fat pointer.
            let data_ptr = dangling_with_layout(self.vec.meta.dyn_meta.layout()).as_ptr();
            let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr as *mut (), self.vec.meta.dyn_meta);
            self.consumed = true;
            let boxed: Box<dyn Any> = unsafe { Box::from_raw(fat) };
            // Drop will handle len adjustment.
            boxed
        } else {
            unsafe {
                let data_ptr = alloc(self.vec.meta.dyn_meta.layout());
                if data_ptr.is_null() { std::alloc::handle_alloc_error(self.vec.meta.dyn_meta.layout()); }
                let src = self.vec.idx_ptr(self.idx);
                ptr::copy_nonoverlapping(src, data_ptr, size);
                let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr as *mut (), self.vec.meta.dyn_meta);
                self.consumed = true;
                Box::from_raw(fat)
            }
        }
    }

    /// Consumes the guard and returns the value as `T`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if `T` does not match the vector's element type.
    ///
    /// # Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut v = DynVec::new::<i32>();
    /// v.typed_mut::<i32>().unwrap().extend([1, 2]);
    /// let val: i32 = v.swap_remove(1).unwrap().into_typed().unwrap();
    /// assert_eq!(val, 2);
    /// assert_eq!(v.typed::<i32>().unwrap().as_slice(), &[1]);
    /// ```
    pub fn into_typed<T: 'static>(mut self) -> Result<T, IncorrectTypeError> {
        self.vec.assert_type_t::<T>()?;
        let out = unsafe { self.vec.read_t::<T>(self.idx) };
        self.consumed = true;
        Ok(out)
    }

    /// Moves the value into another `DynVec` with the same element type.
    ///
    /// Returns an `Err(IncorrectTypeError)` if the destination's element type differs.
    ///
    /// # Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut a = DynVec::new::<u32>();
    /// let mut b = DynVec::new::<u32>();
    /// a.typed_mut::<u32>().unwrap().extend([1, 2, 3]);
    /// a.swap_remove(0).unwrap().push_into(&mut b).unwrap();
    /// assert_eq!(a.typed::<u32>().unwrap().as_slice(), &[3, 2]);
    /// assert_eq!(b.typed::<u32>().unwrap().as_slice(), &[1]);
    /// ```
    pub fn push_into(mut self, dst: &mut DynVec) -> Result<(), IncorrectTypeError> {
        dst.assert_type_meta(&self.vec.meta)?;

        let size = self.vec.meta.dyn_meta.layout().size();
        if size == 0 {
            dst.reserve(1);
            dst.len += 1;
            self.consumed = true;
        } else {
            dst.reserve(1);
            unsafe {
                ptr::copy_nonoverlapping(self.vec.idx_ptr(self.idx), dst.idx_ptr(dst.len), size);
            }
            dst.len += 1;
            self.consumed = true;
        }

        Ok(())
    }

    /// Overwrites the element at `idx` in another `DynVec` with this value.
    ///
    /// Same semantics as [`DynVec::set`]: drops the previous value in `dst` at `idx`
    /// and writes the new one in-place. Length is unchanged. Returns an error if
    /// `idx` is out of bounds or `dst` has a different element type.
    ///
    /// # Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut a = DynVec::new::<i32>();
    /// let mut b = DynVec::new::<i32>();
    /// a.typed_mut::<i32>().unwrap().extend([10, 20, 30]);
    /// b.typed_mut::<i32>().unwrap().extend([1, 2, 3]);
    /// // Move 20 from `a` and overwrite index 1 in `b`
    /// a.swap_remove(1).unwrap().set_into(&mut b, 1).unwrap();
    /// assert_eq!(a.typed::<i32>().unwrap().as_slice(), &[10, 30]);
    /// assert_eq!(b.typed::<i32>().unwrap().as_slice(), &[1, 20, 3]);
    /// ```
    pub fn set_into(mut self, dst: &mut DynVec, idx: usize) -> Result<(), InsertionError> {
        dst.assert_type_meta(&self.vec.meta)?;
        dst.assert_index(idx)?;

        let size = self.vec.meta.dyn_meta.layout().size();

        if size == 0 {
            // ZST: drop previous value's drop glue; nothing to copy.
            unsafe { dst.drop_at(idx) };
            self.consumed = true;
            return Ok(());
        }

        unsafe {
            // Drop the previous value at destination index
            dst.drop_at(idx);
            // Copy bytes from source element into destination slot
            ptr::copy_nonoverlapping(self.vec.idx_ptr(self.idx), dst.idx_ptr(idx), size);
        }
        self.consumed = true;
        Ok(())
    }
}

impl<'a> Drop for OwnedDynVecValue<'a> {
    fn drop(&mut self) {
        // Finalize removal from the source vector.
        // We must drop the value at idx if not consumed, then swap in last and decrement len.
        let size = self.vec.meta.dyn_meta.layout().size();
        unsafe {
            if !self.consumed {
                // Drop the value in place using dyn Any vtable
                self.vec.drop_at(self.idx);
            }
            if self.idx != self.last {
                if size != 0 {
                    let last_ptr = self.vec.idx_ptr(self.last);
                    let dst = self.vec.idx_ptr(self.idx);
                    ptr::copy_nonoverlapping(last_ptr, dst, size);
                }
            }
            // Adjust length
            self.vec.len -= 1;
        }
    }
}

/// A typed shared reference view over a `DynVec`.
///
/// Constructing this view validates the type once; subsequent operations
/// do not re-check the type each call.
pub struct TypedDynVecRef<'a, T: 'static> {
    vec: &'a DynVec,
    _marker: std::marker::PhantomData<&'a T>,
}

impl<'a, T: 'static> TypedDynVecRef<'a, T> {
    /// Creates a typed view, panicking if the vector's element type mismatches `T`.
    pub fn new(vec: &'a DynVec) -> Result<Self, IncorrectTypeError> {
        vec.assert_type_t::<T>()?;
        Ok(Self { vec, _marker: std::marker::PhantomData })
    }

    /// Returns the length of the underlying vector.
    pub fn len(&self) -> usize { self.vec.len }
    /// Returns `true` if empty.
    pub fn is_empty(&self) -> bool { self.vec.len == 0 }

    /// Returns a shared slice over all elements.
    pub fn as_slice(&self) -> &'a [T] {
        let len = self.vec.len;
        let ptr = self.vec.ptr.as_ptr() as *const T;
        unsafe { slice::from_raw_parts(ptr, len) }
    }
}

impl<'a, T: 'static> Deref for TypedDynVecRef<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target { self.as_slice() }
}

/// A typed mutable reference view over a `DynVec`.
///
/// Constructing this view validates the type once; subsequent operations
/// do not re-check the type each call.
pub struct TypedDynVecRefMut<'a, T: 'static> {
    vec: &'a mut DynVec,
    _marker: std::marker::PhantomData<&'a mut T>,
}

impl<'a, T: 'static> TypedDynVecRefMut<'a, T> {
    /// Creates a typed mutable view, panicking if the vector's element type mismatches `T`.
    pub fn new(vec: &'a mut DynVec) -> Result<Self, IncorrectTypeError> {
        vec.assert_type_t::<T>()?;
        Ok(Self { vec, _marker: std::marker::PhantomData })
    }

    /// Returns the length of the underlying vector.
    pub fn len(&self) -> usize { self.vec.len }
    /// Returns `true` if empty.
    pub fn is_empty(&self) -> bool { self.vec.len == 0 }
    /// Clears all elements from the underlying vector.
    pub fn clear(&mut self) { self.vec.clear() }
    /// Pushes a value without boxing or virtual dispatch.
    pub fn push(&mut self, val: T) {
        self.vec.reserve(1);
        unsafe { self.vec.write_t::<T>(self.vec.len, val); }
        self.vec.len += 1;
    }
    /// Sets index to value, dropping the old, without boxing.
    ///
    /// Returns `Err(IndexOutOfBoundError)` if `idx >= len`.
    pub fn set(&mut self, idx: usize, val: T) -> Result<(), IndexOutOfBoundError> {
        self.vec.assert_index(idx)?;
        unsafe { self.vec.drop_at(idx); }
        unsafe { self.vec.write_t::<T>(idx, val); }
        Ok(())
    }
    /// Removes at index and returns the removed value, swapping in the last.
    ///
    /// Returns `Err(IndexOutOfBoundError)` if `idx >= len`.
    pub fn swap_remove(&mut self, idx: usize) -> Result<T, IndexOutOfBoundError> {
        self.vec.assert_index(idx)?;
        let last_idx = self.vec.len - 1;
        let out = unsafe { self.vec.read_t::<T>(idx) };
        if idx != last_idx {
            unsafe { self.vec.copy_t::<T>(last_idx, idx, 1); }
        }
        self.vec.len -= 1;
        Ok(out)
    }
    /// Pops the last element, if any.
    pub fn pop(&mut self) -> Option<T> {
        if self.vec.len == 0 { return None; }
        // safe unwrap: index is in-bounds by construction
        Some(self.swap_remove(self.vec.len - 1).unwrap())
    }

    /// Returns a shared slice over all elements.
    pub fn as_slice(&self) -> &'a [T] {
        let len = self.vec.len;
        let ptr = self.vec.ptr.as_ptr() as *const T;
        unsafe { slice::from_raw_parts(ptr, len) }
    }
    /// Returns a mutable slice over all elements.
    pub fn as_mut_slice(&mut self) -> &'a mut [T] {
        let len = self.vec.len;
        let ptr = self.vec.ptr.as_ptr() as *mut T;
        unsafe { slice::from_raw_parts_mut(ptr, len) }
    }
}

impl<'a, T: 'static> Deref for TypedDynVecRefMut<'a, T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target { self.as_slice() }
}

impl<'a, T: 'static> DerefMut for TypedDynVecRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { self.as_mut_slice() }
}

impl Extend<Box<dyn Any>> for DynVec {
    fn extend<I: IntoIterator<Item = Box<dyn Any>>>(&mut self, iter: I) {
        let it = iter.into_iter();
        let (lower, _) = it.size_hint();
        if lower > 0 { self.reserve(lower); }
        for item in it { self.push(item).expect("Attempted to extend with an incorrect type"); }
    }
}

impl<'a, T: 'static> Extend<T> for TypedDynVecRefMut<'a, T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        let it = iter.into_iter();
        let (lower, _) = it.size_hint();
        if lower > 0 { self.vec.reserve(lower); }
        for item in it { self.push(item); }
    }
}

impl<'a, 'b, T> Extend<&'a T> for TypedDynVecRefMut<'b, T>
where
    T: 'static + Clone,
{
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        let it = iter.into_iter();
        let (lower, _) = it.size_hint();
        if lower > 0 { self.vec.reserve(lower); }
        for item in it { self.push(item.clone()); }
    }
}

impl<T: 'static> std::iter::FromIterator<T> for DynVec {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let it = iter.into_iter();
        let (lower, _) = it.size_hint();
        let mut v = DynVec::new::<T>();
        if lower > 0 { v.reserve(lower); }
        {
            let mut tv = v.typed_mut::<T>().expect("Type it was just created with");
            for item in it { tv.push(item); }
        }
        v
    }
}
