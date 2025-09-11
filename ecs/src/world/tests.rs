use crate::world::*;
use crate::world_utils::*;

use std::any::TypeId;
use std::assert_matches::assert_matches;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::iter::{repeat, zip};

use utils::prelude::*;
use utils::itertools::izip;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TestComponent1(u32);

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
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

#[derive(Clone, Debug)]
struct DropCheckComponent(Arc<AtomicUsize>);

#[derive(Debug, PartialEq, Eq, Default)]
struct NotCloneComponent;

impl Drop for DropCheckComponent {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

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
        assert_matches!(world.dispawn(entity), Ok(()));
        assert_eq!(world.alive(entity), false);
        assert_matches!(world.dispawn(entity), Err(DispawnError::EntityIsNotAlive { .. }));
    }

    #[test]
    fn generation_at_index() {
        let mut world = World::new();
        let entity = world.spawn();
        
        assert_eq!(
            world.generation_at_index(entity.index()),
            entity.generation()
        );
        
        world.dispawn(entity).unwrap();
        
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

        world.dispawn(e2).unwrap();

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
        world.dispawn(e2).unwrap();
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

        assert_matches!(world.component_storage(e1), Some(ComponentStorageKind::Table { dynvec_meta: DynVecMetadata { default_fn: Some(_), .. }, .. }));
        assert_matches!(world.component_storage(e2), Some(ComponentStorageKind::Table { dynvec_meta: DynVecMetadata { default_fn: None, .. }, .. }));
        assert_matches!(world.component_storage(e3), Some(ComponentStorageKind::Table { dynvec_meta: DynVecMetadata { default_fn: Some(_), .. }, .. }));
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
        assert_matches!(world.add(e, TestComponent1(50)), Ok(AddComponent::Added));
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
        assert_matches!(world.add(e, TestComponent1(42)), Ok(AddComponent::Added));
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::Present);
        world.remove::<TestComponent1>(e).unwrap();
        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::NotPresent);
    }

    #[test]
    fn simple_zst() {
        let mut world = World::new();
        let e = world.spawn();

        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::UnknownComponent);
        assert_matches!(world.add(e, ZSTComponent), Ok(AddComponent::Added));
        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::Present);
        world.remove::<ZSTComponent>(e).unwrap();
        assert_matches!(world.has::<ZSTComponent>(e), HasComponentTyped::NotPresent);
    }

    #[test]
    fn simple_set_once() {
        let mut world = World::new();
        let e = world.spawn();

        world.set(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get(e), Ok(TestComponent1(42)));
    }

    #[test]
    fn on_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();

        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.dispawn(e), Ok(()));

        assert_matches!(world.has::<TestComponent1>(e), HasComponentTyped::EntityIsNotAlive);
        assert_matches!(world.has::<TestComponent2>(e), HasComponentTyped::UnknownComponent);

        assert_matches!(world.dispawn(e), Err(DispawnError::EntityIsNotAlive { .. }));
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
    fn set_multiple_times() {
        let mut world = World::new();
        let e = world.spawn();

        world.set(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get(e), Ok(TestComponent1(42)));
        world.set(e, TestComponent1(52)).unwrap();
        assert_matches!(world.get(e), Ok(TestComponent1(52)));
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
        world.dispawn(c).unwrap();

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
        world.dispawn(e).unwrap();
        assert_matches!(world.component_storage(e), None);
    }

    #[test]
    fn simple_spawn() {
        let mut world = World::new();
        let e = ComponentEntity(world.spawn());

        assert!(world.alive(e));

        assert_matches!(world.component_storage(e), Some(ComponentStorageKind::None));
        world.dispawn(e).unwrap();
        assert_matches!(world.component_storage(e), None);
    }
}

mod torturing_components {
    use super::*;

