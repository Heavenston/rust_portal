pub mod plus_one;

use crate::default;

use std::{ marker::PhantomData, num::NonZero };

/// Trait for hiding the unstable rust `nonzero_internals` feature
pub trait NonZeroPrimitive: std::num::ZeroablePrimitive { }
impl<T: std::num::ZeroablePrimitive> NonZeroPrimitive for T { }

pub trait NonZeroMapper<T: NonZeroPrimitive, O> {
    fn into_nonzero(&self, val: O) -> NonZero<T>;
    fn from_nonzero(&self, val: NonZero<T>) -> O;
}

#[derive_where::derive_where(PartialEq, Eq, Hash, Ord, PartialOrd; T)]
#[derive_where(Debug, Clone, Copy; T, M)]
pub struct MappedNonZero<T, O, M>
    where T: NonZeroPrimitive,
          M: NonZeroMapper<T, O>,
{
    non_zero: NonZero<T>,
    #[derive_where(skip)]
    _output: PhantomData<fn(O) -> O>,
    #[derive_where(skip)]
    mapper: M,
}

impl<T, O, M> MappedNonZero<T, O, M>
    where T: NonZeroPrimitive,
          M: NonZeroMapper<T, O>,
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
    where T: NonZeroPrimitive,
          M: NonZeroMapper<T, O> + Default,
          O: Default,
{
    #[inline]
    fn default() -> Self {
        Self::new_with_mapper(default(), default())
    }
}
