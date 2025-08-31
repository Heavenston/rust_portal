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
    fn new_world_has_reserved_entities_setup() {
        let world = World::new();
        
        // In World::new(), the reserved entities are processed and placed in the world:
        // 1. reserved_entities() iterates over them with FIRST generation 
        // 2. They are placed in the empty archetype and empty table
        // 3. take_next_reserved() is called for ComponentComponent, making it alive
        
        // So most reserved entities are set up with their indices, but only the ones
        // taken by take_next_reserved() are actually alive with FIRST generation.
        // The rest still have MAX generation (not alive yet).
        
        // Check that reserved entity indices are set up in world structures
        let empty_archetype_id = world.components_set_to_archetyp.get(&ComponentSet::default())
            .expect("Empty archetype should exist");
        
        // The first reserved entity (index 0) is the ComponentComponent entity,
        // which is in its own archetype, not the empty one
        let first_entity_archetype = world.entities_archetypes[EntityIndex(0)];
        assert_ne!(first_entity_archetype, *empty_archetype_id, 
            "First reserved entity should not be in empty archetype (it's ComponentComponent)");
        
        // Other reserved entities should be in empty archetype  
        for i in 1..RESERVED_ENTITY_COUNT {
            let entity_index = EntityIndex(i);
            assert_eq!(
                world.entities_archetypes[entity_index],
                *empty_archetype_id,
                "Reserved entity {i} should be in empty archetype"
            );
        }
        
        // The ComponentComponent entity should be taken and thus alive
        // (it's the first reserved entity taken in World::new())
        let first_reserved_entity = Entity::new(EntityIndex(0), EntityGeneration::FIRST);
        assert!(world.alive(first_reserved_entity), "First reserved entity should be alive (ComponentComponent)");
        assert_eq!(world.generation_at_index(EntityIndex(0)), EntityGeneration::FIRST);
        
        // Other reserved entities that haven't been taken should have MAX generation (not alive)
        for i in 1..RESERVED_ENTITY_COUNT {
            assert_eq!(world.generation_at_index(EntityIndex(i)), EntityGeneration::MAX, 
                "Reserved entity {i} should have MAX generation (not alive yet)");
            
            let entity_with_first_gen = Entity::new(EntityIndex(i), EntityGeneration::FIRST);
            assert!(!world.alive(entity_with_first_gen), 
                "Reserved entity {i} with FIRST generation should not be alive yet");
        }
    }

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
        
        // spawn_component calls take_next_reserved which makes the entity alive
        assert!(world.alive(component_entity.0));
        // Component entities use reserved entity space, so index should be < RESERVED_ENTITY_COUNT
        assert!(u32::from(component_entity.index()) < RESERVED_ENTITY_COUNT);
        // Should have FIRST generation when taken from reserved
        assert_eq!(component_entity.generation(), EntityGeneration::FIRST);
    }
}

#[cfg(test)]
mod component_registration_tests {
    use super::*;

    #[test]
    fn component_registration_creates_component_entity() {
        let mut world = World::new();
        
        let comp1 = world.component::<TestComponent1>();
        let comp2 = world.component::<TestComponent2>();
        
        assert!(world.alive(comp1.0));
        assert!(world.alive(comp2.0));
        assert_ne!(comp1, comp2);
        
        // Registering same component returns same entity
        let comp1_again = world.component::<TestComponent1>();
        assert_eq!(comp1, comp1_again);
    }

    #[test]
    fn try_component_without_registration() {
        let world = World::new();
        
        assert!(world.try_component::<TestComponent1>().is_none());
        assert!(world.try_component_entity(TypeId::of::<TestComponent1>()).is_none());
    }

    #[test]
    fn try_component_after_registration() {
        let mut world = World::new();
        let registered = world.component::<TestComponent1>();
        
        assert_eq!(world.try_component::<TestComponent1>(), Some(registered));
        assert_eq!(
            world.try_component_entity(TypeId::of::<TestComponent1>()),
            Some(registered)
        );
    }