    #[test]
    fn add_components_to_components() {
        let mut world = World::new();
        let c = world.component::<TestComponent1>();

        world.add(c, TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(c), Ok(TestComponent1(42)));
        world.add(c, TestComponent2(42.0)).unwrap();
        assert_matches!(world.get::<TestComponent2>(c), Ok(TestComponent2(42.0)));

        world.remove::<TestComponent1>(c).unwrap();
        assert_matches!(world.get::<TestComponent1>(c), Err(GetComponentTypedError::ComponentNotPresent { .. }));
        assert_matches!(world.get::<TestComponent2>(c), Ok(TestComponent2(42.0)));
        world.remove::<TestComponent2>(c).unwrap();
        assert_matches!(world.get::<TestComponent1>(c), Err(GetComponentTypedError::ComponentNotPresent { .. }));
        assert_matches!(world.get::<TestComponent2>(c), Err(GetComponentTypedError::ComponentNotPresent { .. }));
    }

    #[test]
    fn removing_storage_compenent() {
        let mut world = World::new();
        let c = world.component::<TestComponent1>();
        let e = world.spawn();

        // C is an internal entity and thus should not be allowed to be changed
        // like that
        assert_matches!(
            world.remove::<ComponentStorageComponent>(c),
            Err(RemoveComponentTypedError::Forbidden { reason: _ })
        );

        world.add(e, TestComponent1(42)).unwrap();
        assert_matches!(world.get::<TestComponent1>(e), Ok(TestComponent1(42)));

        assert_matches!(
            world.remove::<ComponentStorageComponent>(c),
            Err(RemoveComponentTypedError::Forbidden { reason: _ })
        );
    }

    #[test]
    fn adding_storage_to_custom_component_entity() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        world.component::<u32>();

