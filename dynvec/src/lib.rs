#![feature(ptr_metadata, ptr_alignment_type)]
#![feature(ptr_as_ref_unchecked)]
#![feature(assert_matches)]
#![feature(type_alias_impl_trait)]
#![feature(alloc_layout_extra)]
#![feature(box_vec_non_null)]
#![feature(impl_trait_in_assoc_type)]

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
//!     guard (RemovedDynVecValue) that can be consumed (e.g., `into_typed`, `push_into`, `set_into`).
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

use std::alloc::Layout;
use std::any::{ type_name, Any, TypeId };
use std::cmp::min;
use std::marker::PhantomData;
use std::ops::{ Deref, DerefMut };
use std::ptr::{ self, from_raw_parts, from_raw_parts_mut, NonNull };
use std::slice;

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

/// Error returned for [`DynVec::set`] and [`RemovedDynVecValue::set_into`]
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

unsafe fn try_alloc(layout: Layout) -> NonNull<u8> {
    debug_assert!(layout.size() != 0);
    let allocated = unsafe { std::alloc::alloc(layout) };
    NonNull::new(allocated)
        .unwrap_or_else(|| std::alloc::handle_alloc_error(layout))
}

fn alloc_or_dangling(layout: Layout) -> NonNull<u8> {
    if layout.size() == 0 {
        layout.dangling()
    }
    else {
        unsafe { try_alloc(layout) }
    }
}

