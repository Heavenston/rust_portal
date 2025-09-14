use std::{ marker::PhantomData, ops::{ Index, IndexMut }, fmt::Debug };

use utils::prelude::*;

pub trait IndexMapIndex {
    fn from_usize(usize: usize) -> Self;
    fn to_usize(&self) -> usize;
}

impl<T: Clone, E1, E2> IndexMapIndex for T
    where usize: TryFrom<T, Error = E1>,
          T: TryFrom<usize, Error = E2>,
          E1: Debug, E2: Debug,
{
    fn from_usize(usize: usize) -> Self {
        usize.try_into().expect("No overflow")
    }

    fn to_usize(&self) -> usize {
        ix!(self.clone())
    }
}

/// Convenience wrapper arount `Vec<T>` with a given index type
// FIXME: Should this go to utils?
#[derive_where::derive_where(Debug, Clone, PartialEq, Eq; T)]
#[derive_where(Default)]
pub struct IndexMap<T, I = usize> {
    vec: Vec<T>,
    _index: PhantomData<fn(I) -> I>,
}

impl<T, I> IndexMap<T, I> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.vec
    }

    pub fn from_vec(vec: Vec<T>) -> Self {
        Self {
            vec,
            _index: PhantomData,
        }
    }

    pub fn as_slice(&self) -> &[T] {
        &self.vec
    }

    pub fn last(&self) -> Option<&T> {
        self.vec.last()
    }

    pub fn values(&self) -> impl Iterator<Item = &T> + ExactSizeIterator + DoubleEndedIterator + Clone {
        self.vec.iter()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> + ExactSizeIterator + DoubleEndedIterator {
        self.vec.iter_mut()
    }

    pub fn try_clone(&self) -> Result<Self, T::Error>
        where T: TryClone,
    {
        Ok(Self {
            vec: self.vec.iter().map(T::try_clone).try_collect()?,
            _index: PhantomData,
        })
    }
}

impl<T, I> IndexMap<T, I>
    where I: IndexMapIndex,
{
    pub fn len(&self) -> I {
        I::from_usize(self.vec.len())
    }

    pub fn push(&mut self, val: T) -> I {
        let index = self.len();
        self.vec.push(val);
        index
    }

    pub fn set_or_push(&mut self, idx: I, val: T) {
        let idx = idx.to_usize();
        if idx == self.vec.len() {
            self.vec.push(val);
        }
        else {
            self.vec[idx] = val;
        }
    }

    pub fn set(&mut self, idx: I, val: T) {
        self.vec[idx.to_usize()] = val;
    }

    pub fn extend_until(&mut self, target: I, with: impl Fn() -> T) {
        let target_length = target.to_usize() + 1;
        let missing = target_length.saturating_sub(self.vec.len());
        self.vec.resize_with(self.vec.len() + missing, with);
    }

    pub fn get(&self, idx: I) -> Option<&T> {
        self.vec.get(idx.to_usize())
    }

    pub fn get_mut(&mut self, idx: I) -> Option<&mut T> {
        self.vec.get_mut(idx.to_usize())
    }

    pub fn get_disjoint_mut<const N: usize>(
        &mut self, indices: [I; N]
    ) -> Result<[&mut T; N], std::slice::GetDisjointMutError> {
        self.vec.get_disjoint_mut(indices.map(|i| i.to_usize()))
    }

    pub fn swap_remove(&mut self, idx: I) -> T {
        self.vec.swap_remove(idx.to_usize())
    }

    pub fn indices(&self) -> impl Iterator<Item = I> + ExactSizeIterator + DoubleEndedIterator + Clone {
        (0..self.vec.len()).map(|i| I::from_usize(i))
    }

    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> + ExactSizeIterator + DoubleEndedIterator + Clone {
        self.indices().zip(self.values())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> + ExactSizeIterator + DoubleEndedIterator {
        self.vec.iter_mut().enumerate()
            .map(|(i, val)| (I::from_usize(i), val))
    }

    pub fn get_sorted_disjoint_mut<D>(&mut self, indices: impl Iterator<Item = (D, I)>) -> impl Iterator<Item = (D, I, &mut T)> {
        let mut offset: usize = 0;
        let mut current = self.vec.as_mut_slice();

        indices.map(move |(data, index)| {
            let i = index.to_usize() - offset;

            let (l, r) = std::mem::take(&mut current).split_at_mut(i + 1);
            current = r;
            offset += i + 1;

            (data, index, &mut l[i])
        })
    }
}

impl<T, I> Index<I> for IndexMap<T, I>
    where I: IndexMapIndex,
{
    type Output = T;

    fn index(&self, index: I) -> &T {
        &self.vec[index.to_usize()]
    }
}

impl<T, I> IndexMut<I> for IndexMap<T, I>
    where I: IndexMapIndex,
{
    fn index_mut(&mut self, index: I) -> &mut T {
        &mut self.vec[index.to_usize()]
    }
}

impl<T, I> IntoIterator for IndexMap<T, I> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.vec.into_iter()
    }
}

