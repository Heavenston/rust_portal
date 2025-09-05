#[cfg(debug_assertions)]
use std::panic::Location;
use std::iter::FusedIterator;

fn is_lower_or_equal<T>(a: &T, b: &T) -> bool
    where T: Ord,
{
    a <= b
}

// could be a simple struct but i wanted to play with feature(type_alias_impl_trait)
pub type ByKeyClosure<'a, I: Iterator, F: 'a + FnMut(&I::Item) -> K, K: Ord> = impl 'a + FnMut(&I::Item, &I::Item) -> bool;

pub trait AssertIsSortedExt: Sized + Iterator {
    #[track_caller]
    fn assert_is_sorted_by<F>(self, comparator: F) -> AssertIsSortedBy<Self, F>
        where F: FnMut(&Self::Item, &Self::Item) -> bool {
        AssertIsSortedBy {
            #[cfg(debug_assertions)]
            called_at: Location::caller(),
            inner: self,
            next: None,
            comparator,
        }
    }

    #[cfg(debug_assertions)]
    #[track_caller]
    fn debug_assert_is_sorted_by<F>(self, comparator: F) -> AssertIsSortedBy<Self, F>
        where F: FnMut(&Self::Item, &Self::Item) -> bool {
        self.assert_is_sorted_by(comparator)
    }

    #[cfg(not(debug_assertions))]
    fn debug_assert_is_sorted_by<F>(self, comparator: F) -> Self
        where F: FnMut(&Self::Item, &Self::Item) -> bool {
        let _ = comparator;
        self
    }

    #[define_opaque(ByKeyClosure)]
    #[track_caller]
    fn assert_is_sorted_by_key<'a, F, K>(self, mut get_key: F) -> AssertIsSortedBy<Self, ByKeyClosure<'a, Self, F, K>>
        where F: 'a + FnMut(&Self::Item) -> K,
              K: Ord,
    {
        self.assert_is_sorted_by(move |a, b| get_key(a) <= get_key(b))
    }

    #[cfg(debug_assertions)]
    #[track_caller]
    fn debug_assert_is_sorted_by_key<'a, F, K>(self, get_key: F) -> AssertIsSortedBy<Self, ByKeyClosure<'a, Self, F, K>>
        where F: 'a + FnMut(&Self::Item) -> K,
              K: Ord,
    {
        self.assert_is_sorted_by_key(get_key)
    }

    #[cfg(not(debug_assertions))]
    fn debug_assert_is_sorted_by_key<'a, F, K>(self, get_key: F) -> Self
        where F: 'a + FnMut(&Self::Item) -> K,
              K: Ord,
    {
        let _ = get_key;
        self
    }

    #[track_caller]
    fn assert_is_sorted(self) -> AssertIsSortedBy<Self, fn(&Self::Item, &Self::Item) -> bool>
        where Self::Item: Ord,
    {
        self.assert_is_sorted_by(is_lower_or_equal::<Self::Item>)
    }

    #[cfg(debug_assertions)]
    #[track_caller]
    fn debug_assert_is_sorted(self) -> AssertIsSortedBy<Self, fn(&Self::Item, &Self::Item) -> bool>
        where Self::Item: Ord,
    {
        self.assert_is_sorted()
    }

    #[cfg(not(debug_assertions))]
    fn debug_assert_is_sorted(self) -> Self
        where Self::Item: Ord,
    {
        self
    }
}

impl<I> AssertIsSortedExt for I
    where I: Iterator,
{ }

#[derive(Debug, Clone)]
pub struct AssertIsSortedBy<I, G>
    where I: Iterator,
          G: FnMut(&I::Item, &I::Item) -> bool,
{
    #[cfg(debug_assertions)]
    called_at: &'static Location<'static>,
    inner: I,
    next: Option<I::Item>,
    comparator: G,
}

impl<I, G> Iterator for AssertIsSortedBy<I, G>
    where I: Iterator,
          G: FnMut(&I::Item, &I::Item) -> bool,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let previous = self.next.take().or_else(|| self.inner.next())?;
        if let Some(new) = self.inner.next() {
            #[cfg(debug_assertions)]
            assert!((self.comparator)(&previous, &new), "Iterator is not sorted at {}", self.called_at);
            #[cfg(not(debug_assertions))]
            assert!((self.comparator)(&previous, &new), "Iterator is not sorted");
            self.next = Some(new);
        }
        Some(previous)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, G> ExactSizeIterator for AssertIsSortedBy<I, G>
    where I: ExactSizeIterator,
          G: FnMut(&I::Item, &I::Item) -> bool,
{ }

impl<I, G> FusedIterator for AssertIsSortedBy<I, G>
    where I: FusedIterator,
          G: FnMut(&I::Item, &I::Item) -> bool,
{ }

#[cfg(test)]
mod tests {
    use std::iter::zip;

    use itertools::Itertools as _;
    use crate::assert_is_sorted::AssertIsSortedExt;

    #[test]
    fn simple() {
        assert_eq!(
            (0..=42).assert_is_sorted().collect_vec(),
            (0..=42).collect_vec(),
        );
    }

    #[test]
    fn simple_one_element() {
        assert_eq!(
            (0..1).assert_is_sorted().collect_vec(),
            (0..1).collect_vec(),
        );
    }

    #[test]
    fn simple_two_element() {
        assert_eq!(
            (0..2).assert_is_sorted().collect_vec(),
            (0..2).collect_vec(),
        );
    }

    #[test]
    #[should_panic(expected = "Iterator is not sorted")]
    fn simple_not_sorted() {
        [3, 8, 2].into_iter().assert_is_sorted().collect_vec();
    }

    #[test]
    fn sorted_by_reversed() {
        assert_eq!(
            (0..=42).rev().assert_is_sorted_by(|a, b| b < a).collect_vec(),
            (0..=42).rev().collect_vec(),
        );
    }

    #[test]
    fn by_key_tuple() {
        let iter = zip(
            (30..85).rev(),
            8..22,
        );
        assert_eq!(
            iter.clone().assert_is_sorted_by_key(|tuple| tuple.1).collect_vec(),
            iter.clone().collect_vec(),
        );
    }

    #[test]
    #[should_panic(expected = "Iterator is not sorted")]
    fn by_key_tuple_not_sorted() {
        let iter = zip(
            8..22,
            (30..85).rev(),
        );
        assert_eq!(
            iter.clone().assert_is_sorted_by_key(|tuple| tuple.1).collect_vec(),
            iter.clone().collect_vec(),
        );
    }
}
