#![feature(ptr_metadata)]
#![warn(missing_docs)]
//! Type-erased, single-type vector.
//!
//! DynVec is a growable vector whose element type is picked at runtime, yet
//! all elements share that same concrete type. This enables dynamic component
//! storage while still providing fast typed access through zero-overhead views.
//!
//! - Construction: `DynVec::new::<T>()` or `DynVec::new_with_meta(DynVecMetadata::new::<T>())`.
//! - Untyped access: `get`/`get_mut` yield `&dyn Any`/`&mut dyn Any`.
//! - Typed views: `typed::<T>()`/`typed_mut::<T>()` provide fast typed methods without
//!   re-checking the type on every call.
//! - Mutation: `push`, `set`, `swap_remove`, `pop` operate on `Box<dyn Any>` or via typed views.
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
//! let mut t = v.typed_mut::<i32>();
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
use std::any::{ Any, TypeId };
use std::ops::{ Deref, DerefMut, Index, IndexMut };
use std::ptr::{ self, from_raw_parts, from_raw_parts_mut, NonNull };
use std::slice;

/// Metadata describing the element type stored in a `DynVec`.
///
/// Prefer constructing via [`DynVecMetadata::new`]. All fields are public for
/// transparency and potential interop. The struct cannot be constructed from
/// outside this crate to prevent inconsistent values; use `new::<T>()`.
pub struct DynVecMetadata {
    /// The `TypeId` of the element type.
    pub type_id: TypeId,
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
        Self { type_id: TypeId::of::<T>(), layout: Layout::new::<T>(), meta, _priv: () }
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
    /// v.push(Box::new(1_i32));
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
            // dangling is fine when capacity == 0; never dereferenced
            ptr: NonNull::dangling(),
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
    fn elem_size(&self) -> usize { self.meta.layout.size() }

    #[inline]
    fn elem_align(&self) -> usize { self.meta.layout.align() }

    #[inline]
    unsafe fn idx_ptr(&self, idx: usize) -> *mut u8 {
        debug_assert!(idx < self.len || idx == self.len && self.len <= self.capacity);
        unsafe { self.ptr.as_ptr().add(idx * self.elem_size()) }
    }

