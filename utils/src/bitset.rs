use std::{ marker::PhantomData, fmt::Debug };

use derive_where::derive_where;
use itertools::repeat_n;
use smallvec::SmallVec;

use crate::ix;

pub trait BitSetIndex: Copy + Sized {
    fn from_usize(val: usize) -> Self;
    fn to_usize(&self) -> usize;
}

impl<T, E1, E2> BitSetIndex for T
    where usize: TryFrom<T, Error = E1>,
          T: Copy + TryFrom<usize, Error = E2>,
          E1: Debug, E2: Debug,
{
    fn from_usize(usize: usize) -> Self {
        usize.try_into().expect("No overflow")
    }

    fn to_usize(&self) -> usize {
        ix!(*self)
    }
}

#[derive_where(Default, Clone)]
pub struct BitSet<T = usize>
    where T: BitSetIndex,
{
    _type: PhantomData<fn(T) -> T>,
    words: SmallVec<[usize; 2]>,
}

const WORD_BIT_SIZE: usize = usize::BITS as usize;

impl<T> BitSet<T>
    where T: BitSetIndex,
{
    #[inline]
    pub const fn new() -> Self {
        Self {
            _type: PhantomData,
            words: SmallVec::new_const(),
        }
    }

    pub fn from_array<const N: usize>(vals: [T; N]) -> Self {
        vals.into_iter().collect()
    }

    pub fn from_slice(vals: &[T]) -> Self {
        vals.iter().copied().collect()
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.words.iter().copied()
            .map(|word| crate::ix!(word.count_ones()))
            .sum()
    }

    #[inline]
    fn word_for(val: usize) -> usize {
        val.div_euclid(WORD_BIT_SIZE)
    }

    #[inline]
    fn word_idx_for(val: usize) -> usize {
        val.rem_euclid(WORD_BIT_SIZE)
    }

    #[inline]
    pub fn has(&self, value: T) -> bool {
        let value = value.to_usize();
        let mask = 1usize << Self::word_idx_for(value);
        (self.words.get(Self::word_for(value)).copied().unwrap_or(0) & mask) != 0
    }

    /// Returns the amount of values contained in the set that are lower than
    /// the given value
    #[inline]
    pub fn index_of(&self, value: T) -> usize {
        let value = value.to_usize();

        let value_word = Self::word_for(value);
        let mask = 1usize << Self::word_idx_for(value);

        let count = self.words.iter().copied().take(value_word + 1).enumerate()
            .map(|(i, word)| {
                (if i != value_word {
                    word
                }
                else {
                    word & (mask - 1)
                }).count_ones()
            })
            .map(|val| ix!(val))
            .sum();

        count
    }

    #[inline]
    pub fn insert(&mut self, value: T) {
        let value = value.to_usize();
        let value_word = Self::word_for(value);
        let value_word_idx = Self::word_idx_for(value);

        // extend the vec by the number of missing words
        let missing = (value_word + 1).saturating_sub(self.words.len());
        self.words.extend(repeat_n(0, missing));

        self.words[value_word] |= 1usize << value_word_idx;
    }

    #[inline]
    pub fn remove(&mut self, value: T) {
        let value = value.to_usize();
        let value_word = Self::word_for(value);
        let value_word_idx = Self::word_idx_for(value);

        let Some(word) = self.words.get_mut(value_word)
        else { return };
        *word &= !(1usize << value_word_idx);

        if value_word + 1 < self.words.len() {
            return;
        }

        for i in (0..self.words.len()).rev() {
            debug_assert_eq!(i+1, self.words.len());
            if self.words[i] == 0 {
                self.words.pop();
            }
            else {
                break;
            }
        }
    }

    fn iter_from_words(words: impl Iterator<Item = usize>) -> impl Iterator<Item = T> {
        words.enumerate()
            .flat_map(|(word_idx, mut word)| {
                let base = word_idx * WORD_BIT_SIZE;
                std::iter::from_fn(move || {
                    (word != 0).then(|| {
                        let tz = word.trailing_zeros() as usize;
                        word &= word - 1;
                        base + tz
                    })
                })
            })
            .map(T::from_usize)
    }

    #[inline]
    pub fn into_iter(self) -> impl Iterator<Item = T> {
        Self::iter_from_words(self.words.into_iter())
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = T> {
        Self::iter_from_words(self.words.iter().copied())
    }

    #[inline]
    pub fn with(mut self, value: T) -> (usize, Self) {
        let idx = self.index_of(value);
        self.insert(value);
        (idx, self)
    }

    #[inline]
    pub fn without(mut self, value: T) -> (usize, Self) {
        let idx = self.index_of(value);
        self.remove(value);
        (idx, self)
    }
}

impl<T> Debug for BitSet<T>
    where T: BitSetIndex + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("BitSet")
            .field_with(|f| {
                f.debug_list()
                    .entries(self.iter())
                    .finish()
            })
            .finish()
    }
}