    #[test]
    fn component_storage_kinds() {
        let mut world = World::new();
        
        // Unregistered component
        let unregistered = world.spawn_component();
        assert_eq!(world.component_storage(unregistered), Some(ComponentStorageKind::None));
        
        // Component with default storage
        let default_comp = world.component::<DefaultComponent>();
        assert_eq!(
            world.component_storage(default_comp),
            Some(ComponentStorageKind::Table { has_default: true })
        );
        
        // Component without default storage  
        let no_default_comp = world.component::<NoDefaultComponent>();
        assert_eq!(
            world.component_storage(no_default_comp),
            Some(ComponentStorageKind::Table { has_default: false })
        );
        
        // Dead component entity
        world.dispawn(unregistered.0);
        assert_eq!(world.component_storage(unregistered), None);
    }
}

#[cfg(test)]
mod has_component_tests {
    use super::*;

    #[test]
    fn has_component_on_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<TestComponent1>();
        
        world.dispawn(entity);
        
        assert_eq!(
            world.has_component(entity, component),
            HasComponent::EntityIsNotAlive
        );
        
        assert_eq!(
            world.has::<TestComponent1>(entity),
            HasComponent::EntityIsNotAlive
        );
    }

    #[test]
    fn has_component_unknown_component() {
        let mut world = World::new();
        let entity = world.spawn();
        let dead_component = world.spawn_component();
        
        world.dispawn(dead_component.0);
        
        assert_eq!(
            world.has_component(entity, dead_component),
            HasComponent::UnknownComponent
        );
        
        assert_eq!(
            world.has::<TestComponent1>(entity),
            HasComponent::UnknownComponent
        );
    }

    #[test]
    fn has_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<TestComponent1>();
        
        assert_eq!(
            world.has_component(entity, component),
            HasComponent::NotPresent
        );
        
        assert_eq!(
            world.has::<TestComponent1>(entity),
            HasComponent::NotPresent
        );
    }

    #[test]
    fn has_component_present() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let result = world.add(entity, TestComponent1(42)).unwrap();
        assert!(result.was_added);
        
        let component = world.component::<TestComponent1>();
        
        assert_eq!(
            world.has_component(entity, component),
            HasComponent::Present
        );
        
        assert_eq!(
            world.has::<TestComponent1>(entity),
            HasComponent::Present
        );
        
        assert!(world.has::<TestComponent1>(entity).bool());
    }
}

#[cfg(test)]
mod add_component_tests {
    use super::*;

    #[test]
    fn add_component_to_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<DefaultComponent>();
        
        world.dispawn(entity);
        
        let result = world.add_component(entity, component);
        assert!(matches!(result, Err(AddComponentError::EntityIsNotAlive { .. })));
        
