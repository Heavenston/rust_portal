use std::{ marker::PhantomData, num::{ NonZero, ZeroablePrimitive } };

use crate::default;

pub trait NonZeroMapper<T: ZeroablePrimitive, O> {
    fn into_nonzero(&self, val: O) -> NonZero<T>;
    fn from_nonzero(&self, val: NonZero<T>) -> O;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlusOneMapper<T> {
    _phantom: PhantomData<T>,
}

impl NonZeroMapper<usize, usize> for PlusOneMapper<usize> {
    #[inline]
    fn into_nonzero(&self, val: usize) -> NonZero<usize> {
        val.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max usize given")
    }

    #[inline]
    fn from_nonzero(&self, val: NonZero<usize>) -> usize {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

impl NonZeroMapper<u32, u32> for PlusOneMapper<u32> {
    #[inline]
    fn into_nonzero(&self, val: u32) -> NonZero<u32> {
        val.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max u32 given")
    }

    #[inline]
    fn from_nonzero(&self, val: NonZero<u32>) -> u32 {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

impl NonZeroMapper<u64, u64> for PlusOneMapper<u64> {
    #[inline]
    fn into_nonzero(&self, val: u64) -> NonZero<u64> {
        val.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max u64 given")
    }

    #[inline]
    fn from_nonzero(&self, val: NonZero<u64>) -> u64 {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

#[derive_where::derive_where(PartialEq, Eq, Hash, Ord, PartialOrd; T)]
#[derive_where(Debug, Clone, Copy; T, M)]
pub struct MappedNonZero<T: ZeroablePrimitive, O, M> {
    non_zero: NonZero<T>,
    #[derive_where(skip)]
    _output: PhantomData<*const O>,
    #[derive_where(skip)]
    mapper: M,
}

impl<T: ZeroablePrimitive, O, M> MappedNonZero<T, O, M>
    where M: NonZeroMapper<T, O>,
{
    pub fn new_with_mapper(value: O, mapper: M) -> Self {
        Self {
            non_zero: mapper.into_nonzero(value),
            _output: PhantomData,
            mapper,
        }
    }

    #[inline]
    pub fn new(value: O) -> Self
        where M: Default,
    {
        Self::new_with_mapper(value, default())
    }

    #[inline]
    pub fn get_raw(self) -> NonZero<T> {
        self.non_zero
    }

    #[inline]
    pub fn get(self) -> O {
        self.mapper.from_nonzero(self.non_zero)
    }

    #[inline]
    pub fn set(&mut self, new_val: O) {
        self.non_zero = self.mapper.into_nonzero(new_val)
    }
}

impl<T, O, M> Default for MappedNonZero<T, O, M>
    where T: ZeroablePrimitive,
          M: NonZeroMapper<T, O> + Default,
          O: Default,
{
    #[inline]
    fn default() -> Self {
        Self::new_with_mapper(default(), default())
    }
}

pub type PlusOneNonZero<T> = MappedNonZero<T, T, PlusOneMapper<T>>;
pub type PlusOneNonZeroUsize = PlusOneNonZero<usize>;
pub type PlusOneNonZeroU32 = PlusOneNonZero<u32>;
