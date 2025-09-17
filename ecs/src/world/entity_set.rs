use crate::world::Entity;

use utils::prelude::*;

// FIXME: Could (should) be a bit set (as it is mainly used as a ComponentSet
// which are all (supposedly) contiguous)
// But this doesn't really work with non zero generation :( so dont know whats
// best
/// Stores a 'set' of entities
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntitySet<P = Entity>
    where P: Ord + Into<Entity> + Default + Copy,
{
    entities: SortedSet<P>,
}

impl<P> EntitySet<P>
    where P: Ord + Into<Entity> + Default + Copy,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn has(&self, entity: P) -> bool {
        self.entities.contains(&entity)
    }

    pub fn index_of(&self, entity: P) -> Option<usize> {
        self.entities.binary_search(&entity).ok()
    }

    /// Returns the new element's index
    pub fn insert(&mut self, entity: P) -> usize {
        self.entities.find_or_insert(entity).index()
    }

    pub fn remove(&mut self, entity: P) -> Option<usize> {
        let idx = self.entities.binary_search(&entity).ok()?;
        self.entities.remove_index(idx);
        Some(idx)
    }

    /// Returns a *sorted* iterator over the contained entities
    pub fn iter(&self) -> impl Iterator<Item = P> + DoubleEndedIterator + ExactSizeIterator + Clone {
        self.entities.iter().copied()
    }

    pub fn with(mut self, entity: P) -> (usize, Self) {
        let idx = self.insert(entity);
        (idx, self)
    }

    pub fn without(mut self, entity: P) -> (Option<usize>, Self) {
        let idx = self.remove(entity);
        (idx, self)
    }
}

impl<P> Extend<P> for EntitySet<P>
    where P: Ord + Into<Entity> + Default + Copy,
{
    fn extend<T: IntoIterator<Item = P>>(&mut self, iter: T) {
        self.entities.extend(iter.into_iter());
    }
}

impl<P> From<&[P]> for EntitySet<P>
    where P: Ord + Into<Entity> + Default + Copy,
{
    fn from(value: &[P]) -> Self {
        Self {
            entities: SortedSet::from_unsorted(value.into()),
        }
    }
}

impl<P> FromIterator<P> for EntitySet<P>
    where P: Ord + Into<Entity> + Default + Copy,
{
    fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
        Self {
            entities: SortedSet::from_iter(iter),
        }
    }
}

pub type ComponentSet = EntitySet<super::ComponentEntity>;

#[cfg(test)]
mod tests {
    use super::EntitySet;
    use crate::world::{Entity, EntityIndex, EntityGeneration};

    fn e(i: u32, _g: u32) -> Entity {
        // Tests only need distinct indices; use FIRST generation
        Entity::new(EntityIndex(i), EntityGeneration::FIRST)
    }

    #[test]
    fn basic_insert_has_index_of_and_len() {
        let mut s = EntitySet::new();
        assert_eq!(s.len(), 0);
        let a = e(1, 0);
        let b = e(2, 0);
        let ia = s.insert(a);
        let ib = s.insert(b);
        assert!(s.has(a) && s.has(b));
        assert_eq!(s.index_of(a), Some(ia));
        assert_eq!(s.index_of(b), Some(ib));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn remove_and_iter_and_with_without() {
        let a = e(5, 0);
        let b = e(3, 0);
        let c = e(9, 0);
        let mut s = EntitySet::from(&[a, c, b][..]);
        // iter is sorted
        let v: Vec<_> = s.iter().collect();
        assert_eq!(v, vec![b, a, c]);

        // remove existing and non-existing
        assert!(s.remove(a).is_some());
        assert!(s.remove(a).is_none());

        // with/without helpers
        let (_idx, s2) = s.clone().with(a);
        assert!(s2.has(a));
        let (_idx2, s3) = s2.clone().without(a);
        assert!(!s3.has(a));
    }
}
