use crate::{ component::ComponentComponent, world::{EntityGeneration, EntityIndex} };

use super::{ World, Entity, RESERVED_ENTITY_COUNT };

use std::{alloc::Layout, any::TypeId};

#[test]
fn spawn_produces_sequential_indices_after_reserved_and_alive() {
    let mut w = World::new();

    let e0 = w.spawn();
    let e1 = w.spawn();
    let e2 = w.spawn();

    assert_eq!(e0.index(), RESERVED_ENTITY_COUNT);
    assert_eq!(e1.index(), RESERVED_ENTITY_COUNT + 1);
    assert_eq!(e2.index(), RESERVED_ENTITY_COUNT + 2);

    assert_eq!(e0.generation(), 0);
    assert_eq!(e1.generation(), 0);
    assert_eq!(e2.generation(), 0);

    assert!(w.alive(e0));
    assert!(w.alive(e1));
    assert!(w.alive(e2));
}

#[test]
fn dispawn_makes_entity_dead_and_double_despawn_false() {
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
    // Invalid index well beyond allocated range
    let invalid = Entity::new(EntityIndex(42), EntityGeneration::FIRST);
    assert!(!w.dispawn(invalid));

    // Stale entity after despawn
    let e = w.spawn();
    assert!(w.dispawn(e));
    // Old handle should now be stale
    assert!(!w.dispawn(e));
}

#[test]
fn component_registration_creates_entity_with_componentcomponent() {
    #[derive(Debug)]
    struct Foo;

    let mut w = World::new();
    let comp_entity = w.component::<Foo>();
    assert!(w.alive(comp_entity));

    // Same call returns the same entity
    let comp_entity2 = w.component::<Foo>();
    assert_eq!(comp_entity.index(), comp_entity2.index());

    // The component entity must carry ComponentComponent describing Foo
    let meta = w.get::<ComponentComponent>(comp_entity).expect("has meta");
    assert_eq!(meta.dynvec_meta.type_id, TypeId::of::<Foo>());
    assert_eq!(meta.dynvec_meta.layout(), Layout::new::<Foo>());
}

#[test]
fn archetypes_and_tables_created_and_reused_on_moves() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(u32);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct B(u32);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct C(u32);

    let mut w = World::new();
    let e1 = w.spawn();
    let e2 = w.spawn();

    // Start from empty archetype for each entity
    assert!(!w.has::<A>(e1).bool());
    assert!(!w.has::<B>(e1).bool());
    assert!(!w.has::<A>(e2).bool());
    assert!(!w.has::<B>(e2).bool());

    // Move e1 to archetype {A}
    let add_a_e1 = w.add(e1, A(1));
    assert!(add_a_e1.was_added);
    assert!(w.has::<A>(e1).bool());
    assert_eq!(w.get::<A>(e1).unwrap().0, 1);

    // Move e2 to archetype {B}
    let add_b_e2 = w.add(e2, B(2));
    assert!(add_b_e2.was_added);
    assert!(w.has::<B>(e2).bool());
    assert_eq!(w.get::<B>(e2).unwrap().0, 2);

    // Move e1 to a not-yet-existing archetype {A,B}
    let add_b_e1 = w.add(e1, B(20));
    assert!(add_b_e1.was_added, "inserting new component should mark was_added");
    assert!(w.has::<A>(e1).bool() && w.has::<B>(e1).bool());
    assert_eq!(w.get::<A>(e1).unwrap().0, 1);
    assert_eq!(w.get::<B>(e1).unwrap().0, 20);

    // Move e2 to an already existing archetype {A,B}
    let add_a_e2 = w.add(e2, A(10));
    assert!(add_a_e2.was_added);
    assert!(w.has::<A>(e2).bool() && w.has::<B>(e2).bool());
    assert_eq!(w.get::<A>(e2).unwrap().0, 10);
    assert_eq!(w.get::<B>(e2).unwrap().0, 2);

    // Move e2 further to a new archetype {A,B,C}
    let add_c_e2 = w.add(e2, C(7));
    assert!(add_c_e2.was_added);
    assert!(w.has::<C>(e2).bool());
    assert!(!w.has::<C>(e1).bool());

    // Ensure add again does not mark was_added and we can mutate via get_mut
    let add_again = w.add(e1, A(999)); // already had A
    assert!(!add_again.was_added);
    let before = w.get::<A>(e1).unwrap().0;
    w.get_mut::<A>(e1).unwrap().0 = before + 1;
    assert_eq!(w.get::<A>(e1).unwrap().0, before + 1);
}

