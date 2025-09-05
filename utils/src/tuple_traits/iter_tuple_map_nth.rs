use std::iter::FusedIterator;

use super::*;

pub struct TupleMapNth<const N: usize, I, M, O>
    where I: Iterator,
          I::Item: TupleAt<N>,
          M: FnMut(<I::Item as TupleAt<N>>::Item) -> O,
{
    pub(super) inner: I,
    pub(super) mapper: M,
}

impl<const N: usize, I, M, O> Iterator for TupleMapNth<N, I, M, O>
    where I: Iterator,
          I::Item: TupleAt<N>,
          M: FnMut(<I::Item as TupleAt<N>>::Item) -> O,
{
    type Item = <I::Item as TupleAt<N>>::Mapped<O>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.inner.next()?.tuple_map_at(&mut self.mapper))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<const N: usize, I, M, O> ExactSizeIterator for TupleMapNth<N, I, M, O>
    where I: ExactSizeIterator,
          I::Item: TupleAt<N>,
          M: FnMut(<I::Item as TupleAt<N>>::Item) -> O,
{ }

impl<const N: usize, I, M, O> FusedIterator for TupleMapNth<N, I, M, O>
    where I: FusedIterator,
          I::Item: TupleAt<N>,
          M: FnMut(<I::Item as TupleAt<N>>::Item) -> O,
{ }