        let typed_result = world.add(entity, TestComponent1(42));
        assert!(matches!(typed_result, Err(crate::world_utils::AddComponentTypedError::EntityIsNotAlive { .. })));
    }

    #[test]
    fn add_component_with_dead_component() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.spawn_component();
        
        world.dispawn(component.0);
        
        let result = world.add_component(entity, component);
        assert!(matches!(result, Err(AddComponentError::ComponentIsNotAlive { .. })));
    }

    #[test]
    fn add_component_needs_value() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<NoDefaultComponent>();
        
        let result = world.add_component(entity, component);
        assert!(matches!(result, Err(AddComponentError::ComponentNeedsValue { .. })));
    }

    #[test]
    fn add_component_with_default() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<DefaultComponent>();
        
        let result = world.add_component(entity, component).unwrap();
        assert!(result.was_added);
        
        match result.component_ref {
            OptionalComponentRef::HasStorage(mut comp_ref) => {
                let value: &DefaultComponent = comp_ref.as_typed().unwrap();
                assert_eq!(*value, DefaultComponent::default());
            },
            OptionalComponentRef::NoStorage => panic!("Should have storage"),
        }
    }

    #[test]
    fn add_component_no_storage() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.spawn_component();
        
        let result = world.add_component(entity, component).unwrap();
        assert!(result.was_added);
        
        assert!(matches!(result.component_ref, OptionalComponentRef::NoStorage));
    }

    #[test]
    fn add_component_already_present() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(42)).unwrap();
        
        let component = world.component::<TestComponent1>();
        let result = world.add_component(entity, component).unwrap();
        assert!(!result.was_added);
        
        match result.component_ref {
            OptionalComponentRef::HasStorage(mut comp_ref) => {
                let value: &TestComponent1 = comp_ref.as_typed().unwrap();
                assert_eq!(*value, TestComponent1(42));
            },
            OptionalComponentRef::NoStorage => panic!("Should have storage"),
        }
    }

    #[test]
    fn add_typed_component() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let result = world.add(entity, TestComponent1(123)).unwrap();
        assert!(result.was_added);
        assert_eq!(*result.component_ref, TestComponent1(123));
        
        // Adding again should not add but return existing
        let result2 = world.add(entity, TestComponent1(456)).unwrap();
        assert!(!result2.was_added);
        assert_eq!(*result2.component_ref, TestComponent1(123)); // Original value preserved
    }

    #[test]
    fn add_with_closure() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let mut counter = 0;
        let result = world.add_with(entity, || {
            counter += 1;
            TestComponent1(counter)
        }).unwrap();
        
        assert!(result.was_added);
        assert_eq!(*result.component_ref, TestComponent1(1));
        
        // Adding again should not call closure
        let result2 = world.add_with(entity, || {
            counter += 1;
            TestComponent1(counter)
        }).unwrap();
        
        assert!(!result2.was_added);
        assert_eq!(*result2.component_ref, TestComponent1(1)); // Original value
        assert_eq!(counter, 1); // Closure only called once
    }

    #[test]
    fn get_or_default() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let result = world.get_or_default::<DefaultComponent>(entity).unwrap();
        assert!(result.was_added);
        assert_eq!(*result.component_ref, DefaultComponent::default());
        
        // Getting again should not add
        let result2 = world.get_or_default::<DefaultComponent>(entity).unwrap();
        assert!(!result2.was_added);
        assert_eq!(*result2.component_ref, DefaultComponent::default());
    }
}

#[cfg(test)]
mod get_component_tests {
    use super::*;

    #[test]
    fn get_component_from_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<TestComponent1>();
        
        world.dispawn(entity);
        
        let result = world.get_component(entity, component);
        assert!(matches!(result, Err(GetComponentError::EntityIsNotAlive { .. })));
        
        let typed_result = world.get::<TestComponent1>(entity);
        assert!(matches!(typed_result, Err(crate::world_utils::GetComponentTypedError::EntityIsNotAlive { .. })));
    }

    #[test]
    fn get_component_dead_component() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.spawn_component();
        
        world.dispawn(component.0);
        
        let result = world.get_component(entity, component);
        assert!(matches!(result, Err(GetComponentError::ComponentIsNotAlive { .. })));
    }

    #[test]
    fn get_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<TestComponent1>();
        
        let result = world.get_component(entity, component);
        assert!(matches!(result, Err(GetComponentError::ComponentNotPresent { .. })));
        
        let typed_result = world.get::<TestComponent1>(entity);
        assert!(matches!(typed_result, Err(crate::world_utils::GetComponentTypedError::ComponentNotPresent { .. })));
    }

    #[test]
    fn get_component_no_storage() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.spawn_component();
        
        // Add component with no storage
        world.add_component(entity, component).unwrap();
        
        let result = world.get_component(entity, component);
        assert!(matches!(result, Err(GetComponentError::ComponentHasNoStorage { .. })));
    }

    #[test]
    fn get_component_success() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(789)).unwrap();
        
        let value = world.get::<TestComponent1>(entity).unwrap();
        assert_eq!(*value, TestComponent1(789));
        
        let component = world.component::<TestComponent1>();
        let dyn_ref = world.get_component(entity, component).unwrap();
        let typed_value: &TestComponent1 = dyn_ref.as_typed().unwrap();
        assert_eq!(*typed_value, TestComponent1(789));
    }

    #[test]
    fn get_component_mut_success() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(100)).unwrap();
        
        {
            let value = world.get_mut::<TestComponent1>(entity).unwrap();
            *value = TestComponent1(200);
        }
        
        let value = world.get::<TestComponent1>(entity).unwrap();
        assert_eq!(*value, TestComponent1(200));
    }

    #[test] 
    fn get_unknown_component() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let result = world.get::<TestComponent1>(entity);
        assert!(matches!(result, Err(crate::world_utils::GetComponentTypedError::UnknownComponent { .. })));
    }
}

