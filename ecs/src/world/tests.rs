use super::*;

use crate::world::{
    EntityGeneration,
    HasComponent, AddComponentError
};
use crate::world_utils::{
    HasComponentTyped,
};

use std::any::TypeId;
use std::assert_matches::assert_matches;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TestComponent1(u32);

#[derive(Clone, Copy, Debug, PartialEq)]
struct TestComponent2(f32);

#[derive(Clone, Debug, PartialEq, Eq)]
struct DefaultComponent(String);

impl Default for DefaultComponent {
    fn default() -> Self {
        Self(format!("default value"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
struct ZSTComponent;

mod entity_spawning {
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
        
        assert_eq!(world.alive(entity), true);
        assert_eq!(world.dispawn(entity), true);
        assert_eq!(world.alive(entity), false);
        assert_eq!(world.dispawn(entity), false);
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

mod typed_components_api {
    use super::*;

    #[test]
    fn registering_components() {
        let mut world = World::new();

        assert_eq!(world.try_component::<TestComponent1>(), None);
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent1>()), None);
        assert_eq!(world.try_component::<TestComponent2>(), None);
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent2>()), None);

        let e1 = world.component::<TestComponent1>();

        assert!(world.alive(e1));

        assert_eq!(world.try_component::<TestComponent1>(), Some(e1));
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent1>()), Some(e1));
        assert_eq!(world.try_component::<TestComponent2>(), None);
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent2>()), None);

        let e2 = world.component::<TestComponent2>();

        assert!(world.alive(e1));
        assert!(world.alive(e2));

        assert_eq!(world.try_component::<TestComponent1>(), Some(e1));
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent1>()), Some(e1));
        assert_eq!(world.try_component::<TestComponent2>(), Some(e2));
        assert_eq!(world.try_component_entity(TypeId::of::<TestComponent2>()), Some(e2));
    }

    #[test]
    fn unregistering_components() {
        let mut world = World::new();

        let e1 = world.component::<TestComponent1>();
        let e2 = world.component::<TestComponent2>();

        assert!(world.alive(e1));
        assert!(world.alive(e2));

        assert_eq!(world.try_component::<TestComponent1>(), Some(e1));
        assert_eq!(world.try_component::<TestComponent2>(), Some(e2));

        world.dispawn(e2);

        assert!(world.alive(e1));
        assert!(!world.alive(e2));

        assert_eq!(world.try_component::<TestComponent1>(), Some(e1));
        assert_eq!(world.try_component::<TestComponent2>(), None);
    }

    #[test]
    fn unregistering_components_then_reregistering() {
        let mut world = World::new();

        let e1 = world.component::<TestComponent1>();
        let e2 = world.component::<TestComponent2>();
        world.dispawn(e2);
        assert!(world.alive(e1));
        assert!(!world.alive(e2));
        let new_e2 = world.component::<TestComponent2>();
        assert_ne!(e2, new_e2);
        assert!(!world.alive(e2));
        assert!(world.alive(new_e2));
    }

    #[test]
    fn components_storage() {
        let mut world = World::new();

        let e1 = world.component::<DefaultComponent>();
        let e2 = world.component::<TestComponent1>();
        let e3 = world.component::<ZSTComponent>();

        assert_eq!(world.component_storage(e1), Some(ComponentStorageKind::Table { has_default: true }));
        assert_eq!(world.component_storage(e2), Some(ComponentStorageKind::Table { has_default: false }));
        assert_eq!(world.component_storage(e3), Some(ComponentStorageKind::Table { has_default: true }));
    }

    #[test]
    fn component_storage_of_component_storage_component() {
        let mut world = World::new();
        let e = world.component::<ComponentStorageComponent>();
        assert_matches!(world.component_storage(e), Some(ComponentStorageKind::Table { .. }));
    }
}

mod entities_with_typed_components {
    use super::*;

