use super::*;
use crate::world::{
    Entity, EntityIndex, EntityGeneration,
    HasComponent, GetComponentError, AddComponentError
};
use std::any::TypeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestComponent1(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct TestComponent2(f32);

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct DefaultComponent(String);

#[derive(Clone, Debug, PartialEq, Eq)]
struct NoDefaultComponent(Vec<u32>);


#[cfg(test)]
mod basic_world_tests {
    use super::*;

    #[test]
    fn spawn_creates_alive_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        
        assert!(world.alive(entity));
        assert_eq!(entity.generation(), EntityGeneration::FIRST);
        
        // Should be in empty archetype
        let empty_archetype_id = world.components_set_to_archetyp.get(&ComponentSet::default())
            .expect("Empty archetype should exist");
        assert_eq!(world.entities_archetypes[entity.index()], *empty_archetype_id);
    }

    #[test]
    fn dispawn_makes_entity_dead() {
        let mut world = World::new();
        let entity = world.spawn();
        
        assert!(world.alive(entity));
        assert!(world.dispawn(entity));
        assert!(!world.alive(entity));
        
        // Double dispawn should return false
        assert!(!world.dispawn(entity));
    }

    #[test]
    fn generation_at_index() {
        let mut world = World::new();
        let entity = world.spawn();
        
        assert_eq!(
            world.generation_at_index(entity.index()),
            entity.generation()
        );
        
        world.dispawn(entity);
        
        // Generation should increment after dispawn
        assert_eq!(
            world.generation_at_index(entity.index()),
            entity.generation().next()
        );
    }

    #[test]
    fn spawn_component_creates_component_entity() {
        let mut world = World::new();
        let component_entity = world.spawn_component();
        
        assert!(world.alive(component_entity.0));
        assert!(u32::from(component_entity.index()) < RESERVED_ENTITY_COUNT);
        assert_eq!(component_entity.generation(), EntityGeneration::FIRST);
    }
}