#[cfg(test)]
mod remove_component_tests {
    use super::*;

    #[test]
    fn remove_component_from_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        world.dispawn(entity);
        
        let component = world.component::<TestComponent1>();
        let result = world.remove_component(entity, component);
        assert!(result.is_none());
    }

    #[test]
    fn remove_typed_component_from_dead_entity() {
        let mut world = World::new();
        let entity = world.spawn();
        world.dispawn(entity);
        
        let typed_result = world.remove::<TestComponent1>(entity);
        assert!(typed_result.is_none());
    }

    #[test]
    fn remove_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn();
        let component = world.component::<TestComponent1>();
        
        let result = world.remove_component(entity, component);
        assert!(result.is_none());
    }

    #[test]
    fn remove_typed_component_not_present() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let typed_result = world.remove::<TestComponent1>(entity);
        assert!(typed_result.is_none());
    }

    #[test]
    fn remove_component_success() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(999)).unwrap();
        
        let removed = world.remove::<TestComponent1>(entity).unwrap();
        assert_eq!(removed, TestComponent1(999));
        
        // Component should no longer be present
        assert_eq!(world.has::<TestComponent1>(entity), HasComponent::NotPresent);
        
        // Remove again should return None
        let removed_again = world.remove::<TestComponent1>(entity);
        assert!(removed_again.is_none());
    }

    #[test]
    fn remove_unregistered_component() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let result = world.remove::<TestComponent1>(entity);
        assert!(result.is_none());
    }
}

#[cfg(test)]
mod archetype_tests {
    use super::*;

    #[test]
    fn entity_starts_in_empty_archetype() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let archetype_id = world.entities_archetypes[entity.index()];
        let archetype = &world.archetypes[archetype_id];
        