    #[inline]
    fn assert_type(&self, any: &dyn Any) {
        assert!(any.type_id() == self.meta.type_id, "TypeId mismatch in DynVec::push/set");
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

    // Vec-like API
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
        let mut new_cap = self.capacity.max(4);
        while new_cap < needed { new_cap = new_cap.saturating_mul(2); }
        let elem_size = self.elem_size();
        let align = self.elem_align();
        unsafe {
            if self.capacity == 0 {
                let size_bytes = new_cap.checked_mul(elem_size).expect("capacity overflow");
                let layout = Layout::from_size_align(size_bytes, align).expect("invalid layout");
                let new_ptr = alloc(layout);
                if new_ptr.is_null() { std::alloc::handle_alloc_error(layout); }
                self.ptr = NonNull::new_unchecked(new_ptr);
                self.capacity = new_cap;
            } else {
                let old_size = self.capacity.checked_mul(elem_size).expect("overflow");
                let old_layout = Layout::from_size_align(old_size, align).expect("invalid layout");
                let new_size = new_cap.checked_mul(elem_size).expect("overflow");
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

    /// Shrinks the capacity as much as possible.
    ///
    /// If `len == 0`, frees the allocation. Otherwise, shrinks to exactly `len`.
    pub fn shrink_to_fit(&mut self) {
        if self.capacity == self.len { return; }
        let elem_size = self.elem_size();
        let align = self.elem_align();
        unsafe {
            if self.len == 0 {
                if self.capacity > 0 {
                    let total_size = self.capacity * elem_size;
                    let layout = Layout::from_size_align(total_size, align).expect("invalid layout");
                    dealloc(self.ptr.as_ptr(), layout);
                    self.ptr = NonNull::dangling();
                    self.capacity = 0;
                }
            } else {
                let old_size = self.capacity * elem_size;
                let old_layout = Layout::from_size_align(old_size, align).expect("invalid layout");
                let new_size = self.len * elem_size;
                let new_ptr = realloc(self.ptr.as_ptr(), old_layout, new_size);
                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(
                        Layout::from_size_align(new_size, align).expect("invalid layout"),
                    );
                }
                self.ptr = NonNull::new_unchecked(new_ptr);
                self.capacity = self.len;
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
    /// Panics if `T` does not match the vector's element type.
    pub fn typed<T: 'static>(&self) -> TypedDynVecRef<'_, T> {
        TypedDynVecRef::<T>::new(self)
    }

    /// Creates a typed mutable view for element type `T`.
    ///
    /// Panics if `T` does not match the vector's element type.
    pub fn typed_mut<T: 'static>(&mut self) -> TypedDynVecRefMut<'_, T> {
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
    /// Panics if the boxed value's `TypeId` does not match the vector's element type.
    pub fn push(&mut self, val: Box<dyn Any>) {
        self.assert_type(val.as_ref());
        let raw: *mut dyn Any = Box::into_raw(val);
        let data_ptr = raw as *mut u8;
        self.reserve(1);
        unsafe {
            let dst = self.idx_ptr(self.len);
            ptr::copy_nonoverlapping(data_ptr, dst, self.elem_size());
            dealloc(data_ptr, self.meta.layout);
        }
        self.len += 1;
    }

    /// Pops the last element, if any, returning it boxed as `dyn Any`.
    ///
    /// Downcast the returned box with `Box::<dyn Any>::downcast::<T>()`.
    pub fn pop(&mut self) -> Option<Box<dyn Any>> {
        if self.len == 0 { return None; }
        Some(self.swap_remove(self.len - 1))
    }

    /// Replaces the element at `idx`, dropping the previous value in place.
    ///
    /// Panics if `idx` is out of bounds or the boxed value's `TypeId` mismatches.
    pub fn set(&mut self, idx: usize, val: Box<dyn Any>) {
        assert!(idx < self.len, "index out of bounds");
        self.assert_type(val.as_ref());
        let raw: *mut dyn Any = Box::into_raw(val);
        let data_ptr = raw as *mut u8;
        unsafe {
            self.drop_at(idx);
            let dst = self.idx_ptr(idx);
            ptr::copy_nonoverlapping(data_ptr, dst, self.elem_size());
            dealloc(data_ptr, self.meta.layout);
        }
    }

    /// Removes and returns the element at `idx` as `Box<dyn Any>`.
    ///
    /// The last element is moved into `idx` (swap-remove). Panics if out of bounds.
    pub fn swap_remove(&mut self, idx: usize) -> Box<dyn Any> {
        assert!(idx < self.len, "index out of bounds");
        unsafe {
            let data_ptr = alloc(self.meta.layout);
            if data_ptr.is_null() { std::alloc::handle_alloc_error(self.meta.layout); }
            let src = self.idx_ptr(idx);
            ptr::copy_nonoverlapping(src, data_ptr, self.elem_size());
            let meta = self.meta.meta;
            let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr as *mut (), meta);
            let boxed: Box<dyn Any> = Box::from_raw(fat);
            let last_idx = self.len - 1;
            if idx != last_idx {
                let last_ptr = self.idx_ptr(last_idx);
                ptr::copy_nonoverlapping(last_ptr, src, self.elem_size());
            }
            self.len -= 1;
            boxed
        }
    }
}

impl Index<usize> for DynVec {
    type Output = dyn Any;
    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.len, "index out of bounds");
        unsafe { self.as_dyn_ref_at(index) }
    }
}

impl IndexMut<usize> for DynVec {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.len, "index out of bounds");
        unsafe { self.as_dyn_mut_at(index) }
    }
}

impl Drop for DynVec {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.len {
                self.drop_at(i);
            }
            if self.capacity > 0 {
                let total_size = self.capacity * self.elem_size();
                let layout = Layout::from_size_align(total_size, self.elem_align())
                    .expect("invalid layout");
                dealloc(self.ptr.as_ptr(), layout);
            }
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
    pub fn new(vec: &'a DynVec) -> Self {
        assert!(TypeId::of::<T>() == vec.meta.type_id, "TypedDynVecRef::new: type mismatch");
        Self { vec, _marker: std::marker::PhantomData }
    }