#[test]
fn get_or_default_inserts_once_then_reuses() {
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    struct D(i32);

    let mut w = World::new();
    let e = w.spawn();

    // First call inserts default
    let add1 = w.get_or_default::<D>(e);
    assert!(add1.was_added);
    assert_eq!(add1.r#ref.0, 0);

    // Second call should not insert again
    let add2 = w.get_or_default::<D>(e);
    assert!(!add2.was_added);
    assert_eq!(add2.r#ref.0, 0);
}

// ----------------------
// Additional comprehensive tests
// ----------------------

#[test]
fn try_component_none_before_use_then_some_after_add() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct E(u8);

    let mut w = World::new();
    let e = w.spawn();

    // Not registered yet
    assert!(w.try_component::<E>().is_none());
    assert!(w.get::<E>(e).is_err());

    // Using add registers the component type
    w.add(e, E(5));
    assert!(w.try_component::<E>().is_some());
    assert_eq!(w.get::<E>(e).unwrap().0, 5);
}

#[test]
fn remove_nonexistent_component_returns_none_and_no_change() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct B(i32);

    let mut w = World::new();
    let e = w.spawn();

    w.add(e, A(1));
    assert!(w.has::<A>(e).bool());
    assert!(!w.has::<B>(e).bool());

    // Removing B should be a no-op
    assert_eq!(w.remove::<B>(e), None);
    assert!(w.has::<A>(e).bool());
    assert!(!w.has::<B>(e).bool());
    assert_eq!(w.get::<A>(e).unwrap().0, 1);
}

#[test]
fn remove_present_component_returns_value_and_entity_loses_component() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let mut w = World::new();
    let e = w.spawn();
    w.add(e, A(42));

    let removed = w.remove::<A>(e);
    assert_eq!(removed, Some(A(42)));
    assert!(!w.has::<A>(e).bool());
    assert!(w.get::<A>(e).is_err());
}

#[test]
fn readding_after_remove_inserts_again_with_new_value() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let mut w = World::new();
    let e = w.spawn();
    w.add(e, A(1));
    let _ = w.remove::<A>(e);
    assert!(!w.has::<A>(e).bool());

    let add_again = w.add(e, A(7));
    assert!(add_again.was_added);
    assert_eq!(w.get::<A>(e).unwrap().0, 7);
}

#[test]
fn add_order_does_not_change_end_state() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(u8);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct B(u8);

    let mut w = World::new();
    let x = w.spawn();
    let y = w.spawn();

    // Add in A then B order
    w.add(x, A(1));
    w.add(x, B(2));

    // Add in B then A order
    w.add(y, B(2));
    w.add(y, A(1));

    assert!(w.has::<A>(x).bool() && w.has::<B>(x).bool());
    assert!(w.has::<A>(y).bool() && w.has::<B>(y).bool());
    assert_eq!(w.get::<A>(x).unwrap().0, w.get::<A>(y).unwrap().0);
    assert_eq!(w.get::<B>(x).unwrap().0, w.get::<B>(y).unwrap().0);
}

#[test]
fn value_persists_when_moving_between_archetypes() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct B(i32);

    let mut w = World::new();
    let e = w.spawn();

    w.add(e, A(10));
    // mutate via get_mut
    w.get_mut::<A>(e).unwrap().0 = 11;
    // add B, forcing a move to a new archetype
    w.add(e, B(1));

    // A should still be 11
    assert_eq!(w.get::<A>(e).unwrap().0, 11);
    assert_eq!(w.get::<B>(e).unwrap().0, 1);
}

#[test]
fn spawn_despawn_reuse_index_does_not_leak_previous_components() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(u32);

    let mut w = World::new();
    let e1 = w.spawn();
    w.add(e1, A(99));
    assert!(w.has::<A>(e1).bool());
    assert!(w.dispawn(e1));

    // Reuse index for a fresh entity
    let e2 = w.spawn();
    assert_eq!(e1.index(), e2.index());
    assert_ne!(e1.generation(), e2.generation());

    // A should not be visible on the new entity until explicitly added
    assert!(!w.has::<A>(e2).bool());
    assert!(w.get::<A>(e2).is_err());
}

#[test]
fn has_get_get_mut_remove_on_dead_entity_behave_safely() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let mut w = World::new();
    let e = w.spawn();
    w.add(e, A(1));
    assert!(w.dispawn(e));

    // Dead entities should not appear to have components
    assert!(!w.has::<A>(e).bool());
    // Accessors should return error on dead entities
    assert!(w.get::<A>(e).is_err());
    assert!(w.get_mut::<A>(e).is_err());
    // Removing from dead should be a no-op
    assert!(w.remove::<A>(e).is_none());
}

#[test]
#[should_panic]
fn add_component_to_dead_entity_should_panic() {
    // It is expected that adding to a dead entity is invalid and should panic.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let mut w = World::new();
    let e = w.spawn();
    assert!(w.dispawn(e));
    let _ = w.add(e, A(3));
}