impl<T, I> FromIterator<T> for IndexMap<T, I> {
    fn from_iter<U: IntoIterator<Item = T>>(iter: U) -> Self {
        Self::from_vec(iter.into_iter().collect())
    }
}

#[macro_export]
macro_rules! indexmap {
    () => (
        $crate::index_map::IndexMap::new()
    );
    ($elem:expr; $n:expr) => (
        $crate::index_map::IndexMap::from_vec(vec![$elem; $n])
    );
    ($($x:expr),+ $(,)?) => (
        $crate::index_map::IndexMap::from_vec(vec![$($x,)*])
    );
}

#[cfg(test)]
mod tests {
    use super::IndexMap;

    #[test]
    fn set_or_push_and_get_behave_as_expected() {
        // Exercise new()
        let mut m: IndexMap<i32, usize> = IndexMap::new();
        // push returns next index implicitly; verify set_or_push grows
        m.set_or_push(0usize, 10);
        m.set_or_push(1usize, 20);
        // overwrite in place
        m.set_or_push(1usize, 21);
        assert_eq!(m.get(0usize), Some(&10));
        assert_eq!(m.get(1usize), Some(&21));
        // set updates existing
        m.set(0usize, 11);
        assert_eq!(m[0usize], 11);

        // values()/last()/into_vec cover convenience APIs
        let vals: Vec<_> = m.values().copied().collect();
        assert_eq!(vals, vec![11, 21]);
        assert_eq!(m.last(), Some(&21));
        let v = m.clone().into_vec();
        assert_eq!(v, vec![11, 21]);
    }

    #[test]
    fn extend_until_and_get_disjoint_mut_cover_paths() {
        let mut m: IndexMap<i32, usize> = IndexMap::default();
        // Ensure we can extend until a distant index
        m.extend_until(5usize, || 0);
        // Now set values at multiple positions
        m.set(2usize, 12);
        m.set(5usize, 55);
        // Get two disjoint mutable refs and mutate
        let [a, b] = m.get_disjoint_mut([2usize, 5usize]).expect("disjoint");
        *a += 1;
        *b += 1;
        assert_eq!(m[2usize], 13);
        assert_eq!(m[5usize], 56);
    }

    #[test]
    fn get_mut_returns_mutable_reference() {
        let mut m: IndexMap<i32, usize> = IndexMap::new();
        m.set_or_push(0usize, 1);
        if let Some(v) = m.get_mut(0usize) {
            *v = 2;
        }
        assert_eq!(m.get(0usize), Some(&2));
    }

    #[test]
    fn get_all_disjoint_mut() {
        let mut m: IndexMap<i32, i32> = IndexMap::new();
        m.extend_until(8, || -1);
        m.set_or_push(0, 0);
        m.set_or_push(3, 6);
        m.set_or_push(8, 16);
        m.set_or_push(2, 4);

        let p = m.get_sorted_disjoint_mut([0, 3, 8].into_iter().map(|v| ((), v)))
            .map(|((), k, v)| (k, *v)).collect::<Vec::<_>>();

        assert_eq!(
            p,
            vec![
                (0, 0),
                (3, 6),
                (8, 16),
            ]
        );
    }
}