    /// Returns the length of the underlying vector.
    pub fn len(&self) -> usize { self.vec.len }
    /// Returns `true` if empty.
    pub fn is_empty(&self) -> bool { self.vec.len == 0 }
    /// Gets `&T` at `idx`.
    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self.vec.len { return None; }
        unsafe { Some(&*(self.vec.idx_ptr(idx) as *const T)) }
    }

    /// Returns a shared slice over all elements.
    pub fn as_slice(&self) -> &[T] {
        let len = self.vec.len;
        let ptr: *const T = if len == 0 {
            // Use a properly aligned dangling pointer for T
            NonNull::<T>::dangling().as_ptr()
        } else {
            unsafe { self.vec.idx_ptr(0) as *const T }
        };
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
    pub fn new(vec: &'a mut DynVec) -> Self {
        assert!(TypeId::of::<T>() == vec.meta.type_id, "TypedDynVecRefMut::new: type mismatch");
        Self { vec, _marker: std::marker::PhantomData }
    }

    /// Returns the length of the underlying vector.
    pub fn len(&self) -> usize { self.vec.len }
    /// Returns `true` if empty.
    pub fn is_empty(&self) -> bool { self.vec.len == 0 }
    /// Gets `&T` at `idx`.
    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx >= self.vec.len { return None; }
        unsafe { Some(&*(self.vec.idx_ptr(idx) as *const T)) }
    }
    /// Gets `&mut T` at `idx`.
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        if idx >= self.vec.len { return None; }
        unsafe { Some(&mut *(self.vec.idx_ptr(idx) as *mut T)) }
    }
    /// Clears all elements from the underlying vector.
    pub fn clear(&mut self) { self.vec.clear() }
    /// Pushes a value without boxing or virtual dispatch.
    pub fn push(&mut self, val: T) {
        self.vec.reserve(1);
        unsafe {
            let dst = self.vec.idx_ptr(self.vec.len) as *mut T;
            std::ptr::write(dst, val);
        }
        self.vec.len += 1;
    }
    /// Sets index to value, dropping the old, without boxing.
    pub fn set(&mut self, idx: usize, val: T) {
        assert!(idx < self.vec.len, "index out of bounds");
        unsafe {
            self.vec.drop_at(idx);
            let dst = self.vec.idx_ptr(idx) as *mut T;
            std::ptr::write(dst, val);
        }
    }
    /// Removes at index and returns the removed value, swapping in the last.
    pub fn swap_remove(&mut self, idx: usize) -> T {
        assert!(idx < self.vec.len, "index out of bounds");
        unsafe {
            let src = self.vec.idx_ptr(idx) as *mut T;
            let out = std::ptr::read(src);
            let last_idx = self.vec.len - 1;
            if idx != last_idx {
                let last_ptr = self.vec.idx_ptr(last_idx) as *mut T;
                std::ptr::copy_nonoverlapping(last_ptr, src, 1);
            }
            self.vec.len -= 1;
            out
        }
    }
    /// Pops the last element, if any.
    pub fn pop(&mut self) -> Option<T> {
        if self.vec.len == 0 { return None; }
        Some(self.swap_remove(self.vec.len - 1))
    }

    /// Returns a shared slice over all elements.
    pub fn as_slice(&self) -> &[T] {
        let len = self.vec.len;
        let ptr: *const T = if len == 0 {
            NonNull::<T>::dangling().as_ptr()
        } else {
            unsafe { self.vec.idx_ptr(0) as *const T }
        };
        unsafe { slice::from_raw_parts(ptr, len) }
    }
    /// Returns a mutable slice over all elements.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.vec.len;
        let ptr: *mut T = if len == 0 {
            NonNull::<T>::dangling().as_ptr()
        } else {
            unsafe { self.vec.idx_ptr(0) as *mut T }
        };
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

impl<'a, T: 'static> Index<usize> for TypedDynVecRef<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output { &self.as_slice()[index] }
}

impl<'a, T: 'static> Index<usize> for TypedDynVecRefMut<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output { &self.as_slice()[index] }
}

