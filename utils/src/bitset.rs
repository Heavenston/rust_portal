use std::{ iter::empty, marker::PhantomData, fmt::Debug };

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
    pub fn new() -> Self {
        Self::default()
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

    /// Returns `true` if the value was in the set, `false` otherwise
    #[inline]
    pub fn remove(&mut self, value: T) {
        let value = value.to_usize();
        let value_word = Self::word_for(value);
        let value_word_idx = Self::word_idx_for(value);

        // extend the vec by the number of missing words
        let missing = (value_word + 1).saturating_sub(self.words.len());
        self.words.extend(repeat_n(0, missing));

        self.words[value_word] &= !(1usize << value_word_idx);
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = T> {
        self.words.iter().copied().enumerate()
            .flat_map(|(word_idx, mut word)| {
                let mut current_sub_idx = 0usize;
                std::iter::from_fn(move || {
                    let tz = word.trailing_zeros() as usize;
                    current_sub_idx += tz;
                    word <<= tz;
                    (word != 0).then_some(word_idx * WORD_BIT_SIZE + current_sub_idx)
                })
            })
            .map(T::from_usize)
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
