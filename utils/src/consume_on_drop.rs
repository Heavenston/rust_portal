use std::iter::FusedIterator;

pub trait ConsumeOnDropExt: Iterator {
    fn consume_on_drop(self) -> ConsumeOnDrop<Self>;
}

impl<I> ConsumeOnDropExt for I
    where I: Iterator
{
    fn consume_on_drop(self) -> ConsumeOnDrop<Self> {
        ConsumeOnDrop { inner: self }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumeOnDrop<I: Iterator + ?Sized> {
    inner: I,
}

impl<I> Iterator for ConsumeOnDrop<I>
    where I: Iterator
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I> DoubleEndedIterator for ConsumeOnDrop<I>
    where I: DoubleEndedIterator,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<I> ExactSizeIterator for ConsumeOnDrop<I>
    where I: ExactSizeIterator
{ }

impl<I> FusedIterator for ConsumeOnDrop<I>
    where I: FusedIterator
{ }

impl<I: ?Sized> Drop for ConsumeOnDrop<I>
    where I: Iterator,
{
    fn drop(&mut self) {
        for _ in &mut self.inner { }
    }
}