unsafe fn dealloc_or_dangling(ptr: NonNull<u8>, layout: Layout) {
    if layout.size() == 0 {
        // NOOP
    }
    else {
        unsafe { std::alloc::dealloc(ptr.as_ptr(), layout) }
    }
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
    pub default_fn: Option<unsafe fn(NonNull<()>)>,
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

        unsafe fn default_fn<T: 'static>(into: NonNull<()>) {
            debug_assert!(<T as MaybeDefault>::maybe_default().is_some(), "This function should only be called when T: Default");
            unsafe {
                let maybe_default = <T as MaybeDefault>::maybe_default()
                    .unwrap_unchecked();
                into.cast::<T>().write(maybe_default())
            };
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

    fn assert_type_t<T: 'static>(&self) -> Result<(), IncorrectTypeError> {
        let received_typeid = TypeId::of::<T>();
        if received_typeid == self.type_id {
            Ok(())
        }
        else {
            Err(IncorrectTypeError {
                expected_name: Some(self.type_name),
                expected_typeid: self.type_id,
                received_name: Some(type_name::<T>()),
                received_typeid,
            })
        }
    }

    unsafe fn drop_ptr(&self, data_ptr: NonNull<()>) {
        let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr.as_ptr(), self.dyn_meta);
        unsafe { ptr::drop_in_place(fat) };
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
    ptr: NonNull<()>,
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
            ptr: meta.dyn_meta.layout().dangling().cast(),
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

    /// # Safety
    /// The index must point inside the allocated memory
    unsafe fn idx_ptr(&self, idx: usize) -> NonNull<()> {
        // Idx can be == len when we are pushing into the dynvec
        debug_assert!(idx < self.len || (idx == self.len && self.len < self.capacity));
        unsafe { self.ptr.byte_add(idx * self.meta.dyn_meta.layout().size()) }
    }

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

    fn assert_type_t<T: 'static>(&self) -> Result<(), IncorrectTypeError> {
        self.meta.assert_type_t::<T>()
    }

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

    fn try_idx_ptr(&self, idx: usize) -> Result<NonNull<()>, IndexOutOfBoundError> {
        self.assert_index(idx)?;
        // SAFETY: Index just checked
        Ok(unsafe { self.idx_ptr(idx) })
    }

    unsafe fn drop_at(&mut self, idx: usize) {
        debug_assert!(idx < self.len);
        unsafe { self.meta.drop_ptr(self.idx_ptr(idx)) };
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

    /// This returns an uninitialized slot
    fn push_and_get_ptr(&mut self) -> NonNull<()> {
        self.reserve(1);
        // SAFETY: Just reserved additional capacity
        let ptr = unsafe { self.idx_ptr(self.len) };
        self.len += 1;
        ptr
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

        if new_cap == self.capacity { return }

        let (old_layout, _) = self.meta.dyn_meta.layout()
            .repeat(self.capacity)
            .expect("capacity overflow");
        let (new_layout, _) = self.meta.dyn_meta.layout()
            .repeat(new_cap)
            .expect("capacity overflow");
        debug_assert_eq!(old_layout.align(), new_layout.align());

        // NOTE: we don't use realloc, because i ditn't want to and it is not
        // necessarily faster (?)
        // Though it may very much may make memory fragmentation a big problem
        // so who knows.

        let new_ptr = alloc_or_dangling(new_layout);
        unsafe { new_ptr.copy_from_nonoverlapping(self.ptr.cast(), min(old_layout.size(), new_layout.size())); }
        unsafe { dealloc_or_dangling(self.ptr.cast(), old_layout) };

        self.ptr = new_ptr.cast();
        self.capacity = new_cap;
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
        Ok(DynVecValueRef {
            metadata: &self.meta,
            ptr: self.try_idx_ptr(idx)?,
            _data: PhantomData,
        })
    }

    /// Returns a mutable reference to the element at `idx` using a wrapper
    /// that delays chosing how to read the typed value later.
    /// See [`DynVecValueRefMut`].
    ///
    /// Return Err(IndexOutOfBoundError) if `idx` is out of bounds.
    pub fn get_mut(&mut self, idx: usize) -> Result<DynVecValueRefMut<'_>, IndexOutOfBoundError> {
        Ok(DynVecValueRefMut {
            metadata: &self.meta,
            ptr: self.try_idx_ptr(idx)?,
            _data: PhantomData,
        })
    }

    /// Returns an iterator over reference of all elements in this vec
    pub fn iter(&self) -> impl Iterator<Item = DynVecValueRef<'_>> + DoubleEndedIterator + ExactSizeIterator {
        (0..self.len).map(|idx| DynVecValueRef {
            metadata: &self.meta,
            ptr: unsafe { self.idx_ptr(idx) },
            _data: PhantomData,
        })
    }

    /// Returns an iterator over reference of all elements in this vec
    pub fn iter_mut(&mut self) -> impl Iterator<Item = DynVecValueRefMut<'_>> + DoubleEndedIterator + ExactSizeIterator {
        // (0..self.len).map(|idx| DynVecValueRefMut {
        //     vec: self,
        //     idx,
        // })
        todo!();
        std::iter::empty()
    }

    /// Removes all elements from the vec like [`DynVec::clear`], but
    /// also returns an iterator of these elements.
    ///
    /// Note that the unconsumed elements from the iterator are still dropped
    /// alongside the iterator.
    pub fn drain(&mut self) -> DynVecDrain<'_> {
        DynVecDrain {
            len: &mut self.len,
            ptr: self.ptr,
            metadata: &self.meta,
            current_idx: 0,
            _ref: PhantomData,
        }
    }

    /// Appends an element to the back as `Box<dyn Any>`.
    ///
    /// Returns an `Err(IncorrectTypeError)` if the boxed value's `TypeId`
    /// does not match the vector's element type.
    pub fn push(&mut self, val: Box<dyn Any>) -> Result<(), IncorrectTypeError> {
        self.assert_type(val.as_ref())?;
        let val = Box::into_non_null(val).cast::<u8>();

        let new_ptr = self.push_and_get_ptr();

        // SAFETY: destination is within allocation; `data_ptr` points to a valid T value.
        unsafe {
            new_ptr.cast::<u8>()
                .copy_from_nonoverlapping(val, self.meta.dyn_meta.layout().size());
        }
        // dealloc without running destructor
        // SAFETY: val comes from the Box
        unsafe { dealloc_or_dangling(val, self.meta.dyn_meta.layout()) };

        Ok(())
    }

    /// If the dynvec's type implements [`Default`] this pushes a new value
    /// using its default method and returns Ok(()).
    /// 
    /// If it doesn't implement [`Default`] this return Err(NoDefaultConstructorError).
    pub fn push_default(&mut self) -> Result<(), NoDefaultConstructorError> {
        let Some(write_default) = self.meta.default_fn
        else { return Err(NoDefaultConstructorError) };
        unsafe { write_default(self.push_and_get_ptr()) };
        Ok(())
    }

    /// Pops the last element, if any, returning an owning guard over that value.
    ///
    /// The element is actually removed from the vector when the returned guard is dropped.
    pub fn pop(&mut self) -> Option<RemovedDynVecValue<'_>> {
        self.swap_remove(self.len.checked_sub(1)?).ok()
    }

    /// Replaces the element at `idx`, dropping the previous value in place.
    ///
    /// Returns an error if `idx` is out of bounds or the boxed value's `TypeId` mismatches.
    pub fn set(&mut self, idx: usize, val: Box<dyn Any>) -> Result<(), InsertionError> {
        let ptr = self.try_idx_ptr(idx)?;
        self.assert_type(val.as_ref())?;
        let val = Box::into_non_null(val).cast::<u8>();

        // SAFETY: destination is within allocation; `data_ptr` points to a valid T value.
        unsafe {
            ptr.cast::<u8>()
                .copy_from_nonoverlapping(val, self.meta.dyn_meta.layout().size());
        }
        // dealloc without running destructor
        // SAFETY: val comes from the Box
        unsafe { dealloc_or_dangling(val, self.meta.dyn_meta.layout()) };

        Ok(())
    }
    
    /// Replaces the element at `idx`, dropping the previous value in place.
    ///
    /// Returns an error if `idx` is out of bounds or the boxed value's `TypeId` mismatches.
    pub fn set_default(&mut self, idx: usize) -> Result<(), DefaultInsertionError> {
        let Some(write_default) = self.meta.default_fn
        else { return Err(NoDefaultConstructorError.into()) };

        let ptr = self.try_idx_ptr(idx)?;
        unsafe { self.drop_at(idx); write_default(ptr) }

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
    pub fn swap_remove(&mut self, idx: usize) -> Result<RemovedDynVecValue<'_>, IndexOutOfBoundError> {
        Ok(RemovedDynVecValue {
            value_ptr: self.try_idx_ptr(idx)?,
            moved: false,

            vec: self,
        })
    }
}