        world.add(c, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<u32>(),
        }).unwrap();

        assert_matches!(
            world.add_component_with(e, c, || 53u32),
            Ok(AddComponent::Added),
        );

        assert_matches!(
            world.get_component(e, c).unwrap().as_typed::<u32>(),
            Ok(&53u32),
        );
        assert_matches!(
            world.get::<u32>(e),
            Err(GetComponentTypedError::ComponentNotPresent { .. }),
        );
        world.set(e, 1288u32).unwrap();
        assert_matches!(
            world.get::<u32>(e),
            Ok(&1288u32),
        );
        assert_eq!(
            world.get_component(e, c).unwrap().as_typed::<u32>().unwrap(),
            &53u32,
        );
    }

    #[test]
    fn add_component_with_wrong_type() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        world.component::<u32>();

        world.add(c, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<u32>(),
        }).unwrap();
        assert_matches!(
            world.add_component_with(e, c, || format!("Fail!")),
            Err(AddComponentWithError::TypeMismatched { .. }),
        );
    }

    #[test]
    fn removing_storage_component_on_custom_component_before_usage() {
        let mut world = World::new();
        let c = world.spawn_component();

        world.add(c, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<TestComponent1>(),
        }).unwrap();

        // Should be allowed because nobody uses the storage of this entity
        world.remove::<ComponentStorageComponent>(c).unwrap();

        let e = world.spawn();

        // can still use the component, it has no storage

        assert_matches!(world.add_component(e, c), Ok(AddComponent::Added));
        assert_matches!(
            world.get_component(e, c),
            Err(GetComponentError::ComponentHasNoStorage { .. }),
        );
    }

    #[test]
    fn removing_storage_component_on_custom_component_after_usage() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        let drops = Arc::new(AtomicUsize::new(0));

        world.add(c, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<DropCheckComponent>(),
        }).unwrap();
        world.add_component_with(
            e, c,
            || DropCheckComponent(Arc::clone(&drops))
        ).unwrap();
        assert_eq!(world.has_component(e, c), HasComponent::Present);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_matches!(world.remove::<ComponentStorageComponent>(c), Err(RemoveComponentTypedError::Forbidden { .. }));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(world.has_component(e, c), HasComponent::Present);
        assert_matches!(world.get_component(e, c), Ok(_));
        assert_eq!(world.try_component::<DropCheckComponent>(), None);
    }

    #[test]
    fn add_component_storage_component_on_custom_component_after_usage() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        world.add_component(e, c).unwrap();

        assert_matches!(
            world.add(c, ComponentStorageComponent {
                dynvec_meta: DynVecMetadata::new::<TestComponent1>(),
            }),
            Err(AddComponentTypedError::Forbidden { reason: _ })
        );

        assert_matches!(world.get_component(e, c), Err(GetComponentError::ComponentHasNoStorage { .. }));
        assert!(world.has_component(e, c).is_present());
    }

    #[test]
    fn mutate_component_storage_component_on_registered_component() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<TestComponent1>();

        world.add(e, TestComponent1(42)).unwrap();

        assert_matches!(
            world.get_mut::<ComponentStorageComponent>(c),
            Err(GetComponentTypedError::Forbidden { .. })
        );

        assert_eq!(world.get::<TestComponent1>(e).unwrap(), &TestComponent1(42));
        world.set(e, TestComponent1(52)).unwrap();
        assert_eq!(world.get::<TestComponent1>(e).unwrap(), &TestComponent1(52));
    }

    #[test]
    fn dispawning_custom_component_with_storage_drops_it() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        let drops = Arc::new(AtomicUsize::new(0));

        world.add(c, ComponentStorageComponent {
            dynvec_meta: DynVecMetadata::new::<DropCheckComponent>(),
        }).unwrap();
        world.add_component_with(
            e, c,
            || DropCheckComponent(Arc::clone(&drops))
        ).unwrap();
        assert_eq!(world.has_component(e, c), HasComponent::Present);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        world.dispawn(c).unwrap();
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert_eq!(world.has_component(e, c), HasComponent::ComponentIsNotAlive);
        assert_eq!(world.try_component::<DropCheckComponent>(), None);
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
        assert_matches!(world.add_component(e, c), Ok(AddComponent::Added));
        assert_eq!(world.has_component(e, c), HasComponent::Present);

        assert_matches!(world.get_component(e, c), Err(GetComponentError::ComponentHasNoStorage { .. }));

        assert_matches!(world.remove_component(e, c).unwrap(), OptionalComponentRef::NoStorage);
    }

    #[test]
    fn on_dead_entity() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        world.add_component(e, c).unwrap();
        world.dispawn(e).unwrap();
        assert_matches!(world.add_component(e, c), Err(AddComponentError::EntityIsNotAlive { .. }));
        assert_eq!(world.has_component(e, c), HasComponent::EntityIsNotAlive);

        assert_matches!(world.remove_component(e, c), Err(RemoveComponentError::EntityIsNotAlive { .. }));
    }

    #[test]
    fn add_multiple_times() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_eq!(world.has_component(e, c), HasComponent::NotPresent);
        assert_matches!(world.add_component(e, c), Ok(AddComponent::Added));
        assert_eq!(world.has_component(e, c), HasComponent::Present);
        assert_matches!(world.add_component(e, c), Ok(AddComponent::AlreadyPresent));
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
        world.dispawn(c).unwrap();
        assert!(!world.alive(c));

        assert_matches!(world.has_component(e, c), HasComponent::ComponentIsNotAlive);
        assert_matches!(world.add_component(e, c), Err(AddComponentError::ComponentIsNotAlive { .. }));
    }

    #[test]
    fn with_dead_component() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert!(world.alive(c));
        world.dispawn(c).unwrap();
        assert!(!world.alive(c));

        assert_matches!(world.has_component(e, c), HasComponent::ComponentIsNotAlive);
        assert_matches!(world.add_component(e, c), Err(AddComponentError::ComponentIsNotAlive { .. }));
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
        
        assert_matches!(world.dispawn(e), Ok(()));

        let e2 = world.spawn();
        assert_ne!(e, e2);
        assert_eq!(e.index(), e2.index());
        assert_eq!(e.generation().next(), e2.generation());

        assert_matches!(world.get::<TestComponent1>(e), Err(GetComponentTypedError::EntityIsNotAlive { .. }));
        assert_matches!(world.get::<TestComponent1>(e2), Err(GetComponentTypedError::ComponentNotPresent { .. }));
        assert_matches!(world.add(e2, TestComponent1(52)), Ok(AddComponent::Added));
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

        assert_matches!(
            world.add_component(e, c),
            Err(AddComponentError::ComponentNeedsValue { .. }),
        );
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

mod world_try_clone {
    use dynvec::NoCloneError;

    use super::*;

    #[test]
    fn simple_empty() {
        let world = World::new();
        let _new_world = world.try_clone().unwrap();
    }

