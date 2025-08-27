use std::iter::FusedIterator;

pub trait SkipAfterExt: Sized + Iterator {
    fn skip_after(self, after: usize, amount: usize) -> SkipAfter<Self>;
}

impl<I> SkipAfterExt for I
    where I: Iterator,
{
    fn skip_after(self, after: usize, amount: usize) -> SkipAfter<I> {
        SkipAfter {
            inner: self,
            after,
            amount,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SkipAfter<I>
    where I: Iterator,
{
    inner: I,
    after: usize,
    amount: usize,
}

impl<I> Iterator for SkipAfter<I>
    where I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.after == 0 && self.amount > 0 {
            self.inner.nth(std::mem::take(&mut self.amount))
        }
        else {
            self.after = self.after.saturating_sub(1);
            self.inner.next()
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.inner.size_hint();

        (
            {
                let to_remove = min.saturating_sub(self.after)
                    .min(self.amount);
                min.saturating_sub(to_remove)
            },
            max.map(|t| {
                let to_remove = t.saturating_sub(self.after)
                    .min(self.amount);
                t.saturating_sub(to_remove)
            })
        )
    }
}

impl<I> ExactSizeIterator for SkipAfter<I>
    where I: ExactSizeIterator,
{ }

impl<I> FusedIterator for SkipAfter<I>
    where I: FusedIterator,
{ }

#[cfg(test)]
mod tests {
    use super::SkipAfterExt;

    #[test]
    fn start() {
        assert_eq!(
            (1..=5).skip_after(0, 1).collect::<Vec<_>>(),
            vec![2, 3, 4, 5],
        );
    }

    #[test]
    fn middle() {
        assert_eq!(
            (1..=5).skip_after(2, 1).collect::<Vec<_>>(),
            vec![1, 2, 4, 5],
        );
    }

    #[test]
    fn end() {
        assert_eq!(
            (1..=5).skip_after(4, 1).collect::<Vec<_>>(),
            vec![1, 2, 3, 4],
        );
    }

    #[test]
    fn after_end() {
        assert_eq!(
            (1..=5).skip_after(5, 1).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5],
        );
    }

    #[test]
    fn skip_multiple() {
        assert_eq!(
            (1..=10).skip_after(3, 5).collect::<Vec<_>>(),
            vec![1, 2, 3, 9, 10],
        );
    }

    #[test]
    fn skip_none() {
        assert_eq!(
            (1..=10).skip_after(3, 0).collect::<Vec<_>>(),
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        );
    }

    #[test]
    fn exact_size_tests() {
        let my_vec = (1..=10).collect::<Vec<_>>();
        assert_eq!(
            my_vec.iter().skip_after(0, 0).len(),
            my_vec.len(),
        );
        assert_eq!(
            my_vec.iter().skip_after(0, 2).len(),
            my_vec.len() - 2,
        );
        assert_eq!(
            my_vec.iter().skip_after(8, 2).len(),
            my_vec.len() - 2,
        );
        assert_eq!(
            my_vec.iter().skip_after(11, 2).len(),
            my_vec.len(),
        );
        assert_eq!(
            my_vec.iter().skip_after(0, 10).len(),
            0,
        );
        assert_eq!(
            my_vec.iter().skip_after(5, 10).len(),
            5,
        );
    }
}