impl Drop for DynVec {
    fn drop(&mut self) {
        self.clear();
        self.realloc_capacity(0);
    }
}

impl Extend<Box<dyn Any>> for DynVec {
    fn extend<I: IntoIterator<Item = Box<dyn Any>>>(&mut self, iter: I) {
        let it = iter.into_iter();
        let (lower, _) = it.size_hint();
        if lower > 0 { self.reserve(lower); }
        for item in it { self.push(item).expect("Attempted to extend with an incorrect type"); }
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

impl<'a> IntoIterator for &'a DynVec {
    type Item = DynVecValueRef<'a>;
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut DynVec {
    type Item = DynVecValueRefMut<'a>;
    type IntoIter = impl Iterator<Item = Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

/// Iterator returned by [`DynVec::drain`]
pub struct DynVecDrain<'a> {
    len: &'a mut usize,
    ptr: NonNull<()>,
    metadata: &'a DynVecMetadata,
    current_idx: usize,

    _ref: PhantomData<&'a mut dyn Any>,
}

impl<'a> Iterator for DynVecDrain<'a> {
    type Item = DrainedDynVecValue<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_idx < *self.len {
            // SAFETY: self.current_idx is inside the vec
            let ptr = unsafe { self.ptr.byte_add(self.metadata.layout().size() * self.current_idx) };
            self.current_idx += 1;
            Some(DrainedDynVecValue::<'a> {
                meta: self.metadata,
                value_ptr: ptr,
                moved: false,
            })
        }
        else {
            None
        }
    }
}

impl Drop for DynVecDrain<'_> {
    fn drop(&mut self) {
        for i in self.current_idx..*self.len {
            // SAFETY: i is inside the vec
            let ptr = unsafe { self.ptr.byte_add(self.metadata.layout().size() * i) };
            // SAFETY: 
            unsafe { self.metadata.drop_ptr(ptr) };
        }
        *self.len = 0;
    }
}

