use std::iter::FusedIterator;

use super::*;

#[derive(Debug)]
pub struct TupleNth<const N: usize, I>
    where I: Iterator,
          I::Item: TupleAt<N>,
{
    inner: I,
}

impl<const N: usize, I> Iterator for TupleNth<N, I>
    where I: Iterator,
          I::Item: TupleAt<N>,
{
    type Item = <I::Item as TupleAt<N>>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(TupleAt::<N>::tuple_into_at)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<const N: usize, I> ExactSizeIterator for TupleNth<N, I>
    where I: ExactSizeIterator,
          I::Item: TupleAt<N>,
{ }

impl<const N: usize, I> FusedIterator for TupleNth<N, I>
    where I: FusedIterator,
          I::Item: TupleAt<N>,
{ }

impl<const N: usize, I> From<I> for TupleNth<N, I>
    where I: Iterator,
          I::Item: TupleAt<N>,
{
    fn from(inner: I) -> Self {
        Self { inner }
    }
}
