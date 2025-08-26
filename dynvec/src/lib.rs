#![feature(ptr_metadata, ptr_alignment_type)]
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
//!     guard (OwnedDynVecValue) that can be consumed (e.g., `into_typed`, `push_into`, `insert_into`).
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

/// Error returned for insertion where both an incorrect type and index out of bound
/// is possible.
#[derive(thiserror::Error, Debug)]
pub enum InsertionError {
    /// See [`IncorrectTypeError`]
    #[error(transparent)]
    IncorrectType(#[from] IncorrectTypeError),
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
#[derive(Debug, Clone)]
pub struct DynVecMetadata {
    /// The `TypeId` of the element type.
    pub type_id: TypeId,
    /// The type_name of the element type.
    pub type_name: &'static str,
    /// The `Layout` of the element type.
    pub layout: Layout,
    /// The vtable metadata for `dyn Any` corresponding to the element type.
    pub meta: std::ptr::DynMetadata<dyn Any>,
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

        Self {
            type_id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
            layout: Layout::new::<T>(),
            meta,
            _priv: ()
        }
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
            ptr: dangling_with_layout(meta.layout),
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

    // #[inline]
    // fn elem_size(&self) -> usize { self.meta.layout.size() }

    // #[inline]
    // fn elem_align(&self) -> usize { self.meta.layout.align() }

    #[inline]
    unsafe fn idx_ptr(&self, idx: usize) -> *mut u8 {
        debug_assert!(idx < self.len || idx == self.len && self.len <= self.capacity);
        unsafe { self.ptr.as_ptr().add(idx * self.meta.layout.size()) }
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
    fn assert_index_inclusive(&self, idx: usize) -> Result<(), IndexOutOfBoundError> {
        if idx <= self.len() {
            Ok(())
        } else {
            Err(IndexOutOfBoundError { index: idx, len: self.len() })
        }
    }

    #[inline]
    unsafe fn drop_at(&mut self, idx: usize) {
        debug_assert!(idx < self.len);
        let meta = self.meta.meta;
        let data_ptr = unsafe { self.idx_ptr(idx) as *mut () };
        let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr, meta);
        unsafe { ptr::drop_in_place(fat) };
    }

    #[inline]
    unsafe fn as_dyn_ref_at<'a>(&self, idx: usize) -> &'a dyn Any {
        let meta = self.meta.meta;
        let data_ptr = unsafe { self.idx_ptr(idx) as *const () };
        let fat: *const dyn Any = from_raw_parts::<dyn Any>(data_ptr, meta);
        unsafe { &*fat }
    }

    #[inline]
    unsafe fn as_dyn_mut_at<'a>(&mut self, idx: usize) -> &'a mut dyn Any {
        let meta = self.meta.meta;
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
        let elem_size = self.meta.layout.size();
        let align = self.meta.layout.align();
        // ZST: no allocation required; just bump the logical capacity and keep aligned base.
        if elem_size == 0 {
            self.capacity = new_cap;
            self.ptr = dangling_with_layout(self.meta.layout);
            return;
        }
        unsafe {
            if self.capacity == new_cap { return; }
            if self.capacity == 0 {
                if new_cap == 0 {
                    // Keep aligned base pointer for empty allocation
                    self.ptr = dangling_with_layout(self.meta.layout);
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
                    self.ptr = dangling_with_layout(self.meta.layout);
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

    /// Returns a shared reference to the element at `idx` as `&dyn Any`.
    ///
    /// Returns `None` if `idx` is out of bounds. Downcast using `Any::downcast_ref`.
    pub fn get(&self, idx: usize) -> Option<&dyn Any> {
        if idx >= self.len { return None; }
        unsafe { Some(self.as_dyn_ref_at(idx)) }
    }

    /// Returns a mutable reference to the element at `idx` as `&mut dyn Any`.
    ///
    /// Returns `None` if `idx` is out of bounds. Downcast using `Any::downcast_mut`.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut dyn Any> {
        if idx >= self.len { return None; }
        unsafe { Some(self.as_dyn_mut_at(idx)) }
    }

    /// Appends an element to the back as `Box<dyn Any>`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if the boxed value's `TypeId`
    /// does not match the vector's element type.
    pub fn push(&mut self, val: Box<dyn Any>) -> Result<(), IncorrectTypeError> {
        self.assert_type(val.as_ref())?;

        if self.meta.layout.size() == 0 {
            // ZST: no bytes to move; forget the box to defer drop to vector's lifecycle.
            core::mem::forget(val);
            self.reserve(1);
            self.len += 1;
            return Ok(());
        }

        let data_ptr = Box::into_raw(val) as *mut u8;
        self.reserve(1);
        // Safety: destination is within allocation; `data_ptr` points to a valid T value.
        unsafe { ptr::copy_nonoverlapping(data_ptr, self.idx_ptr(self.len), self.meta.layout.size()) };
        unsafe { dealloc(data_ptr, self.meta.layout) };
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

        if self.meta.layout.size() == 0 {
            // ZST: drop the previous value's drop glue now; forget the new one to drop later.
            unsafe { self.drop_at(idx) };
            core::mem::forget(val);
            return Ok(());
        }

        let data_ptr = Box::into_raw(val) as *mut u8;
        unsafe { self.drop_at(idx) };
        unsafe { ptr::copy_nonoverlapping(data_ptr, self.idx_ptr(idx), self.meta.layout.size()) };
        unsafe { dealloc(data_ptr, self.meta.layout) };

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
            if self.capacity > 0 && self.meta.layout.size() > 0 {
                let total_size = self.capacity.checked_mul(self.meta.layout.size()).expect("capacity overflow");
                let layout = Layout::from_size_align(total_size, self.meta.layout.align())
                    .expect("invalid layout");
                dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }
}

/// Owning guard for an element removed from a `DynVec`.
///
/// The value can be consumed via typed extraction or moved into another `DynVec`.
/// The source vector is actually updated on `Drop` of this guard.
///
/// Typical ways to consume the guard:
/// - Move into another `DynVec` of the same type using [`OwnedDynVecValue::push_into`]
/// - Insert at a position using [`OwnedDynVecValue::insert_into`]
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
        let size = self.vec.meta.layout.size();
        if size == 0 {
            // Fabricate a Box<dyn Any> for ZST using a proper fat pointer.
            let data_ptr = dangling_with_layout(self.vec.meta.layout).as_ptr();
            let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr as *mut (), self.vec.meta.meta);
            self.consumed = true;
            let boxed: Box<dyn Any> = unsafe { Box::from_raw(fat) };
            // Drop will handle len adjustment.
            boxed
        } else {
            unsafe {
                let data_ptr = alloc(self.vec.meta.layout);
                if data_ptr.is_null() { std::alloc::handle_alloc_error(self.vec.meta.layout); }
                let src = self.vec.idx_ptr(self.idx);
                ptr::copy_nonoverlapping(src, data_ptr, size);
                let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr as *mut (), self.vec.meta.meta);
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

        let size = self.vec.meta.layout.size();
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

    /// Inserts the value into another `DynVec` at position `at`, shifting elements to the right.
    ///
    /// Returns an error if `at > dst.len()` or the destination's element type differs.
    ///
    /// # Example
    /// ```
    /// use portal_dynvec::DynVec;
    /// let mut a = DynVec::new::<i32>();
    /// let mut b = DynVec::new::<i32>();
    /// a.typed_mut::<i32>().unwrap().extend([10, 20, 30]);
    /// b.typed_mut::<i32>().unwrap().extend([1, 2, 3]);
    /// a.swap_remove(1).unwrap().insert_into(&mut b, 1).unwrap();
    /// assert_eq!(a.typed::<i32>().unwrap().as_slice(), &[10, 30]);
    /// assert_eq!(b.typed::<i32>().unwrap().as_slice(), &[1, 20, 2, 3]);
    /// ```
    pub fn insert_into(mut self, dst: &mut DynVec, at: usize) -> Result<(), InsertionError> {
        dst.assert_type_meta(&self.vec.meta)?;
        dst.assert_index_inclusive(at)?;

        let size = self.vec.meta.layout.size();
        dst.reserve(1);

        if size == 0 {
            // ZST: no bytes to move, only grow logically
            // Shifting is a no-op for ZST.
            dst.len += 1;
            self.consumed = true;
            return Ok(());
        }

        unsafe {
            if at < dst.len {
                // Shift tail to make room: memmove [at..len) -> [at+1..len+1)
                let count = dst.len - at;
                let bytes = count * size;
                ptr::copy(
                    dst.idx_ptr(at),
                    dst.idx_ptr(at + 1),
                    bytes,
                );
            }
            // Copy the value bytes into the hole at `at`
            ptr::copy_nonoverlapping(self.vec.idx_ptr(self.idx), dst.idx_ptr(at), size);
            dst.len += 1;
        }
        self.consumed = true;

        Ok(())
    }
}

impl<'a> Drop for OwnedDynVecValue<'a> {
    fn drop(&mut self) {
        // Finalize removal from the source vector.
        // We must drop the value at idx if not consumed, then swap in last and decrement len.
        let size = self.vec.meta.layout.size();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn test_dyn_vec_i32() {
        let mut dyn_vec = DynVec::new::<i32>();
        assert_eq!(dyn_vec.metadata().type_id, std::any::TypeId::of::<i32>());

        let mut view = dyn_vec.typed_mut::<i32>().unwrap();

        view.push(10_i32);
        view.push(20_i32);
        view.push(30_i32);
        assert_eq!(view.len(), 3);

        let val = view.get(1).unwrap();
        assert_eq!(*val, 20);

        let mut_val = view.get_mut(1).unwrap();
        *mut_val = 25;
        let val_after_mut = view.get(1).unwrap();
        assert_eq!(*val_after_mut, 25);

        view.set(0, 5).unwrap();
        assert_eq!(*view.get(0).unwrap(), 5);
        assert_eq!(view.len(), 3);
        let removed = view.swap_remove(0).unwrap();
        assert_eq!(removed, 5);
        assert_eq!(view.len(), 2);
        assert_eq!(*view.get(0).unwrap(), 30);
        assert_eq!(*view.get(1).unwrap(), 25);
    }

    #[test]
    fn test_type_mismatch_push() {
        let mut dyn_vec = DynVec::new::<i32>();
        let res = dyn_vec.push(Box::new("hello".to_string()));
        assert!(res.is_err());
    }

    #[test]
    fn test_growth_and_bounds() {
        let mut v = DynVec::new::<u64>();
        {
            let mut tv = v.typed_mut::<u64>().unwrap();
            for i in 0..100 { tv.push(i as u64); }
            assert_eq!(tv.len(), 100);
            assert_eq!(*tv.get(50).unwrap(), 50);
        }
        let removed = {
            let mut tv = v.typed_mut::<u64>().unwrap();
            tv.swap_remove(10).unwrap()
        };
        assert_eq!(removed, 10);
        assert_eq!(v.len(), 99);
        let tv = v.typed::<u64>().unwrap();
        assert!(tv.get(999).is_none());

        assert!(!v.is_empty());
    }

    #[test]
    fn test_capacity_reserve_clear_pop_shrink() {
        let mut v = DynVec::with_capacity(DynVecMetadata::new::<i32>(), 0);
        assert!(v.is_empty());
        assert_eq!(v.capacity(), 0);

        v.reserve(10);
        assert!(v.capacity() >= 10);

        {
            let mut tv = v.typed_mut::<i32>().unwrap();
            tv.push(1_i32);
            tv.push(2_i32);
            tv.push(3_i32);
            assert_eq!(tv.len(), 3);
            assert_eq!(tv.pop().unwrap(), 3);
        }
        assert_eq!(v.len(), 2);
        v.clear();
        assert!(v.is_empty());
        let cap = v.capacity();
        {
            let mut tv = v.typed_mut::<i32>().unwrap();
            tv.push(4_i32);
        }
        assert_eq!(v.capacity(), cap);
        v.clear();
        v.shrink_to_fit();
        assert_eq!(v.capacity(), 0);
    }

    #[derive(Debug)]
    struct DropProbe { hits: Arc<AtomicUsize> }
    impl Drop for DropProbe { fn drop(&mut self) { self.hits.fetch_add(1, Ordering::SeqCst); } }

    #[test]
    fn test_drop_paths() {
        let hits = Arc::new(AtomicUsize::new(0));
        let mut v = DynVec::new::<DropProbe>();
        {
            let mut tv = v.typed_mut::<DropProbe>().unwrap();
            for _ in 0..5 { tv.push(DropProbe { hits: hits.clone() }); }
            tv.set(2, DropProbe { hits: hits.clone() }).unwrap();
            let _r: DropProbe = tv.swap_remove(1).unwrap();
            drop(_r);
            tv.clear();
        }
        assert_eq!(hits.load(Ordering::SeqCst), 6);
        drop(v);
        assert_eq!(hits.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn test_typed_views() {
        let mut v = DynVec::new::<i32>();
        {
            let mut tv = v.typed_mut::<i32>().unwrap();
            tv.push(1);
            tv.push(2);
            tv.push(3);
        }
        let view = v.typed::<i32>().unwrap();
        assert_eq!(view.len(), 3);
        assert_eq!(*view.get(1).unwrap(), 2);
        assert_eq!(view[0], 1);
        assert_eq!(view.iter().copied().sum::<i32>(), 1 + 2 + 3);
        let mut view_mut = v.typed_mut::<i32>().unwrap();
        assert_eq!(*view_mut.get_mut(2).unwrap(), 3);
        view_mut.set(1, 20).unwrap();
        view_mut.push(4);
        assert_eq!(view_mut.swap_remove(0).unwrap(), 1);
        assert_eq!(view_mut.pop().unwrap(), 3);
        for x in view_mut.iter_mut() { *x *= 2; }
        let view2 = v.typed::<i32>().unwrap();
        assert_eq!(view2.as_slice(), &[8, 40]);
    }

    #[test]
    fn test_zst_unit_typed() {
        let mut v = DynVec::new::<()>();
        {
            let mut tv = v.typed_mut::<()>().unwrap();
            for _ in 0..10 { tv.push(()); }
            assert_eq!(tv.len(), 10);
            // Indexing and iteration should work
            assert_eq!(tv.as_slice().len(), 10);
            // swap_remove keeps len consistent
            tv.swap_remove(3).unwrap();
            assert_eq!(tv.len(), 9);
            // pop returns Some(()) until empty
            assert!(tv.pop().is_some());
        }
        // After scope, v still valid
        assert!(v.len() <= 9);
    }

    #[test]
    fn test_zst_unit_untyped() {
        let mut v = DynVec::new::<()>();
        for _ in 0..5 { let _ = v.push(Box::new(())); }
        assert_eq!(v.len(), 5);
        // get returns &dyn Any; downcast_ref::<()>() works
        assert!(v.get(0).unwrap().is::<()>());
        // set with ZST keeps len
        let _ = v.set(2, Box::new(()));
        assert_eq!(v.len(), 5);
        // swap_remove returns OwnedDynVecValue; convert to Box<dyn Any>
        let b = v.swap_remove(1).unwrap().into_boxed_any();
        assert!(b.downcast::<()>().is_ok());
        assert_eq!(v.len(), 4);
        // pop to empty
        while v.pop().is_some() {}
        assert!(v.is_empty());
    }

    #[test]
    fn test_zst_custom() {
        #[derive(Copy, Clone, Debug)]
        struct Z;
        let mut v = DynVec::new::<Z>();
        {
            let mut tv = v.typed_mut::<Z>().unwrap();
            for _ in 0..16 { tv.push(Z); }
            assert_eq!(tv.len(), 16);
            // swap remove a few
            tv.swap_remove(0).unwrap();
            tv.swap_remove(5.min(tv.len()-1)).unwrap();
            assert!(tv.pop().is_some());
        }
        assert!(v.len() <= 14);
        // Untyped operations work too
        let _ = v.push(Box::new(Z));
        assert!(v.get(0).unwrap().is::<Z>());
    }

    #[test]
    fn test_owned_guard_push_into_basic() {
        let mut src = DynVec::new::<i32>();
        let mut dst = DynVec::new::<i32>();
        {
            let mut tv = src.typed_mut::<i32>().unwrap();
            tv.extend([1, 2, 3]);
        }
        // remove middle element and push into dst
        src.swap_remove(1).unwrap().push_into(&mut dst).unwrap();
        // After guard drop, src should have [1, 3] in some order (swap removal brings last into idx)
        let s = src.typed::<i32>().unwrap();
        assert_eq!(s.as_slice(), &[1, 3]);
        let d = dst.typed::<i32>().unwrap();
        assert_eq!(d.as_slice(), &[2]);
    }

    #[test]
    fn test_owned_guard_insert_into_positions() {
        let mut src = DynVec::new::<i32>();
        let mut dst = DynVec::new::<i32>();
        {
            let mut tv = src.typed_mut::<i32>().unwrap();
            tv.extend([10, 20, 30]);
        }
        {
            let mut dv = dst.typed_mut::<i32>().unwrap();
            dv.extend([100, 200]);
        }
        // Move first element (10) and insert into middle of dst
        src.swap_remove(0).unwrap().insert_into(&mut dst, 1).unwrap();
        let s = src.typed::<i32>().unwrap();
        assert_eq!(s.as_slice(), &[30, 20]);
        {
            let d = dst.typed::<i32>().unwrap();
            assert_eq!(d.as_slice(), &[100, 10, 200]);
        }

        // Insert at end
        let end = dst.len();
        src.swap_remove(0).unwrap().insert_into(&mut dst, end).unwrap();
        {
            let s2 = src.typed::<i32>().unwrap();
            assert_eq!(s2.as_slice(), &[20]);
        }
        {
            let d2 = dst.typed::<i32>().unwrap();
            assert_eq!(d2.as_slice(), &[100, 10, 200, 30]);
        }

        // Insert at beginning
        src.swap_remove(0).unwrap().insert_into(&mut dst, 0).unwrap();
        let d3 = dst.typed::<i32>().unwrap();
        assert_eq!(d3.as_slice(), &[20, 100, 10, 200, 30]);
    }

    #[test]
    fn test_push_into_type_mismatch_returns_err() {
        let mut a = DynVec::new::<i32>();
        let mut b = DynVec::new::<u64>();
        {
            let mut tv = a.typed_mut::<i32>().unwrap();
            tv.push(42);
        }
        let res = a.swap_remove(0).unwrap().push_into(&mut b);
        assert!(res.is_err());
    }

    #[test]
    fn test_into_typed_and_drop_semantics() {
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
        struct DropProbe { hits: Arc<AtomicUsize> }
        impl Drop for DropProbe { fn drop(&mut self) { self.hits.fetch_add(1, Ordering::SeqCst); } }

        // Consumed path (into_typed): element should NOT be dropped by guard, only by dropping returned value
        let hits = Arc::new(AtomicUsize::new(0));
        let mut v = DynVec::new::<DropProbe>();
        {
            let mut tv = v.typed_mut::<DropProbe>().unwrap();
            tv.push(DropProbe { hits: hits.clone() });
        }
        let p: DropProbe = v.swap_remove(0).unwrap().into_typed::<DropProbe>().unwrap();
        assert_eq!(hits.load(Ordering::SeqCst), 0);
        drop(p);
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        assert_eq!(v.len(), 0);

        // Not consumed path: dropping guard should drop the element
        let mut v2 = DynVec::new::<DropProbe>();
        {
            let mut tv = v2.typed_mut::<DropProbe>().unwrap();
            tv.push(DropProbe { hits: hits.clone() });
        }
        let _guard = v2.swap_remove(0).unwrap();
        drop(_guard);
        assert_eq!(hits.load(Ordering::SeqCst), 2);
        assert_eq!(v2.len(), 0);
    }

    #[test]
    fn test_zst_insert_and_push_into() {
        #[derive(Copy, Clone, Debug)]
        struct Z;
        let mut a = DynVec::new::<Z>();
        let mut b = DynVec::new::<Z>();
        {
            let mut tv = a.typed_mut::<Z>().unwrap();
            for _ in 0..3 { tv.push(Z); }
        }
        a.swap_remove(1).unwrap().push_into(&mut b).unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 1);
        a.swap_remove(0).unwrap().insert_into(&mut b, 0).unwrap();
        assert_eq!(a.len(), 1);
        assert_eq!(b.len(), 2);
        let end = b.len();
        a.swap_remove(0).unwrap().insert_into(&mut b, end).unwrap();
        assert_eq!(a.len(), 0);
        assert_eq!(b.len(), 3);
    }

    #[test]
    fn test_pop_returns_guard() {
        let mut v = DynVec::new::<i32>();
        v.typed_mut::<i32>().unwrap().extend([7, 8]);
        let g = v.pop().unwrap();
        // Move into another vec
        let mut dst = DynVec::new::<i32>();
        g.push_into(&mut dst).unwrap();
        // src len decremented on drop
        assert_eq!(v.len(), 1);
        assert_eq!(dst.typed::<i32>().unwrap().as_slice(), &[8]);
    }

    #[test]
    fn test_extend_typed_and_untyped() {
        // Typed view extend
        let mut v = DynVec::new::<i32>();
        {
            let mut tv = v.typed_mut::<i32>().unwrap();
            // trait method (in scope via prelude)
            tv.extend([1, 2, 3]);
            // trait method
            std::iter::Extend::extend(&mut tv, [4, 5]);
            // extend from references requires Clone
            let buf = vec![6_i32, 7_i32];
            tv.extend(buf.iter());
        }
        let tv = v.typed::<i32>().unwrap();
        assert_eq!(tv.as_slice(), &[1, 2, 3, 4, 5, 6, 7]);

        // Untyped extend via Box<dyn Any>
        let mut u = DynVec::new::<String>();
        let items = ["a", "bb", "ccc"].into_iter().map(|s| Box::new(s.to_string()) as Box<dyn Any>);
        u.extend(items);
        // trait method
        let items2 = ["dddd", "eeeee"].into_iter().map(|s| Box::new(s.to_string()) as Box<dyn Any>);
        std::iter::Extend::extend(&mut u, items2);
        let uv = u.typed::<String>().unwrap();
        assert_eq!(
            uv.as_slice(),
            &["a".to_string(), "bb".to_string(), "ccc".to_string(), "dddd".to_string(), "eeeee".to_string()]
        );
    }

    #[test]
    fn test_collect_into_dynvec() {
        let v: DynVec = [10_i64, 20, 30].into_iter().collect();
        assert_eq!(v.metadata().type_id, TypeId::of::<i64>());
        let t = v.typed::<i64>().unwrap();
        assert_eq!(t.as_slice(), &[10, 20, 30]);
    }
}