    #[test]
    fn simple_entity_alive() {
        let mut world = World::new();
        let entity = world.spawn();

        let new_world = world.try_clone().unwrap();
        assert!(world.alive(entity));
        assert!(new_world.alive(entity));

        world.dispawn(entity).unwrap();

        assert!(!world.alive(entity));
        assert!(new_world.alive(entity));
    }

    #[test]
    fn clonable_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, DefaultComponent("Feur".to_string())).unwrap();

        let new_world = world.try_clone().unwrap();

        assert_eq!(new_world.get::<DefaultComponent>(entity).unwrap(), &DefaultComponent("Feur".to_string()));
    }

    #[test]
    fn runtime_component() {
        let mut world = World::new();
        let entity = world.spawn();
        let c = world.spawn_component();

        world.add_component(entity, c).unwrap();

        let new_world = world.try_clone().unwrap();

        assert_eq!(new_world.has_component(entity, c), HasComponent::Present);
    }

    #[test]
    fn unclonable_component() {
        let mut world = World::new();
        let entity = world.spawn();

        world.add(entity, NotCloneComponent).unwrap();

        assert_matches!(world.try_clone(), Err(NoCloneError { .. }));
    }
}

mod drops_when_it_should {
    use super::*;

    #[test]
    fn drops_component_on_remove() {
        let mut world = World::new();
        let e = world.spawn();

        let checker = Arc::new(AtomicUsize::new(0));

        world.add(e, DropCheckComponent(Arc::clone(&checker))).unwrap();
        assert_eq!(checker.load(Ordering::Relaxed), 0);
        world.remove::<DropCheckComponent>(e).unwrap();
        assert_eq!(checker.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn drops_component_on_dispawn() {
        let mut world = World::new();
        let e = world.spawn();

        let checker = Arc::new(AtomicUsize::new(0));

        world.add(e, DropCheckComponent(Arc::clone(&checker))).unwrap();
        assert_eq!(checker.load(Ordering::Relaxed), 0);
        world.dispawn(e).unwrap();
        assert_eq!(checker.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn drops_component_on_world_drop() {
        let mut world = World::new();
        let e = world.spawn();

        let checker = Arc::new(AtomicUsize::new(0));

        world.add(e, DropCheckComponent(Arc::clone(&checker))).unwrap();
        assert_eq!(checker.load(Ordering::Relaxed), 0);
        drop(world);
        assert_eq!(checker.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn drops_component_on_unregister() {
        let mut world = World::new();
        let e = world.spawn();
        let c = world.component::<DropCheckComponent>();

        let checker = Arc::new(AtomicUsize::new(0));

        world.add(e, DropCheckComponent(Arc::clone(&checker))).unwrap();
        assert_eq!(checker.swap(0, Ordering::Relaxed), 0);
        world.dispawn(c).unwrap();
        assert_eq!(checker.swap(0, Ordering::Relaxed), 1);
        world.dispawn(e).unwrap();
        assert_eq!(checker.swap(0, Ordering::Relaxed), 0);
    }

    #[test]
    fn get_or_default() {
        let mut world = World::new();
        let e1 = world.spawn();
        let e2 = world.spawn();

        assert_eq!(
            world.get_or_default::<DefaultComponent>(e1).unwrap(),
            &mut DefaultComponent("default value".into()),
        );
        assert_eq!(
            world.get_or_default::<DefaultComponent>(e1).unwrap(),
            &mut DefaultComponent("default value".into()),
        );
        world.set(e2, DefaultComponent("Custom value".into())).unwrap();
        assert_eq!(
            world.get_or_default::<DefaultComponent>(e2).unwrap(),
            &mut DefaultComponent("Custom value".into()),
        );
    }
}

mod tables_and_archetyps {
    use super::*;

    #[test]
    fn simple_empty_at_start() {
        let mut world = World::new();
        let e = world.spawn();

        let empty_archtyp = world.archtyp_for(Cow::default());
        
        assert_eq!(
            world.entities_archetypes[e.index()],
            empty_archtyp,
        );
    }

    #[test]
    fn simple_adding_to_move() {
        let mut world = World::new();
        let e = world.spawn();

        let empty_archetyp = world.archtyp_for(Cow::default());
        let c = world.component::<TestComponent1>();
        let non_empty_archetyp = world.archtyp_for(Cow::Owned([c].into_iter().collect()));
        
        assert_eq!(world.entities_archetypes[e.index()], empty_archetyp);
        world.add(e, TestComponent1(42)).unwrap();
        assert_eq!(world.entities_archetypes[e.index()], non_empty_archetyp);
        world.remove::<TestComponent1>(e).unwrap();
        assert_eq!(world.entities_archetypes[e.index()], empty_archetyp);
    }

    #[test]
    fn move_after_unregister_component() {
        let mut world = World::new();
        let e = world.spawn();

        let empty_archetyp = world.archtyp_for(Cow::default());
        let c = world.component::<TestComponent1>();
        let non_empty_archetyp = world.archtyp_for(Cow::Owned([c].into_iter().collect()));

        assert_eq!(world.entities_archetypes[e.index()], empty_archetyp);
        world.add(e, TestComponent1(42)).unwrap();
        assert_eq!(world.entities_archetypes[e.index()], non_empty_archetyp);
        world.dispawn(c).unwrap();
        assert_eq!(world.entities_archetypes[e.index()], empty_archetyp);
    }

    #[test]
    fn non_storage_still_split_table() {
        trait TableOf {
            fn table_of(&self, entity: Entity) -> TableId;
        }

        impl TableOf for World {
            fn table_of(&self, entity: Entity) -> TableId {
                assert!(self.alive(entity));
                self.archetypes[self.entities_archetypes[entity.index()]].table_id
            }
        }

        let mut world = World::new();
        let e = world.spawn();
        let c = world.spawn_component();

        assert_matches!(world.component_storage(c), Some(ComponentStorageKind::None));

        let empty_archetyp = world.archtyp_for(Cow::default());
        let empty_table = world.archetypes[empty_archetyp].table_id;
        let non_empty_archetyp = world.archtyp_for(Cow::Owned([c].into_iter().collect()));
        let non_empty_table = world.archetypes[non_empty_archetyp].table_id;

        assert_eq!(world.table_of(e), empty_table);
        world.add_component(e, c).unwrap();
        assert_eq!(world.table_of(e), non_empty_table);
        world.remove_component(e, c).unwrap();
        assert_eq!(world.table_of(e), empty_table);
    }
}

mod traits {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct MyReadOnlyComponent(u32);

    impl Component for MyReadOnlyComponent {
        fn on_register(world: &mut World, entity: ComponentEntity) {
            world.add(entity, component_traits::ReadOnly).unwrap();
        }
    }

    #[test]
    fn readonly_registered_by_default() {
        let world = World::new();
        assert_matches!(world.try_component::<component_traits::ReadOnly>(), Some(_));
    }

    #[test]
    fn simple_readonly_registering() {
        let mut world = World::new();
        let c = world.component::<MyReadOnlyComponent>();
        assert_eq!(world.has::<component_traits::ReadOnly>(c), HasComponentTyped::Present);
    }

    #[test]
    fn simple_readonly_using_without_mutating() {
        let mut world = World::new();

        let e = world.spawn();

        world.add(e, MyReadOnlyComponent(42)).unwrap();
        assert_eq!(world.has::<MyReadOnlyComponent>(e), HasComponentTyped::Present);
        assert_matches!(world.get::<MyReadOnlyComponent>(e), Ok(&MyReadOnlyComponent(42)));
    }

    #[test]
    fn simple_readonly_fail_get_mut() {
        let mut world = World::new();

        let e = world.spawn();

        world.add(e, MyReadOnlyComponent(42)).unwrap();
        assert_eq!(world.has::<MyReadOnlyComponent>(e), HasComponentTyped::Present);
        assert_matches!(world.get_mut::<MyReadOnlyComponent>(e), Err(GetComponentTypedError::Forbidden { .. }));
        assert_matches!(world.get::<MyReadOnlyComponent>(e), Ok(&MyReadOnlyComponent(42)));
    }
}

mod queries {
    use super::*;

    use query as q;

    struct Ctx {
        world: World,
        components: Vec<Entity>,
        with_nothing: Vec<Entity>,
        with_test_1: Vec<Entity>,
        with_test_2: Vec<Entity>,
        with_both: Vec<Entity>,
    }

    fn create_ctx() -> Ctx {
        let mut world = World::new();

        // We try to include any internally created entity here
        let components = vec![
            world.component::<ComponentStorageComponent>().0,
            world.component::<TestComponent1>().0,
            world.component::<TestComponent2>().0,
        ];
        let with_nothing = (0..4).map(|_| world.spawn()).collect_vec();
        let with_test_1 = (0..3).map(|_| {
            let e = world.spawn();
            world.add(e, TestComponent1(42)).unwrap();
            e
        }).collect_vec();
        let with_test_2 = (0..2).map(|_| {
            let e = world.spawn();
            world.add(e, TestComponent2(88.)).unwrap();
            e
        }).collect_vec();
        let with_both = (0..2).map(|_| {
            let e = world.spawn();
            world.add(e, TestComponent1(50)).unwrap();
            world.add(e, TestComponent2(99.)).unwrap();
            e
        }).collect_vec();

        Ctx {
            world,
            components,
            with_nothing,
            with_test_1,
            with_test_2,
            with_both,
        }
    }

    #[test]
    fn always() {
        let ctx = create_ctx();
        let query = q::Query::<q::Always>::new(&ctx.world);
        assert_eq!(size_of::<q::Always>(), 0);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .sorted()
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.components).chain(ctx.with_nothing)
                .chain(ctx.with_test_1).chain(ctx.with_test_2)
                .chain(ctx.with_both)
            .collect_vec(),
        );
    }

    #[test]
    fn never() {
        let ctx = create_ctx();
        let query = q::Query::<q::Never>::new(&ctx.world);
        assert_eq!(size_of::<q::Never>(), 0);

        assert_eq!(
            query.entities(&ctx.world).collect_vec(),
            vec![],
        );
    }

    #[test]
    fn simple_has_1() {
        let ctx = create_ctx();
        let query = q::Query::<q::Has<TestComponent1>>::new(&ctx.world);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .sorted()
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.with_test_1).chain(ctx.with_both)
            .collect_vec(),
        );
    }

    #[test]
    fn simple_has_2() {
        let ctx = create_ctx();
        let query = q::Query::<q::Has<TestComponent2>>::new(&ctx.world);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .sorted()
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.with_test_2).chain(ctx.with_both)
            .collect_vec(),
        );
    }

