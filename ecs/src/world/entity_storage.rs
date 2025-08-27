use std::{ fmt::Display, iter::repeat_n };

use utils::{ itertools::chain, prelude::* };

pub type EntityGenerationType = u32;
pub type EntityIndexType = u32;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Entity {
    index: EntityIndexType,
    generation: EntityGenerationType,
}

impl Entity {
    pub fn new(index: EntityIndexType, generation: EntityGenerationType) -> Self {
        Self {
            index,
            generation,
        }
    }

    pub fn index(self) -> EntityIndexType {
        self.index
    }

    pub fn generation(self) -> EntityGenerationType {
        self.generation
    }
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity(0x{:016x})", u64::from(self.index()) | (u64::from(self.generation) << 32))
    }
}

#[derive(Default, Debug, Clone)]
pub struct EntityStorage {
    unused_head: Option<u32>,
    unused_count: u32,
    entities_generations: Vec<EntityGenerationType>,
    /// Linked list by their indices, the last value of the linked list points
    /// to itself, values never referenced by the linked list have undefined values
    entities_unused_next: Vec<u32>,

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
            // Sets reserved entities generations to max int to make the
            // `reserved_entities` iterator output not-yet alive entities
            entities_generations: vec![!0u32; ix!(reserved_internal_count)],
            entities_unused_next: vec![!0u32; ix!(reserved_internal_count)],

            reserved_internal_count,
            reserved_internal_counter: 0,
        }
    }

    /// Iterator over all of the remaining reserved entities, they are not yet valid
    /// as you need to call `take_next_reserved` for them to become valid.
    /// They are all guarenteed to be eventually outputed by take_next_reserved
    /// in the same order.
    pub fn reserved_entities(&self) -> impl Iterator<Item = Entity> {
        (self.reserved_internal_counter..self.reserved_internal_count)
            .map(|idx| {
                Entity {
                    index: idx,
                    generation: 0,
                }
            })
    }

    pub fn take_next_reserved(&mut self) -> Option<Entity> {
        if self.reserved_internal_counter >= self.reserved_internal_count {
            return None;
        }

        let index = self.reserved_internal_counter;
        self.reserved_internal_counter += 1;

        debug_assert_eq!(self.entities_generations[ix!(index)], !0u32);
        self.entities_generations[ix!(index)] = 0;
        debug_assert_eq!(self.entities_unused_next[ix!(index)], !0u32);

        Some(Entity {
            index,
            generation: 0,
        })
    }

    pub fn alive(&self, entity: Entity) -> bool {
        self.entities_generations.get(ix!(entity.index))
            .is_some_and(|&generation| generation == entity.generation)
    }

    pub fn spawn(&mut self) -> Entity {
        match self.unused_head {
            Some(unused_index) => {
                let generation = self.entities_generations[ix!(unused_index)];

                let unused_next = self.entities_unused_next[ix!(unused_index)];
                debug_assert_ne!(unused_next, !0u32);
                self.unused_head = (unused_next != unused_index)
                    .then_some(unused_next);
                self.unused_count -= 1;

                Entity {
                    index: unused_index,
                    generation,
                }
            },
            None => {
                let index = u32::try_from(self.entities_generations.len()).expect("Not too much entities");
                self.entities_unused_next.push(!0u32 /* this value doesn't matter really, using max u32 for easier debugging (this value should never be read) */);
                self.entities_generations.push(0);
                Entity {
                    index,
                    generation: 0,
                }
            },
        }
    }

    pub fn spawn_many(&mut self, amount: u32) -> impl Iterator<Item = Entity> {
        let use_unused = self.unused_count.min(amount);
        let remaining = amount - use_unused;

        let first_index = u32::try_from(self.entities_generations.len()).expect("Not too much entities");
        self.entities_unused_next.extend(
            repeat_n(!0u32 /* see `spawn` */, ix!(remaining))
        );
        self.entities_generations.extend(
            repeat_n(0, ix!(remaining))
        );

        chain(
            (0..use_unused)
                .map(move |_| self.spawn()),
            (0..remaining)
                .map(move |offset| { Entity { index: first_index + offset, generation: 0 } })
        )
    }

    pub fn dispawn(&mut self, entity: Entity) -> bool {
        let Some(generation) = self.entities_generations.get_mut(ix!(entity.index))
        else { return false };
        if *generation != entity.generation { return false; }
        *generation = generation.wrapping_add(1);

        self.entities_unused_next[ix!(entity.index)] = self.unused_head
            .unwrap_or(entity.index);
        self.unused_head = Some(entity.index);
        self.unused_count += 1;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::{ EntityStorage, Entity };

    #[test]
    fn spawn_produces_sequential_indices_and_alive() {
        let mut w = EntityStorage::default();
        let e0 = w.spawn();
        let e1 = w.spawn();
        let e2 = w.spawn();

        assert_eq!(e0.index, 0);
        assert_eq!(e1.index, 1);
        assert_eq!(e2.index, 2);

        assert_eq!(e0.generation, 0);
        assert_eq!(e1.generation, 0);
        assert_eq!(e2.generation, 0);

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
        assert!(!w.dispawn(Entity { index: 42, generation: 0 }));

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
        assert_eq!(n2.index, e2.index, "should reuse most recently despawned index");

        // Then reuse e1's index next
        let n1 = w.spawn();
        assert_eq!(n1.index, e1.index, "should reuse next despawned index");

        // e0 still alive and unchanged
        assert!(w.alive(e0));
        assert_eq!(e0.generation, 0);
    }

    #[test]
    fn generation_increments_by_one_per_reuse() {
        let mut w = EntityStorage::default();
        let e0 = w.spawn();
        assert!(w.dispawn(e0));

        let e1 = w.spawn();
        assert_eq!(e1.index, e0.index);
        assert_eq!(e1.generation, e0.generation + 1, "generation should increment by exactly 1");

        assert!(w.dispawn(e1));
        let e2 = w.spawn();
        assert_eq!(e2.index, e0.index);
        assert_eq!(e2.generation, e1.generation + 1, "generation should increment by exactly 1");
    }
}

