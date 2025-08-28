use std::marker::PhantomData;

use super::{ NonZeroPrimitive, NonZeroMapper, MappedNonZero, NonZero };

pub trait PlusOneMappable: NonZeroPrimitive {
    fn plus_one(self) -> NonZero<Self>;
    fn minus_one(this: NonZero<Self>) -> Self;
}

impl PlusOneMappable for usize {
    #[inline]
    fn plus_one(self) -> NonZero<usize> {
        self.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max usize given")
    }

    #[inline]
    fn minus_one(val: NonZero<usize>) -> usize {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

impl PlusOneMappable for u32 {
    #[inline]
    fn plus_one(self) -> NonZero<u32> {
        self.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max u32 given")
    }

    #[inline]
    fn minus_one(val: NonZero<u32>) -> u32 {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

impl PlusOneMappable for u64 {
    #[inline]
    fn plus_one(self) -> NonZero<u64> {
        self.checked_add(1)
            .and_then(|added| NonZero::new(added))
            .expect("Max u64 given")
    }

    #[inline]
    fn minus_one(val: NonZero<u64>) -> u64 {
        val.get().checked_sub(1).expect("impossible, non-zero")
    }
}

#[derive_where::derive_where(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlusOneMapper<T> {
    _phantom: PhantomData<T>,
}

impl<T> NonZeroMapper<T, T> for PlusOneMapper<T>
    where T: PlusOneMappable,
{
    fn into_nonzero(&self, val: T) -> NonZero<T> {
        T::plus_one(val)
    }

    fn from_nonzero(&self, val: NonZero<T>) -> T {
        T::minus_one(val)
    }
}

pub type PlusOneNonZero<T> = MappedNonZero<T, T, PlusOneMapper<T>>;
pub type PlusOneNonZeroUsize = PlusOneNonZero<usize>;
pub type PlusOneNonZeroU32 = PlusOneNonZero<u32>;
