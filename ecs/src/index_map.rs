use std::{ marker::PhantomData, ops::{ Index, IndexMut } };

use utils::prelude::*;

pub trait IndexMapIndex {
    fn from_usize(usize: usize) -> Self;
    fn to_usize(&self) -> usize;
}

impl IndexMapIndex for usize {
    fn from_usize(usize: usize) -> Self {
        usize
    }

    fn to_usize(&self) -> usize {
        *self
    }
}

impl IndexMapIndex for u32 {
    fn from_usize(usize: usize) -> Self {
        usize.try_into().expect("No overflow")
    }

    fn to_usize(&self) -> usize {
        ix!(*self)
    }
}

/// Convenience wrapper arount `Vec<T>` with a given index type
// FIXME: Should this go to utils?
#[derive_where::derive_where(Debug, Clone, PartialEq, Eq; T)]
#[derive_where(Default)]
pub struct IndexMap<T, I = usize> {
    vec: Vec<T>,
    _index: PhantomData<*const I>,
}

impl<T, I> IndexMap<T, I> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_slice(&self) -> &[T] {
        &self.vec
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.vec
    }

    pub fn as_vec(&self) -> &Vec<T> {
        &self.vec
    }

    pub fn as_mut_vec(&mut self) -> &mut Vec<T> {
        &mut self.vec
    }

    pub fn into_vec(self) -> Vec<T> {
        self.vec
    }
}

impl<T, I> IndexMap<T, I>
    where I: IndexMapIndex,
{
    pub fn push(&mut self, val: T) -> I {
        let index = I::from_usize(self.vec.len());
        self.vec.push(val);
        index
    }

    pub fn set_or_push(&mut self, idx: I, val: T) {
        let idx = idx.to_usize();
        if idx == self.vec.len() {
            self.vec.push(val);
        }
        else {
            self.vec[idx] = val;
        }
    }
}

impl<T, I> Index<I> for IndexMap<T, I>
    where I: IndexMapIndex,
{
    type Output = T;

    fn index(&self, index: I) -> &T {
        &self.vec[index.to_usize()]
    }
}

impl<T, I> IndexMut<I> for IndexMap<T, I>
    where I: IndexMapIndex,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.vec[index.to_usize()]
    }
}
