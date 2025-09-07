use crate::index_map::{IndexMap, IndexMapIndex};

use std::fmt::Debug;
use std::iter::zip;
use utils::prelude::*;
use utils::mapped_nonzero::{
    NonZeroPrimitive,
    plus_one::{ PlusOneMappable, PlusOneNonZero },
};

pub trait SparseSetDenseStorageInput<C>: SparseSetDenseStorage {
    fn push(&mut self, item: C);
    fn set(&mut self, idx: Self::DenseIdx, item: C);
}

// TODO: Invariants of this trait should be layed out precisely
pub trait SparseSetDenseStorage {
    type SparseIdx: Copy + Debug + IndexMapIndex + PartialEq;
    type DenseIdx: Copy + Debug + IndexMapIndex + PartialEq;
    type PrimitiveDenseIdx: PlusOneMappable + NonZeroPrimitive + Into<Self::DenseIdx> + From<Self::DenseIdx>;

    type RemovedOutput<'a>
        where Self: 'a;
    type RefItem<'a>
        where Self: 'a;
    type RefMutItem<'a>
        where Self: 'a;

    /// Must start at `0`
    fn len(&self) -> Self::DenseIdx;

    /// Must work the same way as [`std::vec::Vec::swap_remove`]
    fn swap_remove(&mut self, idx: Self::DenseIdx) -> Self::RemovedOutput<'_>;

    /// Must return None if and only if `idx` is strictly inferior to the length
    fn get(&self, idx: Self::DenseIdx) -> Option<Self::RefItem<'_>>;
    /// Must return None if and only if `idx` is strictly inferior to the length
    fn get_mut(&mut self, idx: Self::DenseIdx) -> Option<Self::RefMutItem<'_>>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::RefItem<'a>> + DoubleEndedIterator + ExactSizeIterator + Clone;
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = Self::RefMutItem<'a>> + DoubleEndedIterator + ExactSizeIterator;
}

impl<T> SparseSetDenseStorageInput<T> for Vec<T> {
    fn push(&mut self, item: T) {
        self.push(item)
    }

    fn set(&mut self, idx: usize, item: T) {
        self[idx] = item;
    }
}

impl<T> SparseSetDenseStorage for Vec<T> {
    type SparseIdx = u32;
    type DenseIdx = usize;
    type PrimitiveDenseIdx = usize;

    type RemovedOutput<'a> = T
        where Self: 'a;
    type RefItem<'a> = &'a T
        where Self: 'a;
    type RefMutItem<'a> = &'a mut T
        where Self: 'a;

    fn len(&self) -> usize {
        self.len()
    }

    fn swap_remove(&mut self, idx: usize) -> Self::RemovedOutput<'_> {
        self.swap_remove(idx)
    }

    fn get(&self, idx: usize) -> Option<Self::RefItem<'_>> {
        self.as_slice().get(idx)
    }

    fn get_mut(&mut self, idx: usize) -> Option<Self::RefMutItem<'_>> {
        self.as_mut_slice().get_mut(idx)
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::RefItem<'a>> + DoubleEndedIterator + ExactSizeIterator + Clone {
        self.as_slice().iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = Self::RefMutItem<'a>> + DoubleEndedIterator + ExactSizeIterator {
        self.as_mut_slice().iter_mut()
    }
}

pub struct DiconstructedSparseSet<S: SparseSetDenseStorage> {
    pub sparse_to_dense_indices: IndexMap<Option<PlusOneNonZero<S::PrimitiveDenseIdx>>, S::SparseIdx>,
    pub dense_to_sparse_indices: IndexMap<S::SparseIdx, S::DenseIdx>,
    pub dense_values: S,
}

/// Basically a Map<SparseIdx, T>, where the 'SparseIdx' is the sparse idx
#[derive(Default, Debug, Clone)]
pub struct SparseSet<S: SparseSetDenseStorage> {
    sparse_to_dense_indices: IndexMap<Option<PlusOneNonZero<S::PrimitiveDenseIdx>>, S::SparseIdx>,
    dense_to_sparse_indices: IndexMap<S::SparseIdx, S::DenseIdx>,
    dense_values: S,
}

