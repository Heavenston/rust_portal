use crate::{ index_map::IndexMap, indexmap };

use utils::prelude::*;

mod entity {
    use std::fmt::Display;
    use derive_more::{From, Into};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
    pub struct EntityGeneration(u32);

    impl EntityGeneration {
        pub const MAX: Self = Self(u32::MAX);
        pub const FIRST: Self = Self(u32::MIN);

        /// Wraps arount after MAX
        pub fn next(self) -> Self {
            Self(self.0.wrapping_add(1))
        }

        pub fn nth(self, nth: u32) -> Self {
            Self(self.0.wrapping_add(nth))
        }
    }

    impl Default for EntityGeneration {
        fn default() -> Self {
            Self::FIRST
        }
    }

    impl From<EntityGeneration> for u64 {
        fn from(value: EntityGeneration) -> Self {
            u64::from(u32::from(value))
        }
    }

    impl TryFrom<EntityGeneration> for usize {
        type Error = <u32 as TryInto<usize>>::Error;

        fn try_from(value: EntityGeneration) -> Result<Self, Self::Error> {
            usize::try_from(value.0)
        }
    }

    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into, From)]
    pub struct EntityIndex(pub u32);

    impl From<EntityIndex> for u64 {
        fn from(value: EntityIndex) -> Self {
            u64::from(u32::from(value))
        }
    }

    impl TryFrom<u64> for EntityIndex {
        type Error = <u32 as TryFrom<u64>>::Error;

        fn try_from(value: u64) -> Result<Self, Self::Error> {
            Ok(Self(u32::try_from(value)?))
        }
    }

    impl TryFrom<EntityIndex> for usize {
        type Error = <u32 as TryInto<usize>>::Error;

        fn try_from(value: EntityIndex) -> Result<Self, Self::Error> {
            usize::try_from(value.0)
        }
    }

    impl TryFrom<usize> for EntityIndex {
        type Error = <usize as TryFrom<u32>>::Error;

        fn try_from(value: usize) -> Result<Self, Self::Error> {
            Ok(Self(u32::try_from(value)?))
        }
    }

    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Entity {
        index: EntityIndex,
        generation: EntityGeneration,
    }

    impl Entity {
        pub fn new(index: EntityIndex, generation: EntityGeneration) -> Self {
            Self {
                index,
                generation,
            }
        }

        pub fn index(self) -> EntityIndex {
            self.index
        }

        pub fn generation(self) -> EntityGeneration {
            self.generation
        }
    }

    impl Display for Entity {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "Entity(0x{:08x},0x{:08x})", u32::from(self.index()), u32::from(self.generation))
        }
    }

    impl<'a> Into<Entity> for &'a Entity {
        fn into(self) -> Entity {
            *self
        }
    }

    /// This is used and expected internally by the `EntityStorage` so not
    /// changeable easily.
    #[test]
    fn generation_wraps_around() {
        assert_eq!(EntityGeneration::MAX.next(), EntityGeneration::FIRST);
    }
}
pub use entity::*;

/// this value doesn't matter really, using max u32 for easier debugging (this value should never be read)
const UNUSED_NEXT_SENTINEL: EntityIndex = EntityIndex(u32::MAX);

#[derive(Default, Debug, Clone)]
pub struct EntityStorage {
    unused_head: Option<EntityIndex>,
    unused_count: u32,
    entities_generations: IndexMap<EntityGeneration, EntityIndex>,
    /// Linked list by their indices, the last value of the linked list points
    /// to itself, values never referenced by the linked list have undefined values
    entities_unused_next: IndexMap<EntityIndex, EntityIndex>,

    /// This means that the first `n` entities are reserved at the creation
    /// and are not added to the free list
    reserved_internal_count: u32,
    /// Index of the next unused entity index from the reserved internal entities
    reserved_internal_counter: u32,
}

impl EntityStorage {
    pub fn new(reserved_internal_count: u32) -> Self {
        Self {
            unused_head: default(),
            unused_count: default(),
            entities_generations: indexmap![EntityGeneration::MAX; ix!(reserved_internal_count)],
            entities_unused_next: indexmap![UNUSED_NEXT_SENTINEL; ix!(reserved_internal_count)],

            reserved_internal_count,
            reserved_internal_counter: 0,
        }
    }

    pub fn generation_at_index(&self, index: EntityIndex) -> EntityGeneration {
        self.entities_generations.get(index)
            .copied().unwrap_or(EntityGeneration::FIRST)
    }

    /// Iterator over all of the remaining reserved entities, they are not yet valid
    /// as you need to call `take_next_reserved` for them to become valid.
    /// They are all guarenteed to be eventually outputed by take_next_reserved
    /// in the same order.
    pub fn reserved_entities(&self) -> impl Iterator<Item = Entity> {
        (self.reserved_internal_counter..self.reserved_internal_count)
            .map(EntityIndex)
            .map(|idx| {
                Entity::new(
                    idx,
                    // We now the generations of reserved entities start at MAX
                    // so this entity is not yet 'alive' until `take_nex_reserved`
                    EntityGeneration::FIRST,
                )
            })
    }

    pub fn take_next_reserved(&mut self) -> Option<Entity> {
        if self.reserved_internal_counter >= self.reserved_internal_count {
            return None;
        }

        let index = EntityIndex(self.reserved_internal_counter);
        self.reserved_internal_counter += 1;

        // Before being used reserved entities should have this generation
        debug_assert_eq!(self.entities_generations[index], EntityGeneration::MAX);
        self.entities_generations[index] = EntityGeneration::FIRST;

        Some(Entity::new(index, EntityGeneration::FIRST))
    }