    #[test]
    fn simple_registering_first() {
        let mut world = World::new();

        let e = world.spawn();

        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::UnknownComponent);
        let component = world.component::<TestComponent1>();
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::NotPresent);
        assert_matches!(world.has::<TestComponent2>(e), HasComponentTyped::UnknownComponent);
        assert_matches!(world.has_component(e, component), HasComponent::NotPresent);
        assert_matches!(
            world.add(e, TestComponent1(50)).unwrap(),
            AddComponent { component_ref: &mut TestComponent1(50), was_added: true },
        );
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::Present);
        assert_matches!(world.has::<TestComponent2>(e), HasComponentTyped::UnknownComponent);
        assert_matches!(world.has_component(e, component), HasComponent::Present);

        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(50)));
        assert_matches!(world.get::<TestComponent2>(e), Err(GetComponentTypedError::UnknownComponent { .. }));

        world.get_mut::<TestComponent1>(e).unwrap().0 = 42;
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));

        world.remove::<TestComponent1>(e).unwrap();

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::ComponentNotPresent { .. }));
        assert_matches!(world.get::<TestComponent2>(e), Err(GetComponentTypedError::UnknownComponent { .. }));
    }

    #[test]
    fn simple_auto_register() {
        let mut world = World::new();
        let e = world.spawn();

        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::UnknownComponent);
        assert_matches!(
            world.add(e, TestComponent1(42)),
            Ok(AddComponent { component_ref: &mut TestComponent1(42), was_added: true })
        );
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::Present);
        world.remove::<TestComponent1>(e).unwrap();
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::NotPresent);
    }

    #[test]
    fn simple_zst() {
        let mut world = World::new();
        let e = world.spawn();

        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::UnknownComponent);
        assert_matches!(
            world.add(e, ZSTComponent),
            Ok(AddComponent { component_ref: &mut ZSTComponent, was_added: true })
        );
        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::Present);
        world.remove::<ZSTComponent>(e).unwrap();
        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::NotPresent);
    }

    #[test]
    fn on_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();

        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.dispawn(e), true);

        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::EntityIsNotAlive);
        assert_matches!(world.has::<TestComponent2>(e), HasComponentTyped::UnknownComponent);

        assert_matches!(world.dispawn(e), false);
    }

    #[test]
    fn add_multiple_times() {
        let mut world = World::new();
        let e = world.spawn();

        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
        world.add(e, TestComponent1(50)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
    }

    #[test]
    fn add_with_simple() {
        let mut world = World::new();
        let e = world.spawn();

        world.add_with(e, || TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
    }

    #[test]
    fn add_with_multiple_times() {
        let mut world = World::new();
        let e = world.spawn();

        world.add_with(e, || TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
        world.add_with(e, || {
            assert!(false, "Should not be called");
            TestComponent1(50)
        }).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
    }

    #[test]
    fn add_remove_add() {
        let mut world = World::new();
        let e = world.spawn();

        assert_eq!(world.has::<TestComponent1>(e), HasComponentTyped::UnknownComponent);
        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
        world.remove::<TestComponent1>(e).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::ComponentNotPresent { .. }));
        world.add(e, TestComponent1(52)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(52)));
    }

    #[test]
    fn unregister_component_removes_it() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<TestComponent1>();

        world.add(e, TestComponent1(42)).unwrap();
        world.dispawn(c);

        assert!(!world.alive(c));
        assert!(world.alive(e));

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::UnknownComponent { .. }));

        world.component::<TestComponent1>();

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::ComponentNotPresent { .. }));
    }
}

mod untyped_component_apis {
    use super::*;

    #[test]
    fn simple_spawn_component() {
        let mut world = World::new();
        let e = world.spawn_component();

        assert!(world.alive(e));

        assert_matches!(world.component_storage(e), Some(ComponentStorageKind::None));
        world.dispawn(e);
        assert_matches!(world.component_storage(e), None);
    }

    #[test]
    fn simple_spawn() {
        let mut world = World::new();
        let e = ComponentEntity(world.spawn());

        assert!(world.alive(e));

        assert_matches!(world.component_storage(e), Some(ComponentStorageKind::None));
        world.dispawn(e);
        assert_matches!(world.component_storage(e), None);
    }
}

mod entities_with_untyped_components {
    use super::*;

