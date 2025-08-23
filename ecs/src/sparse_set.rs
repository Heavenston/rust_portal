use utils::prelude::*;

/// Basically a Map<usize, T>, where the 'usize' is the sparse idx
#[derive_where::derive_where(Default)]
#[derive(Debug, Clone)]
pub struct SparseSet<T> {
    sparse_to_dense_indices: Vec<Option<PlusOneNonZeroUsize>>,
    dense_to_sparse_indices: Vec<usize>,
    dense_values: Vec<T>,
}

impl<T> SparseSet<T> {
    pub fn has(&self, sparse_idx: usize) -> bool {
        self.sparse_to_dense_indices.get(sparse_idx).copied().flatten().is_some()
    }

    pub fn get(&self, sparse_idx: usize) -> Option<&T> {
        let dense_idx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get();

        Some(&self.dense_values[dense_idx])
    }

    pub fn get_mut(&mut self, sparse_idx: usize) -> Option<&mut T> {
        let dense_idx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get();

        Some(&mut self.dense_values[dense_idx])
    }

    pub fn insert(&mut self, sparse_idx: usize, value: T) {
        if self.sparse_to_dense_indices.len() <= sparse_idx {
            self.sparse_to_dense_indices.resize_with(sparse_idx + 1, || None);
        }

        match &mut self.sparse_to_dense_indices[sparse_idx] {
            &mut Some(dense_idx) => {
                self.dense_values[dense_idx.get()] = value;
                self.dense_to_sparse_indices[dense_idx.get()] = sparse_idx;
            },
            val => {
                *val = Some(PlusOneNonZeroUsize::new(self.dense_values.len()));
                self.dense_values.push(value);
                self.dense_to_sparse_indices.push(sparse_idx);
                debug_assert_eq!(
                    self.dense_values.len(), self.dense_to_sparse_indices.len(),
                );
            },
        }
    }

    pub fn remove(&mut self, sparse_idx: usize) -> Option<T> {
        let dense_idx = self.sparse_to_dense_indices.get(sparse_idx)
            .copied().flatten()?.get();
        debug_assert!(dense_idx < self.sparse_to_dense_indices.len());

        self.sparse_to_dense_indices[sparse_idx] = None;

        if /* is last in dense array */ dense_idx + 1 == self.dense_values.len() {
            self.dense_to_sparse_indices.pop();
            self.dense_values.pop()
        }
        else {
            let moved_sparse_idx = self.dense_to_sparse_indices.last().copied()
                .expect("Cannot be empty here");
            let value = self.dense_values.swap_remove(dense_idx);
            debug_assert_eq!(
                self.dense_to_sparse_indices.swap_remove(dense_idx),
                sparse_idx
            );
            debug_assert_ne!(moved_sparse_idx, sparse_idx);
            debug_assert_eq!(
                self.sparse_to_dense_indices[moved_sparse_idx],
                Some(PlusOneNonZeroUsize::new(
                    self.dense_values.len()
                )),
            );
            self.sparse_to_dense_indices[moved_sparse_idx] =
                Some(PlusOneNonZeroUsize::new(dense_idx));

            Some(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SparseSet;

    #[test]
    fn insert_and_get_and_mutate() {
        let mut set: SparseSet<i32> = SparseSet::default();

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
        let mut set: SparseSet<&'static str> = SparseSet::default();

        set.insert(1, "first");
        // Overwrite value at the same sparse index should not grow dense storage
        let before_len = set.dense_values.len();
        set.insert(1, "second");
        assert_eq!(set.dense_values.len(), before_len);
        assert_eq!(set.get(1), Some(&"second"));
    }

    #[test]
    fn remove_last_element_updates_state() {
        let mut set: SparseSet<i32> = SparseSet::default();

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
        let mut set: SparseSet<&'static str> = SparseSet::default();

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
        let mut set: SparseSet<i32> = SparseSet::default();
        set.insert(2, 5);
        assert_eq!(set.remove(999), None);
        assert!(set.has(2));
        assert_eq!(set.get(2), Some(&5));
    }
}
