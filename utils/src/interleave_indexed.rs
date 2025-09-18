use std::iter::Peekable;

pub trait InterleaveIndexedExt: Sized + Iterator {
    fn interleave_indexed<O: Iterator<Item = (usize, Self::Item)>>(self, other: O) -> InterleaveIndexed<Self, O> {
        InterleaveIndexed::<Self, O> {
            inner: self,
            other: other.peekable(),
            current: 0,
        }
    }
}

impl<I> InterleaveIndexedExt for I
    where I: Iterator,
{ }

#[derive_where::derive_where(Debug; I: std::fmt::Debug, Peekable<O>: std::fmt::Debug)]
pub struct InterleaveIndexed<I, O>
    where I: Iterator,
          O: Iterator<Item = (usize, I::Item)>,
{
    inner: I,
    other: Peekable<O>,
    current: usize,
}

impl<I, O> Iterator for InterleaveIndexed<I, O>
    where I: Iterator,
          O: Iterator<Item = (usize, I::Item)>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let value = match self.other.next_if(|&(idx, _)| idx == self.current) {
            Some((_, value)) => Some(value),
            None => self.inner.next(),
        }?;
        self.current += 1;
        Some(value)
    }

    // fn size_hint(&self) -> (usize, Option<usize>) {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn empty_both() {
        assert_eq!(
            empty::<u32>()
                .interleave_indexed(empty())
                .collect_vec(),
            vec![],
        );
    }

    #[test]
    fn empty_other() {
        assert_eq!(
            [3u32, 2, 1].into_iter()
                .interleave_indexed(empty())
                .collect_vec(),
            vec![3u32, 2, 1],
        );
    }

    #[test]
    fn into_empty() {
        assert_eq!(
            empty::<u32>()
                .interleave_indexed([
                    (0, 3u32),
                    (1, 2u32),
                    (2, 1u32),
                ].into_iter())
                .collect_vec(),
            vec![3u32, 2, 1],
        );
    }

    #[test]
    fn multiple_from_start() {
        assert_eq!(
            [2u32, 1u32].into_iter()
                .interleave_indexed([
                    (0, 5u32),
                    (1, 4u32),
                    (2, 3u32),
                ].into_iter())
                .collect_vec(),
            vec![5, 4, 3, 2, 1],
        );
    }

    #[test]
    fn multiple_at_end() {
        assert_eq!(
            [5u32, 4u32].into_iter()
                .interleave_indexed([
                    (2, 3u32),
                    (3, 2u32),
                    (4, 1u32),
                ].into_iter())
                .collect_vec(),
            vec![5, 4, 3, 2, 1],
        );
    }

    #[test]
    fn multiple_in_middle() {
        assert_eq!(
            [5u32, 1u32].into_iter()
                .interleave_indexed([
                    (1, 4u32),
                    (2, 3u32),
                    (3, 2u32),
                ].into_iter())
                .collect_vec(),
            vec![5, 4, 3, 2, 1],
        );
    }

    #[test]
    fn begin_and_middle_and_end() {
        assert_eq!(
            [4u32, 2u32].into_iter()
                .interleave_indexed([
                    (0, 5u32),
                    (2, 3u32),
                    (4, 1u32),
                ].into_iter())
                .collect_vec(),
            vec![5, 4, 3, 2, 1],
        );
    }

    #[test]
    fn complete() {
        assert_eq!(
            [12, 11, 10, 6, 5, 4].into_iter()
                .interleave_indexed([
                    (0, 15), (1, 14), (2, 13),
                    // (3, 12), (4, 11), (5, 10),
                    (6, 9), (7, 8), (8, 7),
                    // (9, 6), (10, 5), (11, 4),
                    (12, 3), (13, 2), (14, 1),
                ].into_iter())
                .collect_vec(),
            vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1],
        );
    }

    // This is a weird case, maybe we should panic
    #[test]
    fn bonus_after_end() {
        assert_eq!(
            [4, 2, 1].into_iter()
                .interleave_indexed([
                    (1, 3),
                    (5, 8),
                ].into_iter())
                .collect_vec(),
            vec![4, 3, 2, 1],
        );
    }
}