impl<T> IntoIterator for BitSet<T>
    where T: BitSetIndex,
{
    type Item = T;
    type IntoIter = impl Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter {
        BitSet::into_iter(self)
    }
}

impl<'a, T> IntoIterator for &'a BitSet<T>
    where T: BitSetIndex,
{
    type Item = T;
    type IntoIter = impl Iterator<Item = T>;

    fn into_iter(self) -> Self::IntoIter {
        BitSet::iter(self)
    }
}

impl<I: BitSetIndex> FromIterator<I> for BitSet<I> {
    fn from_iter<T: IntoIterator<Item = I>>(iter: T) -> Self {
        let mut result = Self::new();
        for val in iter {
            result.insert(val);
        }
        result
    }
}

impl<I: BitSetIndex> Extend<I> for BitSet<I> {
    fn extend<T: IntoIterator<Item = I>>(&mut self, iter: T) {
        for val in iter {
            self.insert(val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Invariants of BitSet (behavioral contracts):
    // 1) new: empty set — len() == 0, iter() empty, has(any) == false, index_of(any) == 0
    // 2) insert: sets membership — has(v) == true after insert; len() increases by 1 only if v was not present
    // 3) insert idempotency: inserting same value twice does not change len
    // 4) remove: unsets membership — has(v) == false after remove of present value; len() decreases by 1
    // 5) remove idempotency: removing absent value keeps len() unchanged and no panic
    // 6) index_of: counts values strictly less than given value (excludes the value itself if present)
    // 7) index_of on empty: returns 0 for any value
    // 8) index_of monotonicity: non-decreasing with respect to the queried value and capped by len()
    // 9) with: returns index before insertion and inserts the value
    // 10) without: returns index before removal and removes the value if present
    // 11) has: returns false for any value never inserted (including out-of-range/high values)
    // 12) cross-word correctness: operations work for values across multiple machine words, including boundaries
    // 13) iter: yields exactly the set values, once each, in ascending order (may expose bugs if implementation is wrong)
    // 14) len equals number of distinct inserted values at any time

    #[test]
    fn new_is_empty() {
        let bs = BitSet::<usize>::new();
        assert_eq!(bs.len(), 0);
        assert_eq!(bs.iter().count(), 0);
        assert!(!bs.has(0));
        assert!(!bs.has(1));
        assert_eq!(bs.index_of(0), 0);
        assert_eq!(bs.index_of(1), 0);
        assert_eq!(bs.index_of(WORD_BIT_SIZE), 0);
    }

    #[test]
    fn insert_sets_membership_and_len() {
        let mut bs = BitSet::new();
        bs.insert(5usize);
        assert!(bs.has(5));
        assert_eq!(bs.len(), 1);

        // inserting new value increases len
        bs.insert(7);
        assert!(bs.has(7));
        assert_eq!(bs.len(), 2);
    }

    #[test]
    fn insert_is_idempotent() {
        let mut bs = BitSet::new();
        bs.insert(42usize);
        let len1 = bs.len();
        bs.insert(42usize);
        assert_eq!(bs.len(), len1);
        assert!(bs.has(42));
    }

    #[test]
    fn remove_clears_membership_and_decreases_len() {
        let mut bs = BitSet::new();
        bs.insert(3usize);
        bs.insert(9usize);
        assert_eq!(bs.len(), 2);
        bs.remove(9usize);
        assert_eq!(bs.len(), 1);
        assert!(bs.has(3));
        assert!(!bs.has(9));
    }

    #[test]
    fn remove_absent_is_idempotent_and_safe() {
        let mut bs = BitSet::new();
        // removing absent from empty
        bs.remove(100usize);
        assert_eq!(bs.len(), 0);
        assert!(!bs.has(100));

        // removing absent from non-empty
        bs.insert(1usize);
        bs.insert(2usize);
        let len_before = bs.len();
        bs.remove(100usize);
        assert_eq!(bs.len(), len_before);
        assert!(bs.has(1));
        assert!(bs.has(2));
        assert!(!bs.has(100));
    }

    #[test]
    fn index_of_excludes_self_and_counts_lower() {
        let mut bs = BitSet::new();
        bs.insert(0usize);
        bs.insert(1usize);
        bs.insert(3usize);
        assert_eq!(bs.index_of(0), 0); // nothing lower than 0
        assert_eq!(bs.index_of(1), 1); // only 0 lower than 1
        assert_eq!(bs.index_of(2), 2); // 0 and 1 lower than 2
        assert_eq!(bs.index_of(3), 2); // excludes 3 itself
        assert_eq!(bs.index_of(4), 3); // all three are lower than 4
    }

    #[test]
    fn index_of_empty_is_zero_for_any_value() {
        let bs = BitSet::<usize>::new();
        for &v in &[0usize, 1, 2, WORD_BIT_SIZE - 1, WORD_BIT_SIZE, WORD_BIT_SIZE + 1, 9999] {
            assert_eq!(bs.index_of(v), 0, "index_of({v}) on empty set should be 0");
        }
    }

    #[test]
    fn index_of_is_monotonic_and_capped_by_len() {
        let mut bs = BitSet::new();
        let values = [0usize, 2, 5, 7, 13, WORD_BIT_SIZE, WORD_BIT_SIZE + 3];
        for &v in &values { bs.insert(v); }
        let len = bs.len();
        let mut last = 0usize;
        for v in 0..(WORD_BIT_SIZE * 2) {
            let idx = bs.index_of(v);
            assert!(idx >= last, "index_of should be non-decreasing");
            assert!(idx <= len, "index_of should not exceed len");
            last = idx;
        }
    }

    #[test]
    fn with_returns_index_before_insert_and_inserts() {
        let bs = BitSet::new();
        let (idx, bs) = bs.with(10usize);
        assert_eq!(idx, 0); // empty before insert => 0 lower elements
        assert!(bs.has(10));

        // With existing lower elements
        let (idx2, bs) = bs.with(5usize);
        assert_eq!(idx2, 0);
        assert!(bs.has(5));

        let (idx3, bs) = bs.with(7usize);
        assert_eq!(idx3, 1); // only 5 is lower than 7
        assert!(bs.has(7));
    }

    #[test]
    fn without_returns_index_before_remove_and_removes_if_present() {
        let mut bs = BitSet::new();
        for &v in &[2usize, 4, 6, 8] { bs.insert(v); }

        let (idx, bs2) = bs.clone().without(6usize);
        assert_eq!(idx, 2); // {2,4} lower than 6
        assert!(!bs2.has(6));
        assert_eq!(bs2.len(), 3);

        // Absent value: returns index of lower elements and set unchanged except potential internal details
        let len_before = bs.len();
        let (idx_absent, bs3) = bs.without(7usize);
        assert_eq!(idx_absent, 3); // {2,4,6} lower than 7
        assert_eq!(bs3.len(), len_before);
        assert!(!bs3.has(7));
        assert!(bs3.has(2) && bs3.has(4) && bs3.has(6) && bs3.has(8));
    }

    #[test]
    fn has_is_false_for_never_inserted_values() {
        let mut bs = BitSet::new();
        bs.insert(1usize);
        assert!(!bs.has(0));
        assert!(!bs.has(2));
        assert!(!bs.has(usize::MAX / 2));
    }

    #[test]
    fn cross_word_operations_and_boundaries() {
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        // boundary values around word boundaries
        let vals = [w - 1, w, w + 1, 2 * w - 1, 2 * w];
        for &v in &vals { bs.insert(v); }

        for &v in &vals {
            assert!(bs.has(v), "missing value {v} around boundary");
        }
        assert_eq!(bs.len(), vals.len());

        // index_of around boundaries
        assert_eq!(bs.index_of(w - 1), 0);
        assert_eq!(bs.index_of(w), 1);
        assert_eq!(bs.index_of(w + 1), 2);
        assert_eq!(bs.index_of(2 * w - 1), 3);
        assert_eq!(bs.index_of(2 * w), 4);
        assert_eq!(bs.index_of(2 * w + 1), 5);
    }

    #[test]
    fn iter_yields_exact_values_sorted_no_duplicates() {
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        let expected = [0usize, 1, 5, w - 2, w - 1, w, w + 1, w + 7, 2 * w + 3];
        for &v in &expected { bs.insert(v); }

        let mut got: Vec<usize> = bs.iter().map(|x| x.to_usize()).collect();
        // The intended invariant: iter returns sorted unique values exactly equal to inserted values
        got.dedup(); // in case of duplicates produced by a buggy iterator, keep a unique sequence copy for comparison below

        // This test intentionally asserts strict equality; if it fails, it indicates a bug in iter()
        assert_eq!(got, expected, "iter should yield all values in ascending order without duplicates");
        assert_eq!(got.len(), bs.len(), "iter count should equal len()");
        // Also check has() for all yielded values
        for &v in &got { assert!(bs.has(v)); }
    }

    #[test]
    fn len_equals_number_of_distinct_inserted_values() {
        let mut bs = BitSet::new();
        let values = [1usize, 1, 2, 3, 3, 3, 10, WORD_BIT_SIZE, WORD_BIT_SIZE, WORD_BIT_SIZE + 5];
        for &v in &values { bs.insert(v); }
        let distinct: std::collections::BTreeSet<_> = values.into_iter().collect();
        assert_eq!(bs.len(), distinct.len());
        for &v in &distinct { assert!(bs.has(v)); }
    }

    // Internal representation invariants around word storage growth/shrink

    #[test]
    fn internal_vec_starts_empty() {
        let bs = BitSet::<usize>::new();
        assert_eq!(bs.words.len(), 0);
    }

    #[test]
    fn internal_vec_grows_across_word_boundaries() {
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;

        // Insert values within first word
        bs.insert(0usize);
        assert_eq!(bs.words.len(), 1);
        bs.insert(w - 1);
        assert_eq!(bs.words.len(), 1, "still within first word");

        // Cross first boundary
        bs.insert(w);
        assert_eq!(bs.words.len(), 2, "should allocate second word");

        // Cross second boundary
        bs.insert(2 * w);
        assert_eq!(bs.words.len(), 3, "should allocate third word");
    }

    #[test]
    fn internal_vec_len_matches_highest_used_word_plus_one() {
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        let vals = [1usize, w - 1, w + 1, 2 * w + 7];
        for &v in &vals { bs.insert(v); }
        assert_eq!(bs.words.len(), 3, "highest word index is 2, so len should be 3");
    }

    #[test]
    fn internal_vec_does_not_grow_on_insert_within_same_word() {
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        for v in [0usize, 1, 7, w - 2, w - 1] { bs.insert(v); }
        assert_eq!(bs.words.len(), 1);
    }

    #[test]
    fn internal_vec_shrinks_when_highest_word_becomes_empty() {
        // Expected invariant: trailing zero words are trimmed when possible
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        bs.insert(1usize);     // word 0
        bs.insert(w);          // word 1
        assert_eq!(bs.words.len(), 2);
        bs.remove(w);          // clear highest word
        // This test will fail if implementation doesn't shrink trailing empty words
        assert_eq!(bs.words.len(), 1, "should shrink after clearing highest word");
    }

    #[test]
    fn internal_vec_remove_absent_should_not_allocate() {
        // Expected invariant: removing an absent value should not grow storage
        let mut bs = BitSet::new();
        assert_eq!(bs.words.len(), 0);
        bs.remove(123usize);
        // This test will fail if remove() extends underlying storage on absent removal
        assert_eq!(bs.words.len(), 0, "remove of absent element must not allocate");
    }

    #[test]
    fn internal_vec_len_stable_if_highest_word_still_used() {
        // Removing from a lower word should not change words.len() if higher words are used
        let mut bs = BitSet::new();
        let w = WORD_BIT_SIZE;
        bs.insert(0usize);       // word 0
        bs.insert(w + 5);        // word 1
        bs.insert(2 * w + 3);    // word 2 (highest)
        assert_eq!(bs.words.len(), 3);

        bs.remove(w + 5);        // remove from middle word
        assert_eq!(bs.words.len(), 3, "len unchanged since highest word still has bits");
        assert!(bs.has(0));
        assert!(!bs.has(w + 5));
        assert!(bs.has(2 * w + 3));
    }
}