/// Shared reference guard for an element inside a `DynVec`.
///
/// The value can then be then be read as an Any trait object or casted into
/// its real type.
#[derive(Clone, Copy)]
pub struct DynVecValueRef<'a> {
    metadata: &'a DynVecMetadata,
    ptr: NonNull<()>,
    _data: PhantomData<&'a dyn Any>,
}

impl<'a> DynVecValueRef<'a> {
    /// Returns a trait object reference to the underlying value.
    pub fn as_any(&self) -> &'a dyn Any {
        let fat = from_raw_parts::<dyn Any>(self.ptr.as_ptr(), self.metadata.dyn_meta);
        unsafe { &*fat }
    }

    /// If the given type is the same as the value's type a reference to the
    /// value is returned.
    /// Otherwise Err(IncorrectTypeError) is returned.
    pub fn as_typed<T: 'static>(&self) -> Result<&'a T, IncorrectTypeError> {
        self.metadata.assert_type_t::<T>()?;
        Ok(unsafe { self.ptr.cast::<T>().as_ref() })
    }
}

/// Mutable reference guard for an element inside a `DynVec`.
///
/// The value can then be then be read as an Any trait object or casted into
/// its real type.
pub struct DynVecValueRefMut<'a> {
    metadata: &'a DynVecMetadata,
    ptr: NonNull<()>,
    _data: PhantomData<&'a mut dyn Any>,
}

impl<'a> DynVecValueRefMut<'a> {
    /// Returns a trait object mutable reference to the underlying value.
    pub fn as_any(&mut self) -> &'a mut dyn Any {
        let fat = from_raw_parts_mut::<dyn Any>(self.ptr.as_ptr(), self.metadata.dyn_meta);
        unsafe { &mut *fat }
    }

    /// If the given type is the same as the value's type a mutable reference
    /// to the value is returned.
    /// Otherwise Err(IncorrectTypeError) is returned.
    pub fn as_typed<T: 'static>(&mut self) -> Result<&'a mut T, IncorrectTypeError> {
        self.metadata.assert_type_t::<T>()?;
        Ok(unsafe { self.ptr.cast::<T>().as_mut() })
    }
}

/// Owning guard for an element drained from a `DynVec`.
/// Same as [`RemovedDynVecValue`] but for the [`DynVec::drain`] operation.
// FIXME: I would prefer this merged with RemovedDynVecValue but i havn't found
// a solution that I like.
pub struct DrainedDynVecValue<'a> {
    meta: &'a DynVecMetadata,
    value_ptr: NonNull<()>,
    moved: bool,
}

impl<'a> DrainedDynVecValue<'a> {
    /// Like [`RemovedDynVecValue::into_boxed_any`]
    pub fn into_boxed_any(mut self) -> Box<dyn Any> {
        let layout = self.meta.dyn_meta.layout();
        let data_ptr = alloc_or_dangling(layout);

        // SAFETY: Same Layout
        unsafe { data_ptr.copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()); };
        self.moved = true;

