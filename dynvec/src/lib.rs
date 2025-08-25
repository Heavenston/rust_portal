#![feature(ptr_metadata)]
//! Type-erased, single-type vector. (ai-generated)
//!
//! DynVec is a type-erased vector that stores elements of exactly one
//! concrete type chosen at runtime. The element type is identified by
//! a pair of `TypeId` and `Layout`. Memory is managed manually via
//! `std::alloc` and elements are dropped using trait-object drop glue
//! reconstructed from stored metadata.
//!
//! - Construction: `DynVec::new(TypeId::of::<T>(), Layout::new::<T>())`.
//! - Pushing values: `push(Box::new(value))` (panics if `TypeId` mismatches).
//! - Access: `get`/`get_mut` return `&dyn Any`/`&mut dyn Any` which you then
//!   downcast to the concrete type.
//! - Removal: `swap_remove` and `pop` return `Box<dyn Any>` which can be
//!   downcast to the concrete type.
//!
//! Safety invariants (maintained internally):
//! - The backing allocation always has capacity for `capacity * layout.size()` bytes
//!   with alignment `layout.align()`.
//! - The initialized prefix `[0, len)` contains valid values of the element type.
//! - Elements are dropped exactly once on `clear`/`set`/`swap_remove`/`drop`.

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::any::{Any, TypeId};
use std::ptr::{self, from_raw_parts, from_raw_parts_mut, NonNull};

/// Type-erased vector storing a single runtime-selected element type.
///
/// See the crate-level docs for overview and safety guarantees.

pub struct DynVec {
    ptr: NonNull<u8>,
    len: usize,
    capacity: usize, // in elements
    type_id: TypeId,
    layout: Layout, // element layout
    // Metadata for building &dyn Any fat pointers and running correct drop glue.
    meta: Option<std::ptr::DynMetadata<dyn Any>>,
}

impl DynVec {
    /// Creates an empty `DynVec` for elements of the given runtime type.
    ///
    /// - `type_id`: must be `TypeId::of::<T>()` for the intended element type `T`.
    /// - `layout`: must be `Layout::new::<T>()` for the same `T`.
    ///
    /// The vector starts empty and does not allocate until elements are pushed or
    /// `reserve`/`with_capacity` is used.
    pub fn new(type_id: TypeId, layout: Layout) -> Self {
        Self {
            // dangling is fine when capacity == 0; never dereferenced
            ptr: NonNull::dangling(),
            len: 0,
            capacity: 0,
            type_id,
            layout,
            meta: None,
        }
    }

    /// Creates a new `DynVec` with the given element type and capacity (in elements).
    ///
    /// Allocates enough space to hold at least `capacity` elements of the runtime type.
    pub fn with_capacity(type_id: TypeId, layout: Layout, capacity: usize) -> Self {
        let mut v = Self::new(type_id, layout);
        if capacity > 0 {
            v.reserve(capacity);
        }
        v
    }

    #[inline]
    fn elem_size(&self) -> usize {
        self.layout.size()
    }

    #[inline]
    fn elem_align(&self) -> usize {
        self.layout.align()
    }

    #[inline]
    unsafe fn idx_ptr(&self, idx: usize) -> *mut u8 {
        debug_assert!(idx < self.len || idx == self.len && self.len <= self.capacity);
        unsafe { self.ptr.as_ptr().add(idx * self.elem_size()) }
    }

    fn ensure_capacity_for_push(&mut self) {
        if self.len < self.capacity {
            return;
        }
        // grow
        let new_cap = if self.capacity == 0 { 4 } else { self.capacity.saturating_mul(2) };
        let elem_size = self.elem_size();
        let align = self.elem_align();

        unsafe {
            if self.capacity == 0 {
                let size_bytes = new_cap
                    .checked_mul(elem_size)
                    .expect("capacity overflow");
                let layout = Layout::from_size_align(size_bytes, align).expect("invalid layout");
                let new_ptr = alloc(layout);
                if new_ptr.is_null() {
                    std::alloc::handle_alloc_error(layout);
                }
                self.ptr = NonNull::new_unchecked(new_ptr);
                self.capacity = new_cap;
            } else {
                let old_size = self.capacity.checked_mul(elem_size).expect("overflow");
                let old_layout = Layout::from_size_align(old_size, align).expect("invalid layout");
                let new_size = new_cap.checked_mul(elem_size).expect("overflow");
                let new_ptr = realloc(self.ptr.as_ptr(), old_layout, new_size);
                if new_ptr.is_null() {
                    // SAFETY: old_layout describes current allocation
                    std::alloc::handle_alloc_error(
                        Layout::from_size_align(new_size, align).expect("invalid layout"),
                    );
                }
                self.ptr = NonNull::new_unchecked(new_ptr);
                self.capacity = new_cap;
            }
        }
    }