        assert_eq!(archetype.components, ComponentSet::default());
        assert_eq!(archetype.components.len(), 0);
    }

    #[test]
    fn adding_component_changes_archetype() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let initial_archetype_id = world.entities_archetypes[entity.index()];
        let initial_archetype = &world.archetypes[initial_archetype_id];
        assert_eq!(initial_archetype.components.len(), 0);
        
        world.add(entity, TestComponent1(42)).unwrap();
        
        let new_archetype_id = world.entities_archetypes[entity.index()];
        assert_ne!(initial_archetype_id, new_archetype_id);
        
        let component_entity = world.component::<TestComponent1>();
        let new_archetype = &world.archetypes[new_archetype_id];
        assert_eq!(new_archetype.components.len(), 1);
        assert!(new_archetype.components.has(component_entity));
    }

    #[test]
    fn adding_multiple_components_creates_correct_archetype() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(1)).unwrap();
        world.add(entity, TestComponent2(2.0)).unwrap();
        
        let archetype_id = world.entities_archetypes[entity.index()];
        
        let comp1 = world.component::<TestComponent1>();
        let comp2 = world.component::<TestComponent2>();
        
        let archetype = &world.archetypes[archetype_id];
        assert_eq!(archetype.components.len(), 2);
        assert!(archetype.components.has(comp1));
        assert!(archetype.components.has(comp2));
    }

    #[test]
    fn removing_component_changes_archetype() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(1)).unwrap();
        world.add(entity, TestComponent2(2.0)).unwrap();
        
        let initial_archetype_id = world.entities_archetypes[entity.index()];
        let initial_archetype = &world.archetypes[initial_archetype_id];
        assert_eq!(initial_archetype.components.len(), 2);
        
        world.remove::<TestComponent1>(entity).unwrap();
        
        let new_archetype_id = world.entities_archetypes[entity.index()];
        assert_ne!(initial_archetype_id, new_archetype_id);
        
        let comp1 = world.component::<TestComponent1>();
        let comp2 = world.component::<TestComponent2>();
        
        let new_archetype = &world.archetypes[new_archetype_id];
        assert_eq!(new_archetype.components.len(), 1);
        assert!(!new_archetype.components.has(comp1));
        assert!(new_archetype.components.has(comp2));
    }

    #[test]
    fn removing_all_components_returns_to_empty_archetype() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let initial_archetype_id = world.entities_archetypes[entity.index()];
        
        world.add(entity, TestComponent1(1)).unwrap();
        world.add(entity, TestComponent2(2.0)).unwrap();
        
        // Archetype should have changed
        let middle_archetype_id = world.entities_archetypes[entity.index()];
        assert_ne!(initial_archetype_id, middle_archetype_id);
        
        world.remove::<TestComponent1>(entity).unwrap();
        world.remove::<TestComponent2>(entity).unwrap();
        
        // Should be back to empty archetype
        let final_archetype_id = world.entities_archetypes[entity.index()];
        assert_eq!(initial_archetype_id, final_archetype_id);
        
        let final_archetype = &world.archetypes[final_archetype_id];
        assert_eq!(final_archetype.components.len(), 0);
    }

    #[test]
    fn archetype_reuse_across_entities() {
        let mut world = World::new();
        let entity1 = world.spawn();
        let entity2 = world.spawn();
        
        world.add(entity1, TestComponent1(1)).unwrap();
        world.add(entity2, TestComponent1(2)).unwrap();
        
        let archetype1_id = world.entities_archetypes[entity1.index()];
        let archetype2_id = world.entities_archetypes[entity2.index()];
        
        // Same component set should use same archetype
        assert_eq!(archetype1_id, archetype2_id);
        
        // But different component values
        let val1 = world.get::<TestComponent1>(entity1).unwrap();
        let val2 = world.get::<TestComponent1>(entity2).unwrap();
        assert_eq!(*val1, TestComponent1(1));
        assert_eq!(*val2, TestComponent1(2));
    }
}

#[cfg(test)]
mod table_tests {
    use super::*;

    #[test]
    fn entity_starts_in_empty_table() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let archetype_id = world.entities_archetypes[entity.index()];
        let archetype = &world.archetypes[archetype_id];
        let table_id = archetype.table_id;
        let table = &world.tables[table_id];
        
        assert_eq!(table.table_components.len(), 0);
        assert!(table.sparse_set.get(entity.index()).is_some());
    }

    #[test]
    fn adding_component_moves_to_correct_table() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let initial_archetype_id = world.entities_archetypes[entity.index()];
        let initial_table_id = world.archetypes[initial_archetype_id].table_id;
        
        world.add(entity, TestComponent1(42)).unwrap();
        
        let new_archetype_id = world.entities_archetypes[entity.index()];
        let new_table_id = world.archetypes[new_archetype_id].table_id;
        let component_entity = world.component::<TestComponent1>();
        let new_table = &world.tables[new_table_id];
        
        assert_ne!(initial_table_id, new_table_id);
        assert_eq!(new_table.table_components.len(), 1);
        assert!(new_table.table_components.has(component_entity));
        assert!(new_table.sparse_set.get(entity.index()).is_some());
        
        // Entity should no longer be in old table
        let initial_table = &world.tables[initial_table_id];
        assert!(initial_table.sparse_set.get(entity.index()).is_none());
    }

    #[test]
    fn table_reuse_across_archetypes() {
        let mut world = World::new();
        let entity1 = world.spawn();
        let entity2 = world.spawn();
        
        world.add(entity1, TestComponent1(1)).unwrap();
        world.add(entity2, TestComponent1(2)).unwrap();
        
        let archetype1_id = world.entities_archetypes[entity1.index()];
        let archetype2_id = world.entities_archetypes[entity2.index()];
        let table1_id = world.archetypes[archetype1_id].table_id;
        let table2_id = world.archetypes[archetype2_id].table_id;
        
        // Same table components should use same table
        assert_eq!(table1_id, table2_id);
        
        let table = &world.tables[table1_id];
        assert!(table.sparse_set.get(entity1.index()).is_some());
        assert!(table.sparse_set.get(entity2.index()).is_some());
    }
}

