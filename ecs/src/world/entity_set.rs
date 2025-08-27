use crate::world::Entity;

use utils::prelude::*;

// FIXME: Could (should) be a bit set (as it is mainly used as a ComponentSet
// which are all (supposedly) contiguous)
// But this doesn't really work with non zero generation :( so dont know whats
// best
/// Stores a 'set' of entities
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntitySet {
    entities: SortedSet<Entity>,
}

impl EntitySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn has(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    pub fn index_of(&self, entity: Entity) -> Option<usize> {
        self.entities.binary_search(&entity).ok()
    }

    /// Returns the new element's index
    pub fn insert(&mut self, entity: Entity) -> usize {
        self.entities.find_or_insert(entity).index()
    }

    pub fn remove(&mut self, entity: Entity) -> Option<usize> {
        let idx = self.entities.binary_search(&entity).ok()?;
        self.entities.remove_index(idx);
        Some(idx)
    }

    /// Returns a *sorted* iterator over the contained entities
    pub fn iter(&self) -> impl Iterator<Item = Entity> + DoubleEndedIterator + ExactSizeIterator + Clone {
        self.entities.iter().copied()
    }

    pub fn with(mut self, entity: Entity) -> (usize, Self) {
        let idx = self.insert(entity);
        (idx, self)
    }

    pub fn without(mut self, entity: Entity) -> (Option<usize>, Self) {
        let idx = self.remove(entity);
        (idx, self)
    }
}

impl From<&[Entity]> for EntitySet {
    fn from(value: &[Entity]) -> Self {
        Self {
            entities: SortedSet::from_unsorted(value.into()),
        }
    }
}