    #[inline]
    fn assert_type(&self, any: &dyn Any) {
        assert!(
            any.type_id() == self.type_id,
            "TypeId mismatch in DynVec::push/set"
        );
    }

    #[inline]
    unsafe fn drop_at(&mut self, idx: usize) {
        debug_assert!(idx < self.len);
        if let Some(meta) = self.meta {
            let data_ptr = unsafe { self.idx_ptr(idx) as *mut () };
            let fat: *mut dyn Any = from_raw_parts_mut::<dyn Any>(data_ptr, meta);
            unsafe { ptr::drop_in_place(fat) };
        } else {
            // No metadata implies no initialized elements should exist
            // Nothing to drop
        }
    }

    #[inline]
    unsafe fn as_dyn_ref_at<'a>(&self, idx: usize) -> &'a dyn Any {
        let meta = self
            .meta
            .expect("internal error: metadata not set for non-empty DynVec");
        let data_ptr = unsafe { self.idx_ptr(idx) as *const () };
        let fat: *const dyn Any = from_raw_parts::<dyn Any>(data_ptr, meta);
        unsafe { &*fat }
    }

    #[inline]
    unsafe fn as_dyn_mut_at<'a>(&mut self, idx: usize) -> &'a mut dyn Any {
        let meta = self
            .meta
            .expect("internal error: metadata not set for non-empty DynVec");
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

    /// Reserves capacity for at least `additional` more elements to be inserted.
    ///
    /// May reallocate. Uses a geometric growth strategy similar to `Vec`.
    pub fn reserve(&mut self, additional: usize) {
        let needed = self.len.saturating_add(additional);
        if needed <= self.capacity { return; }
        let mut new_cap = self.capacity.max(4);
        if new_cap == 0 { new_cap = 4; }
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

    /// Appends an element to the back.
    ///
    /// Panics if the boxed value's `TypeId` does not match the vector's element type.
    pub fn push(&mut self, val: Box<dyn Any>) {
        self.assert_type(val.as_ref());
        let raw: *mut dyn Any = Box::into_raw(val);
        let data_ptr = raw as *mut u8;
        let meta = std::ptr::metadata(raw);
        if self.meta.is_none() { self.meta = Some(meta); }
        self.ensure_capacity_for_push();
        unsafe {
            let dst = self.idx_ptr(self.len);
            ptr::copy_nonoverlapping(data_ptr, dst, self.elem_size());
            dealloc(data_ptr, self.layout);
        }
        self.len += 1;
    }

    /// Pops the last element, if any, returning it boxed as `dyn Any`.
    ///
    /// Downcast the returned box with `Box::<dyn Any>::downcast::<T>()`.
    pub fn pop(&mut self) -> Option<Box<dyn Any>> {
        if self.len == 0 { return None; }
        let idx = self.len - 1;
        Some(self.swap_remove(idx))
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
            dealloc(data_ptr, self.layout);
        }
    }

    /// Removes and returns the element at `idx` as `Box<dyn Any>`.
    ///
    /// The last element is moved into `idx` (swap-remove). Panics if out of bounds.
    pub fn swap_remove(&mut self, idx: usize) -> Box<dyn Any> {
        assert!(idx < self.len, "index out of bounds");
        unsafe {
            let data_ptr = alloc(self.layout);
            if data_ptr.is_null() { std::alloc::handle_alloc_error(self.layout); }
            let src = self.idx_ptr(idx);
            ptr::copy_nonoverlapping(src, data_ptr, self.elem_size());
            let meta = self.meta.expect("metadata not set");
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

// AnyVec trait removed; methods are inherent on DynVec

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn test_dyn_vec_i32() {
        let mut dyn_vec = DynVec::new(TypeId::of::<i32>(), Layout::new::<i32>());

        // Test push and len
        dyn_vec.push(Box::new(10_i32));
        dyn_vec.push(Box::new(20_i32));
        dyn_vec.push(Box::new(30_i32));
        assert_eq!(dyn_vec.len(), 3);

        // Test get
        let val = dyn_vec.get(1).unwrap().downcast_ref::<i32>().unwrap();
        assert_eq!(*val, 20);

        // Test get_mut
        let mut_val = dyn_vec
            .get_mut(1)
            .unwrap()
            .downcast_mut::<i32>()
            .unwrap();
        *mut_val = 25;
        let val_after_mut = dyn_vec
            .get(1)
            .unwrap()
            .downcast_ref::<i32>()
            .unwrap();
        assert_eq!(*val_after_mut, 25);

        // Test set
        dyn_vec.set(0, Box::new(5_i32));
        assert_eq!(
            *dyn_vec.get(0).unwrap().downcast_ref::<i32>().unwrap(),
            5
        );
        assert_eq!(dyn_vec.len(), 3); // Length should not change

        // Test swap_remove
        let removed = dyn_vec.swap_remove(0); // Removes 5, swaps in 30
        assert_eq!(*removed.downcast::<i32>().unwrap(), 5);
        assert_eq!(dyn_vec.len(), 2);
        assert_eq!(
            *dyn_vec.get(0).unwrap().downcast_ref::<i32>().unwrap(),
            30
        );
        assert_eq!(
            *dyn_vec.get(1).unwrap().downcast_ref::<i32>().unwrap(),
            25
        );

        // Test drop
        drop(dyn_vec);
    }

    #[test]
    #[should_panic]
    fn test_type_mismatch_push() {
        let mut dyn_vec = DynVec::new(TypeId::of::<i32>(), Layout::new::<i32>());
        dyn_vec.push(Box::new("hello".to_string())); // Panic!
    }

    #[test]
    fn test_growth_and_bounds() {
        let mut v = DynVec::new(TypeId::of::<u64>(), Layout::new::<u64>());
        for i in 0..100 {
            v.push(Box::new(i as u64));
        }
        assert_eq!(v.len(), 100);
        assert_eq!(
            *v.get(50).unwrap().downcast_ref::<u64>().unwrap(),
            50
        );
        let removed = v.swap_remove(10);
        assert_eq!(*removed.downcast::<u64>().unwrap(), 10);
        assert_eq!(v.len(), 99);
        assert!(v.get(999).is_none());

        // Vec-like helpers
        assert!(!v.is_empty());
    }

    #[test]
    fn test_capacity_reserve_clear_pop_shrink() {
        let mut v = DynVec::with_capacity(TypeId::of::<i32>(), Layout::new::<i32>(), 0);
        assert!(v.is_empty());
        assert_eq!(v.capacity(), 0);

        v.reserve(10);
        assert!(v.capacity() >= 10);

        v.push(Box::new(1_i32));
        v.push(Box::new(2_i32));
        v.push(Box::new(3_i32));
        assert_eq!(v.len(), 3);
        assert_eq!(*v.pop().unwrap().downcast::<i32>().unwrap(), 3);
        assert_eq!(v.len(), 2);
        v.clear();
        assert!(v.is_empty());
        // capacity unchanged after clear
        let cap = v.capacity();
        v.push(Box::new(4_i32));
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
        let mut v = DynVec::new(TypeId::of::<DropProbe>(), Layout::new::<DropProbe>());
        for _ in 0..5 { v.push(Box::new(DropProbe { hits: hits.clone() })); }
        // set should drop the old element
        v.set(2, Box::new(DropProbe { hits: hits.clone() }));
        // swap_remove should move last into idx and return removed value
        let r = v.swap_remove(1);
        // dropping returned box should drop one element
        drop(r);
        // clear should drop all remaining
        v.clear();
        assert_eq!(hits.load(Ordering::SeqCst), 6); // 5 initial + 1 replaced
        // drop of v after clear should drop nothing more
        drop(v);
        assert_eq!(hits.load(Ordering::SeqCst), 6);
    }
}