        // SAFETY: The Type is the correct one
        let fat = from_raw_parts_mut(data_ptr.as_ptr(), self.meta.dyn_meta);
        unsafe { Box::from_raw(fat) }
    }

    /// Like [`RemovedDynVecValue::into_typed`]
    pub fn into_typed<T: 'static>(mut self) -> Result<T, IncorrectTypeError> {
        self.meta.assert_type_t::<T>()?;
        // SAFETY: value_ptr is inistialized and won't be used again
        let out = unsafe { self.value_ptr.cast::<T>().read() };
        self.moved = true;
        Ok(out)
    }

    /// Like [`RemovedDynVecValue::push_into`]
    pub fn push_into(mut self, dst: &mut DynVec) -> Result<(), IncorrectTypeError> {
        dst.assert_type_meta(&self.meta)?;

        let layout = self.meta.dyn_meta.layout();
        let target_ptr = dst.push_and_get_ptr();

        // SAFETY: Same layout and just reserved the space
        unsafe { target_ptr.cast::<u8>().copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()) };
        self.moved = true;

        Ok(())
    }

    /// Like [`RemovedDynVecValue::set_into`]
    pub fn set_into(mut self, dst: &mut DynVec, idx: usize) -> Result<(), InsertionError> {
        dst.assert_type_meta(&self.meta)?;
        let target_ptr = dst.try_idx_ptr(idx)?;

        let layout = self.meta.dyn_meta.layout();

        // SAFETY: All elements are initialized
        unsafe { dst.meta.drop_ptr(target_ptr); }
        // SAFETY: Same layout and just freed the space
        unsafe { target_ptr.cast::<u8>().copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()) };
        self.moved = true;

        Ok(())
    }
}

impl<'a> Drop for DrainedDynVecValue<'a> {
    fn drop(&mut self) {
        if !self.moved {
            // SAFETY: Got the pointer from the vec, and if
            // consumed is false then it has never been moved.
            unsafe { self.meta.drop_ptr(self.value_ptr) };
        }
    }
}

/// Owning guard for an element removed from a `DynVec`.
///
/// The value can be consumed via typed extraction or moved into another `DynVec`.
/// The source vector is actually updated on `Drop` of this guard.
///
/// Typical ways to consume the guard:
/// - Move into another `DynVec` of the same type using [`RemovedDynVecValue::push_into`]
/// - Overwrite a position in another `DynVec` using [`RemovedDynVecValue::set_into`]
/// - Extract the concrete value with [`RemovedDynVecValue::into_typed`]
/// - Box as `dyn Any` with [`RemovedDynVecValue::into_boxed_any`]
pub struct RemovedDynVecValue<'a> {
    vec: &'a mut DynVec,
    value_ptr: NonNull<()>,
    /// Set to [`true`] when the value referenced has been moved elsewhere
    moved: bool,
}

impl<'a> RemovedDynVecValue<'a> {
    /// Consumes the guard and returns the value boxed as `dyn Any`.
    ///
    /// Allocates a new box and copies the bytes of the value into it.
    pub fn into_boxed_any(mut self) -> Box<dyn Any> {
        let layout = self.vec.meta.dyn_meta.layout();
        let data_ptr = alloc_or_dangling(layout);

        // SAFETY: Same Layout
        unsafe { data_ptr.copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()); };
        self.moved = true;

        // SAFETY: The Type is the correct one
        let fat = from_raw_parts_mut(data_ptr.as_ptr(), self.vec.meta.dyn_meta);
        unsafe { Box::from_raw(fat) }
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
        // SAFETY: value_ptr is inistialized and won't be used again
        let out = unsafe { self.value_ptr.cast::<T>().read() };
        self.moved = true;
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

        let layout = self.vec.meta.dyn_meta.layout();
        let target_ptr = dst.push_and_get_ptr();

        // SAFETY: Same layout and just reserved the space
        unsafe { target_ptr.cast::<u8>().copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()) };
        self.moved = true;

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
        let target_ptr = dst.try_idx_ptr(idx)?;

        let layout = self.vec.meta.dyn_meta.layout();

        // SAFETY: All elements are initialized
        unsafe { dst.meta.drop_ptr(target_ptr); }
        // SAFETY: Same layout and just freed the space
        unsafe { target_ptr.cast::<u8>().copy_from_nonoverlapping(self.value_ptr.cast::<u8>(), layout.size()) };
        self.moved = true;

        Ok(())
    }
}