    #[test]
    fn has_or() {
        let ctx = create_ctx();
        let query = q::Query::<q::Or<(q::Has<TestComponent1>, q::Has<TestComponent2>)>>::new(&ctx.world);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .sorted()
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.with_test_1).chain(ctx.with_test_2).chain(ctx.with_both)
            .collect_vec(),
        );
    }

    #[test]
    fn has_not() {
        let ctx = create_ctx();
        let query = q::Query::<q::Not<q::Has<TestComponent1>>>::new(&ctx.world);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .sorted()
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.components).chain(ctx.with_nothing).chain(ctx.with_test_2)
            .sorted().collect_vec(),
        );
    }

    #[test]
    fn has_xor() {
        let ctx = create_ctx();
        let query = q::Query::<q::Xor<(q::Has<TestComponent1>, q::Has<TestComponent2>)>>::new(&ctx.world);

        assert_eq!(
            query.entities(&ctx.world)
            .assert_is_sorted_by_key(|entity| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .collect_vec(),
            empty::<Entity>()
                .chain(ctx.with_test_1).chain(ctx.with_test_2)
            .collect_vec(),
        );
    }

    #[test]
    fn ref_simple_1() {
        let ctx = create_ctx();
        let query = q::Query::<q::And<(q::EntityHandle, q::Ref<TestComponent1>)>>::new(&ctx.world);

        assert_eq!(
            query.iter(&ctx.world)
            .assert_is_sorted_by_key(|&(entity, _)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .collect_vec(),
            empty::<(Entity, &TestComponent1)>()
                .chain(zip(ctx.with_test_1, repeat(&TestComponent1(42))))
                .chain(zip(ctx.with_both, repeat(&TestComponent1(50))))
            .collect_vec(),
        );
    }

    #[test]
    fn ref_simple_2() {
        let ctx = create_ctx();
        let query = q::Query::<q::And<(q::EntityHandle, q::Ref<TestComponent2>)>>::new(&ctx.world);

        assert_eq!(
            query.iter(&ctx.world)
            .assert_is_sorted_by_key(|&(entity, _)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .collect_vec(),
            empty::<(Entity, &TestComponent2)>()
                .chain(zip(ctx.with_test_2, repeat(&TestComponent2(88.))))
                .chain(zip(ctx.with_both, repeat(&TestComponent2(99.))))
            .collect_vec(),
        );
    }

    #[test]
    fn ref_and_has() {
        let ctx = create_ctx();
        let query = q::Query::<q::And<(q::EntityHandle, q::Has<TestComponent1>, q::Ref<TestComponent2>)>>::new(&ctx.world);

        assert_eq!(
            query.iter(&ctx.world)
            .assert_is_sorted_by_key(|&(entity, _, _)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .collect_vec(),
            empty::<(Entity, (), &TestComponent2)>()
                .chain(izip!(ctx.with_both, repeat(()), repeat(&TestComponent2(99.))))
            .collect_vec(),
        );
    }

    #[test]
    fn ref_and_not_has() {
        let ctx = create_ctx();
        let query = q::Query::<q::And<(q::EntityHandle, q::Not<q::Has<TestComponent1>>, q::Ref<TestComponent2>)>>::new(&ctx.world);

        assert_eq!(
            query.iter(&ctx.world)
            .assert_is_sorted_by_key(|&(entity, _, _)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
            .collect_vec(),
            empty::<(Entity, (), &TestComponent2)>()
                .chain(izip!(ctx.with_test_2, repeat(()), repeat(&TestComponent2(88.))))
            .collect_vec(),
        );
    }

    #[test]
    fn stays_up_to_date() {
        let mut ctx = create_ctx();
        let query = q::Query::<q::And<(q::EntityHandle, q::Ref<TestComponent1>)>>::new(&ctx.world);

        assert_eq!(
            query.iter(&ctx.world)
                .assert_is_sorted_by_key(|&(entity, ..)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
                .collect_vec(),
            empty::<(Entity, &TestComponent1)>()
                .chain(zip(ctx.with_test_1.iter().copied(), repeat(&TestComponent1(42))))
                .chain(zip(ctx.with_both.iter().copied(), repeat(&TestComponent1(50))))
            .collect_vec(),
        );

        let new_entity = ctx.world.spawn();
        ctx.world.add(new_entity, TestComponent1(88)).unwrap();

        assert_eq!(
            query.iter(&ctx.world)
                .assert_is_sorted_by_key(|&(entity, ..)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
                .collect_vec(),
            empty::<(Entity, &TestComponent1)>()
                .chain(zip(ctx.with_test_1.iter().copied(), repeat(&TestComponent1(42))))
                .chain(once((new_entity, &TestComponent1(88))))
                .chain(zip(ctx.with_both.iter().copied(), repeat(&TestComponent1(50))))
            .collect_vec(),
        );

        ctx.world.dispawn(new_entity).unwrap();

        assert_eq!(
            query.iter(&ctx.world)
                .assert_is_sorted_by_key(|&(entity, ..)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
                .collect_vec(),
            empty::<(Entity, &TestComponent1)>()
                .chain(zip(ctx.with_test_1.iter().copied(), repeat(&TestComponent1(42))))
                .chain(zip(ctx.with_both.iter().copied(), repeat(&TestComponent1(50))))
            .collect_vec(),
        );

        for &c in &ctx.with_test_1 {
            ctx.world.add(c, DefaultComponent("feur".into())).unwrap();
        }

        assert_eq!(
            query.iter(&ctx.world)
                .assert_is_sorted_by_key(|&(entity, ..)| (ctx.world.entities_archetypes[entity.index()], entity.index()))
                .collect_vec(),
            empty::<(Entity, &TestComponent1)>()
                .chain(zip(ctx.with_both.iter().copied(), repeat(&TestComponent1(50))))
                .chain(zip(ctx.with_test_1.iter().copied(), repeat(&TestComponent1(42))))
            .collect_vec(),
        );
    }
}