#[test]
fn access_with_invalid_entity_is_safe() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let w = &mut World::new();
    // An entity index far beyond any allocated range
    let invalid = Entity::new(EntityIndex(1_000_000), EntityGeneration::FIRST);

    // Expected safe behavior: has/get/get_mut/remove should not panic and indicate absence
    assert!(!w.has::<A>(invalid).bool());
    assert!(w.get::<A>(invalid).is_err());
    assert!(w.get_mut::<A>(invalid).is_err());
    assert!(w.remove::<A>(invalid).is_none());
}

#[test]
fn get_mut_unknown_component_errors() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Z(i32);

    let mut w = World::new();
    let e = w.spawn();
    let err = w.get_mut::<Z>(e).unwrap_err();
    // Unknown component path should be taken for non-registered types
    assert!(matches!(err, super::GetComponentError::UnknownComponent { .. }));
}

#[test]
fn world_default_constructs_and_works() {
    let mut w: World = Default::default();
    let e = w.spawn();
    assert!(w.alive(e));
}

#[test]
fn get_mut_registered_but_not_present_returns_component_not_present() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(i32);

    let mut w = World::new();
    let e = w.spawn();
    // Register component type without attaching to entity
    let _ = w.component::<A>();
    let err = w.get_mut::<A>(e).unwrap_err();
    assert!(matches!(err, super::GetComponentError::ComponentNotPresent { .. }));
}

#[test]
fn internal_table_for_cache_hit_and_id_defaults_are_exercised() {
    use std::borrow::Cow;
    // Access private items in parent module via `super::` path
    let mut w = World::new();

    // Create a component entity and build a component set
    #[derive(Debug)] struct X;
    let x_entity = w.component::<X>();
    let set = super::EntitySet::from(&[x_entity][..]);

    // First call creates the table (miss path)
    let _t0: super::TableId = w.table_for(Cow::Owned(set.clone()));
    // Second call should hit the cache (Some branch)
    let _t1: super::TableId = w.table_for(Cow::Borrowed(&set));

    // Touch defaults of internal id wrappers
    let _aid: super::ArchetypId = Default::default();
    let _tid: super::TableId = Default::default();
}

#[test]
fn many_entities_heterogeneous_components_stay_isolated() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct A(u8);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct B(u8);
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct C(u8);

    let mut w = World::new();
    let e1 = w.spawn(); // A
    let e2 = w.spawn(); // B
    let e3 = w.spawn(); // A,B
    let e4 = w.spawn(); // C
    let e5 = w.spawn(); // none

    w.add(e1, A(1));
    w.add(e2, B(2));
    w.add(e3, A(10));
    w.add(e3, B(20));
    w.add(e4, C(3));

    assert_eq!(w.get::<A>(e1).unwrap().0, 1);
    assert!(w.get::<B>(e1).is_err());
    assert_eq!(w.get::<B>(e2).unwrap().0, 2);
    assert!(w.get::<A>(e2).is_err());
    assert_eq!(w.get::<A>(e3).unwrap().0, 10);
    assert_eq!(w.get::<B>(e3).unwrap().0, 20);
    assert_eq!(w.get::<C>(e4).unwrap().0, 3);
    assert!(w.get::<A>(e5).is_err());
    assert!(w.get::<B>(e5).is_err());
    assert!(w.get::<C>(e5).is_err());

    // Remove B from e3, ensure isolation
    let _ = w.remove::<B>(e3);
    assert!(w.get::<B>(e3).is_err());
    assert_eq!(w.get::<A>(e3).unwrap().0, 10);
    assert_eq!(w.get::<B>(e2).unwrap().0, 2);
}

#[test]
fn component_entity_is_registered_and_carries_metadata() {
    #[derive(Debug)]
    struct FooBar;

    let mut w = World::new();
    // Before registration: try_component is None
    assert!(w.try_component::<FooBar>().is_none());

    let comp_entity = w.component::<FooBar>();
    assert!(w.alive(comp_entity));
    let cc = w.get::<ComponentComponent>(comp_entity).expect("has ComponentComponent");
    assert_eq!(cc.dynvec_meta.type_id, TypeId::of::<FooBar>());
    assert_eq!(cc.dynvec_meta.layout(), Layout::new::<FooBar>());
}

#[test]
fn removing_componentcomponent_from_component_entity_is_disallowed() {
    // It should not be possible to remove the metadata from the component entity.
    // Expected: None (no-op) or panic; we assert None here.
    let mut w = World::new();
    let comp_entity = w.component::<u32>();
    assert!(w.remove::<ComponentComponent>(comp_entity).is_none());
}