impl<'a> Drop for RemovedDynVecValue<'a> {
    fn drop(&mut self) {
        debug_assert_ne!(self.vec.len, 0);
        let layout = self.vec.meta.dyn_meta.layout();

        if !self.moved {
            // SAFETY: Got the pointer from the vec, and if
            // consumed is false then it has never been moved.
            unsafe { self.vec.meta.drop_ptr(self.value_ptr) };
        }

        // SAFETY: Last element of the vec is in the vec
        let source_ptr = unsafe { self.vec.idx_ptr(self.vec.len - 1) };

        if source_ptr != self.value_ptr {
            // SAFETY: Layout is the same, and the target has been droped or moved
            unsafe { self.value_ptr.cast::<u8>().copy_from_nonoverlapping(source_ptr.cast::<u8>(), layout.size()) };
        }

        // Moved or droped the last element at this point
        self.vec.len -= 1;
    }
}

/// A typed shared reference view over a `DynVec`.
///
/// Could be just a slice but wasn't made that way and now too lazy to change tests and documentation hihi
pub struct TypedDynVecRef<'a, T: 'static> {
    slice: &'a [T],
}

impl<'a, T: 'static> TypedDynVecRef<'a, T> {
    /// Creates a typed view, returning Err(IncorrectTypeError) if the type
    /// does not match the one from the [`DynVec`].
    pub fn new(vec: &'a DynVec) -> Result<Self, IncorrectTypeError> {
        vec.assert_type_t::<T>()?;
        let len = vec.len;
        let ptr = vec.ptr.as_ptr() as *const T;
        Ok(Self {
            // SAFETY: hhhuuuuuh, safe
            slice: unsafe { slice::from_raw_parts(ptr, len) },
        })
    }

    /// Returns a shared slice over all elements.
    pub fn as_slice(&self) -> &'a [T] {
        self.slice
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
    _marker: std::marker::PhantomData<&'a mut [T]>,
}

impl<'a, T: 'static> TypedDynVecRefMut<'a, T> {
    /// Creates a typed mutable view, returning Err(IncorrectTypeError) if the type
    /// does not match the one from the [`DynVec`].
    pub fn new(vec: &'a mut DynVec) -> Result<Self, IncorrectTypeError> {
        vec.assert_type_t::<T>()?;
        Ok(Self { vec, _marker: std::marker::PhantomData })
    }

    /// Clears all elements from the underlying vector.
    pub fn clear(&mut self) {
        self.vec.clear()
    }

    /// Pushes a value without boxing or virtual dispatch.
    pub fn push(&mut self, val: T) {
        let target_ptr = self.vec.push_and_get_ptr();
        // SAFETY: We know this slot isn't initialized
        unsafe { target_ptr.cast::<T>().write(val); }
    }

    /// Sets index to value, dropping the old, without boxing.
    ///
    /// Returns `Err(IndexOutOfBoundError)` if `idx >= len`.
    pub fn set(&mut self, idx: usize, val: T) -> Result<(), IndexOutOfBoundError> {
        self.vec.assert_index(idx)?;
        *unsafe { self.get_unchecked_mut(idx) } = val;
        Ok(())
    }

    /// Removes at index and returns the removed value, swapping in the last.
    ///
    /// Returns `Err(IndexOutOfBoundError)` if `idx >= len`.
    pub fn swap_remove(&mut self, idx: usize) -> Result<T, IndexOutOfBoundError> {
        self.vec.assert_index(idx)?;
        let last_idx = self.vec.len - 1;

        // SAFETY: Indices are checked before
        let target = unsafe { self.vec.idx_ptr(idx) }.cast::<T>();
        let source = unsafe { self.vec.idx_ptr(last_idx) }.cast::<T>();

        // SAFETY: Items inside the vec's len are all initialized
        let removed_value = unsafe { target.read() };

        if target != source {
            // SAFETY: Target has been moved out, source should be 
            unsafe { target.copy_from_nonoverlapping(source, 1) };
        }
        
        self.vec.len -= 1;
        Ok(removed_value)
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
