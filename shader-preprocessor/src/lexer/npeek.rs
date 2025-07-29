use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct NPeek<T>
    where T: Iterator,
{
    inner: T,
    buffer: VecDeque<T::Item>,
}

impl<T> NPeek<T>
    where T: Iterator,
{
    pub fn new(inner: T) -> Self {
        inner.into()
    }

    pub fn next_if(&mut self, pred: impl FnOnce(&T::Item) -> bool) -> Option<T::Item> {
        if pred(self.peek(0)?) {
            self.next()
        }
        else {
            None
        }
    }

    pub fn peek(&mut self, n: usize) -> Option<&T::Item> {
        while self.buffer.len() <= n {
            self.buffer.push_back(self.inner.next()?);
        }
        Some(&self.buffer[n])
    }
}

impl<T> Iterator for NPeek<T>
    where T: Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.buffer.pop_front()
            .or_else(|| self.inner.next())
    }
}

impl<T> From<T> for NPeek<T>
    where T: Iterator,
{
    fn from(inner: T) -> Self {
        Self {
            inner,
            buffer: Default::default(),
        }
    }
}

