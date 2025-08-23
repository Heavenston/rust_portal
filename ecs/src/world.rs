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
        *generation += generation.wrapping_add(1);

        self.entities_unused_next[ix!(entity.index)] = self.unused_head
            .unwrap_or(entity.index);
        self.unused_head = Some(entity.index);

        true
    }
}