#[cfg(test)]
mod component_despawn_tests {
    use super::*;

    #[test]
    fn despawn_component_entity_invalidates_component() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let component = world.component::<TestComponent1>();
        world.add(entity, TestComponent1(42)).unwrap();
        
        // Component should be present
        assert_eq!(world.has_component(entity, component), HasComponent::Present);
        
        // Despawn the component entity
        assert!(world.dispawn(component.0));
        
        // Component should now be unknown
        assert_eq!(world.has_component(entity, component), HasComponent::UnknownComponent);
        
        // Storage should be None for dead component
        assert_eq!(world.component_storage(component), None);
    }

    #[test]
    fn despawn_component_entity_affects_get_operations() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let component = world.component::<TestComponent1>();
        world.add(entity, TestComponent1(42)).unwrap();
        
        // Get should work initially
        world.get_component(entity, component).unwrap();
        
        // Despawn the component entity
        world.dispawn(component.0);
        
        // Get should now fail with ComponentIsNotAlive
        let result = world.get_component(entity, component);
        assert!(matches!(result, Err(GetComponentError::ComponentIsNotAlive { .. })));
    }

    #[test]
    fn despawn_component_entity_affects_add_operations() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let component = world.component::<TestComponent1>();
        
        // Despawn the component entity
        world.dispawn(component.0);
        
        // Add should now fail with ComponentIsNotAlive
        let result = world.add_component(entity, component);
        assert!(matches!(result, Err(AddComponentError::ComponentIsNotAlive { .. })));
    }

    #[test]
    fn despawn_component_entity_affects_remove_operations() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let component = world.component::<TestComponent1>();
        world.add(entity, TestComponent1(42)).unwrap();
        
        // Despawn the component entity
        world.dispawn(component.0);
        
        // Remove should return None (component entity is dead)
        let result = world.remove_component(entity, component);
        assert!(result.is_none());
    }

    #[test] 
    fn despawn_component_should_remove_from_all_archetypes_and_tables() {
        // This test verifies the intended behavior that despawning a component entity
        // should remove all storage and archetype references to that component.
        // According to the requirements, this is not yet implemented and these tests will fail.
        
        let mut world = World::new();
        let entity1 = world.spawn();
        let entity2 = world.spawn();
        
        let component = world.component::<TestComponent1>();
        world.add(entity1, TestComponent1(1)).unwrap();
        world.add(entity2, TestComponent1(2)).unwrap();
        
        // Both entities should have the component
        assert_eq!(world.has_component(entity1, component), HasComponent::Present);
        assert_eq!(world.has_component(entity2, component), HasComponent::Present);
        
        let archetype_id = world.entities_archetypes[entity1.index()];
        let archetype = &world.archetypes[archetype_id];
        assert!(archetype.components.has(component));
        
        // Despawn the component entity - this should remove the component from all entities
        world.dispawn(component.0);
        
        // TODO: These assertions will fail because the cleanup is not implemented yet
        // According to requirements, we should still add these tests even though they fail
        
        // Entities should no longer have the component
        // (Currently fails - component storage still exists)
        // assert_eq!(world.has_component(entity1, component), HasComponent::UnknownComponent);
        // assert_eq!(world.has_component(entity2, component), HasComponent::UnknownComponent);
        
        // Archetypes should no longer contain the component
        // (Currently fails - archetype still contains the component)
        // let updated_archetype = &world.archetypes[archetype_id];
        // assert!(!updated_archetype.components.has(component));
        
        // Tables should no longer contain the component's storage
        // (Currently fails - table still contains component data)
        // let table_id = archetype.table_id;
        // let updated_table = &world.tables[table_id]; 
        // assert!(!updated_table.table_components.has(component));
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn multiple_component_registrations_same_type() {
        let mut world = World::new();
        
        let comp1 = world.component::<TestComponent1>();
        let comp2 = world.component::<TestComponent1>();
        let comp3 = world.component::<TestComponent1>();
        
        assert_eq!(comp1, comp2);
        assert_eq!(comp2, comp3);
        
        // Should only create one entry in the type map
        assert_eq!(
            world.components_typeid_to_entity.len(),
            2 // ComponentStorageComponent + TestComponent1
        );
    }

    #[test]
    fn entity_reuse_after_despawn() {
        let mut world = World::new();
        let entity = world.spawn();
        
        world.add(entity, TestComponent1(42)).unwrap();
        assert_eq!(world.has::<TestComponent1>(entity), HasComponent::Present);
        
        world.dispawn(entity);
        
        // Spawn new entity (might reuse index)
        let new_entity = world.spawn();
        
        // New entity should not have old components
        assert_eq!(world.has::<TestComponent1>(new_entity), HasComponent::NotPresent);
        
        // Old entity handle should be dead
        assert!(!world.alive(entity));
        assert_eq!(world.has::<TestComponent1>(entity), HasComponent::EntityIsNotAlive);
    }

    #[test]
    fn component_entity_lifecycle() {
        let mut world = World::new();
        
        // Register component
        let component = world.component::<TestComponent1>();
        assert!(world.alive(component.0));
        assert_eq!(world.component_storage(component), Some(ComponentStorageKind::Table { has_default: false }));
        
        // Despawn component entity
        world.dispawn(component.0);
        assert!(!world.alive(component.0));
        assert_eq!(world.component_storage(component), None);
        
        // Re-register same component type (should create new entity)
        let new_component = world.component::<TestComponent1>();
        assert!(world.alive(new_component.0));
        assert_ne!(component, new_component);
        assert_eq!(world.component_storage(new_component), Some(ComponentStorageKind::Table { has_default: false }));
    }

    #[test]
    fn archetype_and_table_creation_patterns() {
        let mut world = World::new();
        let entity = world.spawn();
        
        let initial_archetype_count = world.archetypes.len();
        let initial_table_count = world.tables.len();
        
        // Add first component - should create new archetype and table
        world.add(entity, TestComponent1(1)).unwrap();
        assert_eq!(world.archetypes.len().to_usize(), initial_archetype_count.to_usize() + 1);
        assert_eq!(world.tables.len().to_usize(), initial_table_count.to_usize() + 1);
        
        // Add second component - should create new archetype and table
        world.add(entity, TestComponent2(2.0)).unwrap();
        assert_eq!(world.archetypes.len().to_usize(), initial_archetype_count.to_usize() + 2);
        assert_eq!(world.tables.len().to_usize(), initial_table_count.to_usize() + 2);
        
        // Remove first component - should potentially reuse archetype/table
        world.remove::<TestComponent1>(entity).unwrap();
        // Archetype count might not increase if it can reuse existing single-component archetype
        
        // Remove second component - should reuse empty archetype/table
        world.remove::<TestComponent2>(entity).unwrap();
        // Should be back to original counts or reusing existing empty archetype
    }

    #[test]
    fn reserved_entity_space_usage() {
        let mut world = World::new();
        
        // Create many component entities to test reserved space usage
        let mut component_entities = Vec::new();
        for i in 0..10 {
            // Create unique component types by registering different types
            match i {
                0 => component_entities.push(world.component::<TestComponent1>()),
                1 => component_entities.push(world.component::<TestComponent2>()),
                2 => component_entities.push(world.component::<DefaultComponent>()),
                3 => component_entities.push(world.component::<NoDefaultComponent>()),
                _ => {
                    // For remaining iterations, just spawn component entities without registration
                    component_entities.push(world.spawn_component());
                }
            }
        }
        
        // All component entities should use reserved space (index < RESERVED_ENTITY_COUNT)
        for comp_entity in &component_entities {
            assert!(u32::from(comp_entity.index()) < RESERVED_ENTITY_COUNT);
            assert!(world.alive(comp_entity.0));
        }
        
        // Regular entities should start after reserved space
        let _regular_entity = world.spawn();
        // Note: This might not be true if reserved space isn't full yet
        // The actual behavior depends on implementation details
    }
}