    #[test]
    fn simple() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);
        let added = world.add_component(e, c).unwrap();
        assert!(matches!(added.component_ref, OptionalComponentRef::NoStorage));
        assert_eq!(added.was_added, true);
        assert_eq!(world.has_component(e, c), HasComponent::Present);

        assert!(matches!(world.get_component(e, c), Err(GetComponentError::ComponentHasNoStorage { .. })));

        assert!(matches!(world.remove_component(e, c).unwrap(), OptionalComponentRef::NoStorage));
    }

    #[test]
    fn on_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        world.add_component(e, c).unwrap();
        world.dispawn(e);
        assert!(matches!(world.add_component(e, c), Err(AddComponentError::EntityIsNotAlive { .. })));
        assert_eq!(world.has_component(e, c), HasComponent::EntityIsNotAlive);

        assert!(matches!(world.remove_component(e, c), Err(RemoveComponentError::EntityIsNotAlive { .. })));
    }

    #[test]
    fn add_multiple_times() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);

        let added = world.add_component(e, c).unwrap();
        assert!(matches!(added.component_ref, OptionalComponentRef::NoStorage));
        assert_eq!(added.was_added, true);

        assert_eq!(world.has_component(e, c), HasComponent::Present);

        let added = world.add_component(e, c).unwrap();
        assert!(matches!(added.component_ref, OptionalComponentRef::NoStorage));
        assert_eq!(added.was_added, false);

        assert_eq!(world.has_component(e, c), HasComponent::Present);
    }

    #[test]
    fn add_remove_add() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);
        world.add_component(e, c).unwrap();
        assert_eq!(world.has_component(e, c), HasComponent::Present);
        world.remove_component(e, c).unwrap();
        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);
        world.add_component(e, c).unwrap();
        assert_eq!(world.has_component(e, c), HasComponent::Present);
    }

    #[test]
    fn dispawning_the_component() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);
        world.add_component(e, c).unwrap();

        assert!(world.alive(c));
        world.dispawn(c);
        assert!(!world.alive(c));

        assert_matches!(world.has_component(e, c), HasComponent::ComponentIsNotAlive);
        assert!(matches!(world.add_component(e, c), Err(AddComponentError::ComponentIsNotAlive { .. })));
    }

    #[test]
    fn with_dead_component() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert!(world.alive(c));
        world.dispawn(c);
        assert!(!world.alive(c));

        assert_matches!(world.has_component(e, c), HasComponent::ComponentIsNotAlive);
        assert!(matches!(world.add_component(e, c), Err(AddComponentError::ComponentIsNotAlive { .. })));
    }
}

mod misc {
    use super::*;

    #[test]
    fn same_index_different_entity() {
        let mut world = World::new();
        let e = world.spawn();

        world.add(e, TestComponent1(42)).unwrap();

        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
        
        assert_eq!(world.dispawn(e), true);

        let e2 = world.spawn();
        assert_ne!(e, e2);
        assert_eq!(e.index(), e2.index());
        assert_eq!(e.generation().next(), e2.generation());

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::EntityIsNotAlive { .. }));
        assert_matches!(world.get::<TestComponent1>(e2), Err(GetComponentTypedError::ComponentNotPresent { .. }));

        assert_eq!(
            world.add(e2, TestComponent1(52)).unwrap().was_added,
            true,
        );

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::EntityIsNotAlive { .. }));
        assert_matches!(world.get::<TestComponent1>(e2), Ok(&TestComponent1(52)));
    }

    #[test]
    fn add_component_use_default() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<DefaultComponent>();

        world.add_component(e, c).unwrap();

        assert_eq!(
            world.get::<DefaultComponent>(e).unwrap().0,
            "default value",
        );
    }

    #[test]
    fn add_component_requires_value() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<TestComponent1>();

        assert!(matches!(
            world.add_component(e, c),
            Err(AddComponentError::ComponentNeedsValue { .. }),
        ));
    }

    #[test]
    fn simple_remove_component_with_entity() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<TestComponent1>();

        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(&TestComponent1(42)));
        world.remove_component(e, c).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::ComponentNotPresent { .. }));
    }
}
