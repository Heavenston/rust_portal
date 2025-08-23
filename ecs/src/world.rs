use utils::prelude::*;

type GenerationType = u32;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: GenerationType,
}

impl Entity {
    
}

#[derive(Default, Debug, Clone)]
pub struct World {
    unused_head: Option<u32>,
    unused_count: u32,
    entities_generations: Vec<GenerationType>,
    /// Linked list by their indices, the last value of the linked list points
    /// to itself, values never referenced by the linked list have undefined values
    entities_unused_next: Vec<u32>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
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

    /// Returns false if the entity wasn't alive already
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
    use super::{World, Entity};

    #[test]
    fn spawn_produces_sequential_indices_and_alive() {
        let mut w = World::new();
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
        let mut w = World::new();
        let e = w.spawn();
        assert!(w.alive(e));

        assert!(w.dispawn(e));
        assert!(!w.alive(e));

        // Double-despawn should return false
        assert!(!w.dispawn(e));
    }

    #[test]
    fn dispawn_of_invalid_or_stale_entity_returns_false() {
        let mut w = World::new();
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
        let mut w = World::new();
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
        let mut w = World::new();
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
