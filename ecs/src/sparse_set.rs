use utils::prelude::*;
use utils::mapped_nonzero::PlusOneNonZero;

type SparseIdx = u32;

pub trait SparseSetDenseStorage {
    type OwnedItem;
    type RefItem<'a>
        where Self: 'a;
    type RefMutItem<'a>
        where Self: 'a;

    fn len(&self) -> usize;
    fn push(&mut self, val: Self::OwnedItem);
    fn swap_remove(&mut self, idx: usize) -> Self::OwnedItem;
    fn set(&mut self, idx: usize, value: Self::OwnedItem);

    fn get(&self, idx: usize) -> Option<Self::RefItem<'_>>;
    fn get_mut(&mut self, idx: usize) -> Option<Self::RefMutItem<'_>>;

    fn iter<'a>(&'a self) -> impl Iterator<Item = Self::RefItem<'a>> + DoubleEndedIterator + ExactSizeIterator + Clone;
    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = Self::RefMutItem<'a>> + DoubleEndedIterator + ExactSizeIterator;
}

impl<T> SparseSetDenseStorage for Vec<T> {
    type OwnedItem = T;
    type RefItem<'a> = &'a T
        where T: 'a;
    type RefMutItem<'a> = &'a mut T
        where T: 'a;

    fn len(&self) -> usize {
        self.len()
    }

    fn push(&mut self, val: Self::OwnedItem) {
        self.push(val)
    }

    fn swap_remove(&mut self, idx: usize) -> Self::OwnedItem {
        self.swap_remove(idx)
    }

    fn set(&mut self, idx: usize, value: T) {
        self.as_mut_slice()[idx] = value;
    }

    fn get(&self, idx: usize) -> Option<&T> {
        self.as_slice().get(idx)
    }

    fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.as_mut_slice().get_mut(idx)
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &'a T> + DoubleEndedIterator + ExactSizeIterator + Clone {
        self.as_slice().iter()
    }

    fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> + DoubleEndedIterator + ExactSizeIterator {
        self.as_mut_slice().iter_mut()
    }
}

/// Basically a Map<SparseIdx, T>, where the 'SparseIdx' is the sparse idx
#[derive(Default, Debug, Clone)]
pub struct SparseSet<S: SparseSetDenseStorage> {
    sparse_to_dense_indices: Vec<Option<PlusOneNonZero<SparseIdx>>>,
    dense_to_sparse_indices: Vec<SparseIdx>,
    dense_values: S,
}

impl<S> SparseSet<S>
    where S: SparseSetDenseStorage,
{
    pub fn new(dense_values: S) -> Self {
        Self {
            sparse_to_dense_indices: default(),
            dense_to_sparse_indices: default(),
            dense_values,
        }
    }

    pub fn len(&self) -> usize {
        self.dense_values.len()
    }

    pub fn has(&self, sparse_idx: SparseIdx) -> bool {
        let sparse = ix!(sparse_idx);
        self.sparse_to_dense_indices.get(sparse).copied().flatten().is_some()
    }

    pub fn get(&self, sparse_idx: SparseIdx) -> Option<S::RefItem<'_>> {
        let sparse = ix!(sparse_idx);
        let dense_idx = self.sparse_to_dense_indices.get(sparse)
            .copied().flatten()?.get();
        self.dense_values.get(ix!(dense_idx))
    }

    pub fn get_mut(&mut self, sparse_idx: SparseIdx) -> Option<S::RefMutItem<'_>> {
        let sparse = ix!(sparse_idx);
        let dense_idx = self.sparse_to_dense_indices.get(sparse)
            .copied().flatten()?.get();
        self.dense_values.get_mut(ix!(dense_idx))
    }

    pub fn insert(&mut self, sparse_idx: SparseIdx, value: S::OwnedItem) {
        let sparse = ix!(sparse_idx);
        if self.sparse_to_dense_indices.len() <= sparse {
            self.sparse_to_dense_indices.resize_with(sparse + 1, || None);
        }

        match &mut self.sparse_to_dense_indices[sparse] {
            &mut Some(dense_idx) => {
                let d = ix!(dense_idx.get());
                self.dense_values.set(d, value);
                self.dense_to_sparse_indices[d] = sparse_idx;
            },
            stored_dense => {
                let new_dense: SparseIdx = SparseIdx::try_from(self.dense_values.len()).expect("no overflow");
                *stored_dense = Some(PlusOneNonZero::<SparseIdx>::new(new_dense));
                self.dense_values.push(value);
                self.dense_to_sparse_indices.push(sparse_idx);
                debug_assert_eq!(
                    self.dense_values.len(), self.dense_to_sparse_indices.len(),
                );
            },
        }
    }

    pub fn remove(&mut self, sparse_idx: SparseIdx) -> Option<S::OwnedItem> {
        let sparse = ix!(sparse_idx);
        let dense_idx = self.sparse_to_dense_indices.get(sparse)
            .copied().flatten()?.get();
        let d = ix!(dense_idx);
        debug_assert!(d < self.sparse_to_dense_indices.len());

        self.sparse_to_dense_indices[sparse] = None;

        let moved_sparse_idx = self.dense_to_sparse_indices.last().copied()
            .expect("Cannot be empty here");

        let value = self.dense_values.swap_remove(d);
        let removed_sparse_idx = self.dense_to_sparse_indices.swap_remove(d);
        debug_assert_eq!(removed_sparse_idx, sparse_idx);

        if moved_sparse_idx != sparse_idx {
            debug_assert_ne!(moved_sparse_idx, sparse_idx);
            debug_assert_eq!(
                self.sparse_to_dense_indices[ix!(moved_sparse_idx)],
                Some(PlusOneNonZero::<SparseIdx>::new(
                    SparseIdx::try_from(self.dense_values.len()).expect("no overflow")
                )),
            );
            self.sparse_to_dense_indices[ix!(moved_sparse_idx)] =
                Some(PlusOneNonZero::<SparseIdx>::new(dense_idx));
        }

        Some(value)
    }

    pub fn sparse_indices(&self) -> impl Iterator<Item = SparseIdx> + DoubleEndedIterator + Clone + ExactSizeIterator {
        self.dense_to_sparse_indices.iter().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (SparseIdx, S::RefItem<'_>)> + DoubleEndedIterator + Clone + ExactSizeIterator {
        self.dense_to_sparse_indices.iter().copied().zip(
            self.dense_values.iter()
        )
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (SparseIdx, S::RefMutItem<'_>)> + DoubleEndedIterator + ExactSizeIterator {
        self.dense_to_sparse_indices.iter().copied().zip(
            self.dense_values.iter_mut()
        )
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
}
