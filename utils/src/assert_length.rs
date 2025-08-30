use std::iter::{Fuse, FusedIterator};

pub trait AssertLengthExt: Sized + Iterator {
    fn assert_length<F>(self, checker: F) -> AssertLength<Self, F>
          where F: FnOnce(usize);
}

impl<I> AssertLengthExt for I
    where I: Iterator,
{
    fn assert_length<F>(self, checker: F) -> AssertLength<Self, F>
          where F: FnOnce(usize),
    {
        AssertLength {
            inner: self.fuse(),
            n: 0,
            checker: Some(checker),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssertLength<I, F>
    where I: Iterator,
          F: FnOnce(usize),
{
    inner: Fuse<I>,
    n: usize,
    checker: Option<F>,
}

impl<I, F> Iterator for AssertLength<I, F>
    where I: Iterator,
          F: FnOnce(usize),
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.next() {
            Some(o) => {
                self.n += 1;
                Some(o)
            },
            None => {
                self.checker.take().expect("Not already taken")(self.n);
                None
            },
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, F> ExactSizeIterator for AssertLength<I, F>
    where I: ExactSizeIterator,
          F: FnOnce(usize),
{ }

impl<I, F> FusedIterator for AssertLength<I, F>
    where I: Iterator,
          F: FnOnce(usize),
{ }

#[cfg(test)]
mod tests {
    use std::iter::{ repeat };
    use super::AssertLengthExt;

    #[test]
    fn simple() {
        let iter = repeat(()).take(10).assert_length(|n| assert_eq!(n, 10));
        for _ in iter { }
    }

    #[test]
    fn empty() {
        let iter = repeat(()).take(0).assert_length(|n| assert_eq!(n, 0));
        for _ in iter { }
    }
}
