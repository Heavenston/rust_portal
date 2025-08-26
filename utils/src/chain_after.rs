use std::iter::{Fuse, FusedIterator};

pub trait ChainAfterExt: Sized + Iterator {
    fn chain_after<I: Iterator<Item = Self::Item>>(self, after: usize, other: I) -> ChainAfter<Self, I>;
}

impl<I> ChainAfterExt for I
    where I: Iterator,
{
    fn chain_after<O: Iterator<Item = Self::Item>>(self, after: usize, other: O) -> ChainAfter<Self, O> {
        ChainAfter::<Self, O> {
            inner: self.fuse(),
            other: other.fuse(),
            after,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChainAfter<I: Iterator, O: Iterator> {
    inner: Fuse<I>,
    other: Fuse<O>,
    after: usize,
}

impl<I, O> Iterator for ChainAfter<I, O>
    where I: Iterator,
          O: Iterator<Item = I::Item>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.after == 0 {
            match self.other.next() {
                Some(element) => Some(element),
                // After emptying the other iterator we go back to the
                // source iterator
                None => self.inner.next(),
            }
        }
        else {
            self.after -= 1;
            match self.inner.next() {
                Some(element) => Some(element),
                // Even if the number of elements have not been reached
                // to not have any None gaps we directly switch to the other iterator
                None => self.other.next(),
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min1, max1) = self.inner.size_hint();
        let (min2, max2) = self.other.size_hint();
        return (usize::max(min1, min2), max1.zip_with(max2, usize::min));
    }
}

impl<I, O> ExactSizeIterator for ChainAfter<I, O>
    where I: Iterator                 + ExactSizeIterator,
          O: Iterator<Item = I::Item> + ExactSizeIterator,
{ }

impl<I, O> FusedIterator for ChainAfter<I, O>
    where I: Iterator,
          O: Iterator<Item = I::Item>,
{ }

#[cfg(test)]
mod tests {
    use std::iter::empty;

    use super::ChainAfterExt;

    #[test]
    fn at_0() {
        assert_eq!(
            (3..=5).chain_after(0, 1..=2).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn at_end() {
        assert_eq!(
            (1..=3).chain_after(3, 4..=5).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn after_end() {
        assert_eq!(
            (1..=3).chain_after(999, 4..=5).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn in_middle() {
        assert_eq!(
            (1..=5).chain_after(3, 10..=12).collect::<Vec<_>>(),
            vec![1, 2, 3, 10, 11, 12, 4, 5]
        );
    }

    #[test]
    fn in_empty() {
        assert_eq!(
            empty().chain_after(10, 10..=12).collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
    }

    #[test]
    fn with_empty_at_0() {
        assert_eq!(
            (1..=5).chain_after(0, empty()).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn with_empty_in_middle() {
        assert_eq!(
            (1..=5).chain_after(3, empty()).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn with_empty_at_end() {
        assert_eq!(
            (1..=5).chain_after(5, empty()).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn with_empty_after_end() {
        assert_eq!(
            (1..=5).chain_after(10, empty()).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5]
        );
    }
}