impl<'a, T: 'static> IndexMut<usize> for TypedDynVecRefMut<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output { &mut self.as_mut_slice()[index] }
}

#[cfg(test)]
mod tests {
    use super::*;
    // no extra imports
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn test_dyn_vec_i32() {
        let mut dyn_vec = DynVec::new::<i32>();
        assert_eq!(dyn_vec.metadata().type_id, std::any::TypeId::of::<i32>());

        // Use typed() helper for typed API
        let mut view = dyn_vec.typed_mut::<i32>();

        // Test push and len
        view.push(10_i32);
        view.push(20_i32);
        view.push(30_i32);
        assert_eq!(view.len(), 3);

        // Test get
        let val = view.get(1).unwrap();
        assert_eq!(*val, 20);

        // Test get_mut
        let mut_val = view.get_mut(1).unwrap();
        *mut_val = 25;
        let val_after_mut = view.get(1).unwrap();
        assert_eq!(*val_after_mut, 25);

        // Test set
        view.set(0, 5);
        assert_eq!(*view.get(0).unwrap(), 5);
        assert_eq!(view.len(), 3); // Length should not change

        // Test swap_remove
        let removed = view.swap_remove(0); // Removes 5, swaps in 30
        assert_eq!(removed, 5);
        assert_eq!(view.len(), 2);
        assert_eq!(*view.get(0).unwrap(), 30);
        assert_eq!(*view.get(1).unwrap(), 25);
    }

    #[test]
    #[should_panic]
    fn test_type_mismatch_push() {
        let mut dyn_vec = DynVec::new::<i32>();
        dyn_vec.push(Box::new("hello".to_string())); // Panic!
    }

    #[test]
    fn test_growth_and_bounds() {
        let mut v = DynVec::new::<u64>();
        {
            let mut tv = v.typed_mut::<u64>();
            for i in 0..100 { tv.push(i as u64); }
            assert_eq!(tv.len(), 100);
            assert_eq!(*tv.get(50).unwrap(), 50);
        }
        let removed = {
            let mut tv = v.typed_mut::<u64>();
            tv.swap_remove(10)
        };
        assert_eq!(removed, 10);
        assert_eq!(v.len(), 99);
        let tv = v.typed::<u64>();
        assert!(tv.get(999).is_none());

        // Vec-like helpers
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
            let mut tv = v.typed_mut::<i32>();
            tv.push(1_i32);
            tv.push(2_i32);
            tv.push(3_i32);
            assert_eq!(tv.len(), 3);
            assert_eq!(tv.pop().unwrap(), 3);
        }
        assert_eq!(v.len(), 2);
        v.clear();
        assert!(v.is_empty());
        // capacity unchanged after clear
        let cap = v.capacity();
        {
            let mut tv = v.typed_mut::<i32>();
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
            let mut tv = v.typed_mut::<DropProbe>();
            for _ in 0..5 { tv.push(DropProbe { hits: hits.clone() }); }
            // set should drop the old element
            tv.set(2, DropProbe { hits: hits.clone() });
            // swap_remove should move last into idx and return removed value
            let _r: DropProbe = tv.swap_remove(1);
            drop(_r);
            // clear should drop all remaining
            tv.clear();
        }
        assert_eq!(hits.load(Ordering::SeqCst), 6); // 5 initial + 1 replaced
        // drop of v after clear should drop nothing more
        drop(v);
        assert_eq!(hits.load(Ordering::SeqCst), 6);
    }

    #[test]
    fn test_typed_views() {
        let mut v = DynVec::new::<i32>();
        {
            let mut tv = v.typed_mut::<i32>();
            tv.push(1);
            tv.push(2);
            tv.push(3);
        }
        let view = v.typed::<i32>();
        assert_eq!(view.len(), 3);
        assert_eq!(*view.get(1).unwrap(), 2);
        let mut view_mut = v.typed_mut::<i32>();
        assert_eq!(*view_mut.get_mut(2).unwrap(), 3);
        view_mut.set(1, 20);
        view_mut.push(4);
        assert_eq!(view_mut.swap_remove(0), 1);
        assert_eq!(view_mut.pop().unwrap(), 3);
    }
}
