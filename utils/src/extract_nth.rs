use std::iter::FusedIterator;

pub trait ExtractNthExt: Sized + Iterator {
    fn extract_nth<F>(self, nth: usize, extractor: F) -> ExtractNth<Self, F>
          where F: FnOnce(Self::Item) -> ();
}

impl<I> ExtractNthExt for I
    where I: Iterator,
{
    fn extract_nth<F>(self, nth: usize, extractor: F) -> ExtractNth<Self, F>
          where F: FnOnce(Self::Item) -> () {
        ExtractNth {
            inner: self,
            n: nth,
            extractor: Some(extractor),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractNth<I, F>
    where I: Iterator,
          F: FnOnce(I::Item) -> (),
{
    inner: I,
    n: usize,
    extractor: Option<F>,
}

impl<I, F> Iterator for ExtractNth<I, F>
    where I: Iterator,
          F: FnOnce(I::Item) -> (),
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.n == 0 && let Some(extractor) = self.extractor.take() {
            match self.inner.next() {
                Some(i) => extractor(i),
                None => (),
            }
        }

        self.n = self.n.saturating_sub(1);
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min, max) = self.inner.size_hint();

        (
            {
                let to_remove = min.saturating_sub(self.n).min(1);
                min.saturating_sub(to_remove)
            }, max.map(|max| {
                let to_remove = max.saturating_sub(self.n).min(1);
                max.saturating_sub(to_remove)
            })
        )
    }
}

impl<I, F> ExactSizeIterator for ExtractNth<I, F>
    where I: Iterator,
          F: FnOnce(I::Item) -> (),
{ }

impl<I, F> FusedIterator for ExtractNth<I, F>
    where I: Iterator,
          F: FnOnce(I::Item) -> (),
{ }

#[cfg(test)]
mod tests {
    use std::iter::empty;

    use crate::extract_nth::ExtractNthExt;

    #[test]
    fn start() {
        let mut found = None;
        let collected = (1..=10)
            .extract_nth(0, |i| { assert_eq!(found, None); found = Some(i) })
            .collect::<Vec<_>>();

        assert_eq!(found, Some(1));
        assert_eq!(collected, vec![2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn middle() {
        let mut found = None;
        let collected = (1..=10)
            .extract_nth(5, |i| { assert_eq!(found, None); found = Some(i) })
            .collect::<Vec<_>>();

        assert_eq!(found, Some(6));
        assert_eq!(collected, vec![1, 2, 3, 4, 5, 7, 8, 9, 10]);
    }

    #[test]
    fn end() {
        let mut found = None;
        let collected = (1..=10)
            .extract_nth(9, |i| { assert_eq!(found, None); found = Some(i) })
            .collect::<Vec<_>>();

        assert_eq!(found, Some(10));
        assert_eq!(collected, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn after_end() {
        let mut found = None;
        let collected = (1..=10)
            .extract_nth(10, |i| { assert_eq!(found, None); found = Some(i) })
            .collect::<Vec<_>>();

        assert_eq!(found, None);
        assert_eq!(collected, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn empty_iter() {
        let mut found = None;
        let collected = empty::<i32>()
            .extract_nth(0, |i| { assert_eq!(found, None); found = Some(i) })
            .collect::<Vec<_>>();

        assert_eq!(found, None);
        assert_eq!(collected, vec![]);
    }
}