impl<S> SparseSet<S>
    where S: SparseSetDenseStorage,
{
    pub fn new(dense_values: S) -> Self {
        assert_eq!(dense_values.len(), S::DenseIdx::from_usize(0));
        Self {
            sparse_to_dense_indices: default(),
            dense_to_sparse_indices: default(),
            dense_values,
        }
    }

    pub fn into_deconstructed(self) -> DiconstructedSparseSet<S> {
        DiconstructedSparseSet {
            sparse_to_dense_indices: self.sparse_to_dense_indices,
            dense_to_sparse_indices: self.dense_to_sparse_indices,
            dense_values: self.dense_values,
        }
    }

    pub fn len(&self) -> S::DenseIdx {
        self.dense_values.len()
    }

    pub fn has(&self, sparse_idx: S::SparseIdx) -> bool {
        self.sparse_to_dense_indices
            .get(sparse_idx).copied().flatten().is_some()
    }

    pub fn get(&self, sparse_idx: S::SparseIdx) -> Option<S::RefItem<'_>> {
        let dense_idx: S::DenseIdx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get().into();
        self.dense_values.get(dense_idx)
    }

    pub fn get_mut(&mut self, sparse_idx: S::SparseIdx) -> Option<S::RefMutItem<'_>> {
        let dense_idx: S::DenseIdx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get().into();
        self.dense_values.get_mut(dense_idx)
    }

    pub fn insert<I>(&mut self, sparse_idx: S::SparseIdx, value: I)
        where S: SparseSetDenseStorageInput<I>
    {
        self.sparse_to_dense_indices.extend_until(sparse_idx, || None);

        match &mut self.sparse_to_dense_indices[sparse_idx] {
            &mut Some(dense_idx) => {
                let d: S::DenseIdx = dense_idx.get().into();
                self.dense_values.set(d, value);
                self.dense_to_sparse_indices[d] = sparse_idx;
            },
            stored_dense => {
                let new_dense = self.dense_values.len();
                *stored_dense = Some(PlusOneNonZero::new(new_dense.into()));
                self.dense_values.push(value);
                self.dense_to_sparse_indices.push(sparse_idx);
                debug_assert_eq!(
                    self.dense_values.len(), self.dense_to_sparse_indices.len(),
                );
            },
        }
    }

    pub fn remove(&mut self, sparse_idx: S::SparseIdx) -> Option<S::RemovedOutput<'_>> {
        let dense_idx: S::DenseIdx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get().into();

        self.sparse_to_dense_indices[sparse_idx] = None;

        let moved_sparse_idx = self.dense_to_sparse_indices.last().copied()
            .expect("Cannot be empty here");

        if moved_sparse_idx != sparse_idx {
            self.sparse_to_dense_indices[moved_sparse_idx] =
                Some(PlusOneNonZero::new(dense_idx.into()));
        }

        let value = self.dense_values.swap_remove(dense_idx);
        let removed_sparse_idx = self.dense_to_sparse_indices.swap_remove(dense_idx);
        debug_assert_eq!(removed_sparse_idx, sparse_idx);

        Some(value)
    }

    pub fn sparse_indices(&self) -> impl Iterator<Item = S::SparseIdx> + DoubleEndedIterator + Clone + ExactSizeIterator {
        self.dense_to_sparse_indices.values().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (S::SparseIdx, S::RefItem<'_>)> + DoubleEndedIterator + Clone + ExactSizeIterator {
        zip(self.dense_to_sparse_indices.values().copied(), self.dense_values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (S::SparseIdx, S::RefMutItem<'_>)> + DoubleEndedIterator + ExactSizeIterator {
        zip(self.dense_to_sparse_indices.values().copied(), self.dense_values.iter_mut())
    }

    /// Allows you to mutate the internal dense value storage
    ///
    /// Must *not* change any of the invariants of the [`SparseSetDenseStorage`] trait.
    pub fn unsafely_mutate_dense_values(&mut self, mutator: impl FnOnce(&mut S)) {
        mutator(&mut self.dense_values);
    }
}

#[cfg(test)]
mod tests {
    use super::SparseSet;

    #[test]
    fn insert_and_get_and_mutate() {
        let mut set = SparseSet::<Vec<i32>>::default();

        assert!(!set.has(3));
        assert_eq!(set.get(3), None);

        set.insert(3, 10);
        assert!(set.has(3));
        assert_eq!(set.get(3), Some(&10));

        if let Some(v) = set.get_mut(3) { *v += 1; }
        assert_eq!(set.get(3), Some(&11));
    }

    #[test]
    fn insert_overwrite_same_sparse_index() {
        let mut set = SparseSet::<Vec<&'static str>>::default();

        set.insert(1, "first");
        // Overwrite value at the same sparse index should not grow dense storage
        let before_len = set.dense_values.len();
        set.insert(1, "second");
        assert_eq!(set.dense_values.len(), before_len);
        assert_eq!(set.get(1), Some(&"second"));
    }

    #[test]
    fn remove_last_element_updates_state() {
        let mut set = SparseSet::<Vec<i32>>::default();

        set.insert(1, 100);
        set.insert(7, 200); // this is last in dense order

        let removed = set.remove(7);
        assert_eq!(removed, Some(200));
        assert!(!set.has(7));
        assert_eq!(set.get(7), None);
        assert!(set.has(1));
        assert_eq!(set.get(1), Some(&100));
    }

    #[test]
    fn remove_middle_element_swaps_with_last_and_clears_mapping() {
        let mut set = SparseSet::<Vec<&'static str>>::default();

        set.insert(10, "a"); // dense 0
        set.insert(20, "b"); // dense 1 (middle)
        set.insert(30, "c"); // dense 2 (last)

        // Remove middle (sparse 20), should return its value and move last (30)
        let removed = set.remove(20);
        assert_eq!(removed, Some("b"));
        assert!(!set.has(20));
        assert_eq!(set.get(20), None);

        // The moved sparse index should still resolve correctly
        assert!(set.has(30));
        assert_eq!(set.get(30), Some(&"c"));
        assert!(set.has(10));
        assert_eq!(set.get(10), Some(&"a"));

        // Removing again should work and not crash
        assert_eq!(set.remove(20), None);
    }

    #[test]
    fn remove_nonexistent_returns_none_and_noop() {
        let mut set = SparseSet::<Vec<i32>>::default();
        set.insert(2, 5);
        assert_eq!(set.remove(999), None);
        assert!(set.has(2));
        assert_eq!(set.get(2), Some(&5));
    }

    #[test]
    fn iter_yields_pairs_in_dense_order_and_rev() {
        let mut set = SparseSet::<Vec<&'static str>>::default();
        set.insert(10, "a");
        set.insert(20, "b");
        set.insert(30, "c");

        let fwd: Vec<_> = set.iter().collect();
        assert_eq!(fwd, vec![(10, &"a"), (20, &"b"), (30, &"c")]);

        let rev: Vec<_> = set.iter().rev().collect();
        assert_eq!(rev, vec![(30, &"c"), (20, &"b"), (10, &"a")]);

        // After removing a middle element, order reflects swap-remove
        let _ = set.remove(20);
        let fwd2: Vec<_> = set.iter().collect();
        assert_eq!(fwd2, vec![(10, &"a"), (30, &"c")]);
    }

    #[test]
    fn iter_exact_size_and_clone_and_double_ended() {
        let mut set = SparseSet::<Vec<i32>>::default();
        set.insert(1, 1);
        set.insert(2, 2);
        set.insert(3, 3);

        let mut it = set.iter();
        assert_eq!(it.len(), 3);

        let mut it2 = it.clone();
        assert_eq!(it.len(), it2.len());
        assert_eq!(it.next(), it2.next());

        // Double-ended iteration consistency
        let back1 = it.next_back();
        let back2 = it2.next_back();
        assert_eq!(back1, back2);
        // Remaining length adjusts
        assert_eq!(it.len(), it2.len());
    }

    #[test]
    fn iter_mut_allows_in_place_updates() {
        let mut set = SparseSet::<Vec<i64>>::default();
        set.insert(4, 10);
        set.insert(7, 20);
        set.insert(9, 30);

        for (idx, v) in set.iter_mut() {
            // simple transform using sparse index
            *v += i64::from(idx);
        }

        assert_eq!(set.get(4), Some(&(10 + 4)));
        assert_eq!(set.get(7), Some(&(20 + 7)));
        assert_eq!(set.get(9), Some(&(30 + 9)));
    }

    #[test]
    fn len_and_sparse_indices_match_insertions() {
        let mut set = SparseSet::<Vec<i32>>::default();
        assert_eq!(set.len(), 0);
        set.insert(4, 10);
        set.insert(7, 20);
        set.insert(9, 30);
        assert_eq!(set.len(), 3);
        let indices: Vec<_> = set.sparse_indices().collect();
        // Dense order is [4,7,9] after these insertions
        assert_eq!(indices, vec![4, 7, 9]);
    }
}