    pub fn alive(&self, entity: Entity) -> bool {
        self.entities_generations.get(entity.index())
            .is_some_and(|&generation| generation == entity.generation())
    }

    pub fn spawn(&mut self) -> Entity {
        match self.unused_head {
            Some(unused_index) => {
                let generation = self.entities_generations[unused_index];

                let unused_next = self.entities_unused_next[unused_index];
                debug_assert_ne!(unused_next, UNUSED_NEXT_SENTINEL);
                self.unused_head = (unused_next != unused_index)
                    .then_some(unused_next);
                self.unused_count -= 1;

                Entity::new(unused_index, generation)
            },
            None => {
                let index = u32::try_from(self.entities_generations.len()).expect("Not too much entities");
                self.entities_unused_next.push(UNUSED_NEXT_SENTINEL);
                self.entities_generations.push(EntityGeneration::FIRST);
                Entity::new(
                    EntityIndex(index),
                    EntityGeneration::FIRST,
                )
            },
        }
    }

    pub fn dispawn(&mut self, entity: Entity) -> bool {
        let Some(generation) = self.entities_generations.get_mut(entity.index())
        else { return false };
        if *generation != entity.generation() { return false; }
        *generation = generation.next();

        let i = entity.index().into();
        self.entities_unused_next[entity.index()] = self.unused_head.unwrap_or(i);
        self.unused_head = Some(i);
        self.unused_count += 1;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::{ EntityStorage, Entity, EntityIndex, EntityGeneration };

    impl PartialEq<u32> for EntityIndex {
        fn eq(&self, &other: &u32) -> bool {
            u32::from(*self) == other
        }
    }

    impl PartialEq<u32> for EntityGeneration {
        fn eq(&self, &other: &u32) -> bool {
            u32::from(*self) == other
        }
    }

    #[test]
    fn spawn_produces_sequential_indices_and_alive() {
        let mut w = EntityStorage::default();
        let e0 = w.spawn();
        let e1 = w.spawn();
        let e2 = w.spawn();

        assert_eq!(e0.index(), 0);
        assert_eq!(e1.index(), 1);
        assert_eq!(e2.index(), 2);

        assert_eq!(e0.generation(), 0);
        assert_eq!(e1.generation(), 0);
        assert_eq!(e2.generation(), 0);

        assert!(w.alive(e0));
        assert!(w.alive(e1));
        assert!(w.alive(e2));
    }

    #[test]
    fn dispawn_makes_entity_dead() {
        let mut w = EntityStorage::default();
        let e = w.spawn();
        assert!(w.alive(e));

        assert!(w.dispawn(e));
        assert!(!w.alive(e));

        // Double-despawn should return false
        assert!(!w.dispawn(e));
    }

    #[test]
    fn dispawn_of_invalid_or_stale_entity_returns_false() {
        let mut w = EntityStorage::default();
        // Invalid index
        assert!(!w.dispawn(Entity::new(EntityIndex(42), EntityGeneration::FIRST)));

        // Stale entity after despawn
        let e = w.spawn();
        assert!(w.dispawn(e));
        // Old handle should now be stale
        assert!(!w.dispawn(e));
    }

    // The next two tests encode the intended behavior for reuse and generation
    // increment. They currently expose bugs in the implementation.

    #[test]
    fn spawn_reuses_despawned_index_lifo() {
        let mut w = EntityStorage::default();
        let e0 = w.spawn();
        let e1 = w.spawn();
        let e2 = w.spawn();

        // Despawn two; free list should be LIFO: e2, then e1
        assert!(w.dispawn(e1));
        assert!(w.dispawn(e2));

        // Expect to reuse e2's index first
        let n2 = w.spawn();
        assert_eq!(n2.index(), e2.index(), "should reuse most recently despawned index");

        // Then reuse e1's index next
        let n1 = w.spawn();
        assert_eq!(n1.index(), e1.index(), "should reuse next despawned index");

        // e0 still alive and unchanged
        assert!(w.alive(e0));
        assert_eq!(e0.generation(), 0);
    }

    #[test]
    fn generation_increments_by_one_per_reuse() {
        let mut w = EntityStorage::default();
        let e0 = w.spawn();
        assert!(w.dispawn(e0));

        let e1 = w.spawn();
        assert_eq!(e1.index(), e0.index());
        assert_eq!(e1.generation(), e0.generation().next(), "generation should increment by exactly 1");

        assert!(w.dispawn(e1));
        let e2 = w.spawn();
        assert_eq!(e2.index(), e0.index());
        assert_eq!(e2.generation(), e1.generation().next(), "generation should increment by exactly 1");
    }

    #[test]
    fn entity_generation_nth_and_conversions() {
        // nth should add wrapping amount, and conversions should succeed
        let g = EntityGeneration::FIRST.nth(5);
        assert_eq!(u64::from(g), u64::from(5u32));
        let usize_val: usize = usize::try_from(g).unwrap();
        assert_eq!(usize_val, 5usize);

        let idx = EntityIndex::try_from(42usize).unwrap();
        assert_eq!(u64::from(idx), u64::from(42u32));
        let idx2 = EntityIndex::try_from(100u64).unwrap();
        let usize_from_idx: usize = usize::try_from(idx2).unwrap();
        assert_eq!(usize_from_idx, 100usize);
    }

    #[test]
    fn take_next_reserved_none_when_exhausted() {
        // With zero reserved, take_next_reserved should immediately return None
        let mut w = EntityStorage::default();
        assert!(w.take_next_reserved().is_none());
        // Also validate the reserved_entities iterator is empty in this case
        assert_eq!(w.reserved_entities().count(), 0);
    }
}
