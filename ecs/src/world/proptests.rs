use std::assert_matches::assert_matches;
use std::iter::{chain, repeat, zip};

use crate::world::*;
use crate::world_utils::*;

use utils::prelude::*;
use proptest::{prelude::*, strategy::LazyJust};
use proptest::strategy::Union;
use proptest_state_machine::{prop_state_machine, ReferenceStateMachine, StateMachineTest};

prop_state_machine! {
    #[test]
    fn run_world_state_machine_tests(
        sequential 1..20 => WorldMachine
    );
}

trait TestComponent: dyn_clone::DynClone + std::fmt::Debug + Any {
    fn dynvec_metadata(&self) -> DynVecMetadata;

    fn get_entity_from_world(&self, world: &mut World) -> ComponentEntity;
    fn try_get_entity_from_world(&self, world: &World) -> Option<ComponentEntity>;

    fn has_component_from_world(&self, world: &World, entity: Entity) -> HasComponentTyped;
    fn assert_eq_with_from_world(&self, world: &World, entity: Entity) -> Result<bool, GetComponentTypedError>;
    fn assert_get_is_err_from_world(&self, world: &World, entity: Entity) -> Result<(), GetComponentTypedError>;
    fn assert_eq_with_dynvec_ref(&self, dynref: DynVecValueRef) -> Result<bool, dynvec::IncorrectTypeError>;

    fn eq_with(&self, other: &dyn Any) -> bool;

    fn arbitrary_value(&self) -> Option<BoxedStrategy<Box<dyn TestComponent>>> {
        None
    }
}
dyn_clone::clone_trait_object!(TestComponent);

impl<T: Clone + Arbitrary + PartialEq + 'static> TestComponent for T {
    fn dynvec_metadata(&self) -> DynVecMetadata {
        DynVecMetadata::new::<Self>()
    }

    fn get_entity_from_world(&self, world: &mut World) -> ComponentEntity {
        world.component::<Self>()
    }

    fn try_get_entity_from_world(&self, world: &World) -> Option<ComponentEntity> {
        world.try_component::<Self>()
    }

    fn has_component_from_world(&self, world: &World, entity: Entity) -> HasComponentTyped {
        world.has::<Self>(entity)
    }

    fn assert_get_is_err_from_world(&self, world: &World, entity: Entity) -> Result<(), GetComponentTypedError> {
        world.get::<Self>(entity).map(|_| ())
    }

    fn assert_eq_with_from_world(&self, world: &World, entity: Entity) -> Result<bool, GetComponentTypedError> {
        Ok(self == world.get::<Self>(entity)?)
    }

    fn assert_eq_with_dynvec_ref(&self, dynref: DynVecValueRef) -> Result<bool, dynvec::IncorrectTypeError> {
        Ok(self == dynref.as_typed::<Self>()?)
    }

    fn eq_with(&self, other: &dyn Any) -> bool {
        let Some(other) = other.downcast_ref()
        else { return false };
        self == other
    }

    fn arbitrary_value(&self) -> Option<BoxedStrategy<Box<dyn TestComponent>>> {
        Some(
            any::<Self>()
                .prop_map(|comp| Box::new(comp) as Box<dyn TestComponent>)
                .boxed()
        )
    }
}

impl TestComponent for ComponentStorageComponent {
    fn dynvec_metadata(&self) -> DynVecMetadata {
        DynVecMetadata::new::<Self>()
    }

    fn get_entity_from_world(&self, world: &mut World) -> ComponentEntity {
        world.component::<Self>()
    }

    fn try_get_entity_from_world(&self, world: &World) -> Option<ComponentEntity> {
        world.try_component::<Self>()
    }

    fn has_component_from_world(&self, world: &World, entity: Entity) -> HasComponentTyped {
        world.has::<Self>(entity)
    }

    fn assert_get_is_err_from_world(&self, world: &World, entity: Entity) -> Result<(), GetComponentTypedError> {
        world.get::<Self>(entity).map(|_| ())
    }

    fn assert_eq_with_from_world(&self, world: &World, entity: Entity) -> Result<bool, GetComponentTypedError> {
        Ok(self.dynvec_meta.type_id == world.get::<Self>(entity)?.dynvec_meta.type_id)
    }

    fn assert_eq_with_dynvec_ref(&self, dynref: DynVecValueRef) -> Result<bool, dynvec::IncorrectTypeError> {
        Ok(self.dynvec_meta.type_id == dynref.as_typed::<Self>()?.dynvec_meta.type_id)
    }

    fn eq_with(&self, other: &dyn Any) -> bool {
        let Some(other) = other.downcast_ref::<Self>()
        else { return false };
        self.dynvec_meta.type_id == other.dynvec_meta.type_id
    }
}

pub struct ReferenceWorld;

#[derive(Clone)]
pub struct ReferenceWorldState {
    typed_components_references: Vec<Box<dyn TestComponent>>,

    /// Only stores components `typed_components_references` that are expected
    /// to be registered in the World
    component_entities: HashMap<TypeId, Uid>,
    /// Component with value None means the entity has the component but
    /// it has no storage.
    entities: HashMap<Uid, HashMap<Uid, Option<Box<dyn TestComponent>>>>,
    dead_entities: Vec<Uid>,
}

impl ReferenceWorldState {
    fn component_label_from_uid(&self, component_entity_uid: Uid) -> &'static str {
        self.component_entities.iter()
            .find(|&(_, &uid)| uid == component_entity_uid)
            .map(|(&type_id, _)| self.typed_components_references.iter()
                .find(|reference| reference.dynvec_metadata().type_id == type_id).unwrap())
            .map(|reference| reference.dynvec_metadata().type_name)
            .unwrap_or("Runtime component")
    }
}

impl std::fmt::Debug for ReferenceWorldState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReferenceWorldState")
            .field("component_entities", &self.component_entities)
            .field("entities", &self.entities.iter()
                .map(|(entity, comps)| (entity, comps.iter()
                    .map(|(&uid, comp)| (uid, comp.as_ref().map(|c| c.dynvec_metadata().type_name)))
                    .collect_vec()))
                .collect_vec())
            .finish()
    }
}

#[derive(Clone, Debug)]
pub enum Transition {
    SpawnEntity(Uid),
    SpawnComponentEntity(Uid),
    DispawnAliveEntity(Uid),
}

// Implementation of the reference state machine that drives the test. That is,
// it's used to generate a sequence of transitions the `StateMachineTest`.
impl ReferenceStateMachine for ReferenceWorld {
    type State = ReferenceWorldState;
    type Transition = Transition;

    fn init_state() -> BoxedStrategy<Self::State> {
        let ccid = Uid::new();
        Just(ReferenceWorldState {
            typed_components_references: [
                Box::new(ComponentStorageComponent { dynvec_meta: DynVecMetadata::new::<()>() /* dummy value */ }) as Box<dyn TestComponent>,
            ].into(),

            component_entities: [
                (TypeId::of::<ComponentStorageComponent>(), ccid),
            ].into(),
            entities: [
                (ccid, [(ccid, Some(Box::new(ComponentStorageComponent { dynvec_meta: DynVecMetadata::new::<ComponentStorageComponent>() }) as Box<dyn TestComponent>))].into()),
            ].into(),
            dead_entities: vec![],
        }).boxed()
    }

    fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
        let ccuid = state.component_entities[&TypeId::of::<ComponentStorageComponent>()];

        let mut options = Vec::<BoxedStrategy<Self::Transition>>::new();

        options.extend([
            LazyJust::new(Uid::new).prop_map(|uid| Transition::SpawnEntity(uid)).boxed(),
            LazyJust::new(Uid::new).prop_map(|uid| Transition::SpawnComponentEntity(uid)).boxed(),
        ]);

        if state.entities.len() > 1 {
            options.push(
                Union::new(state.entities.keys().copied()
                    .filter(|&e| e != ccuid /* Cannot dispawn the component storage component */)
                    .map(Just))
                    .prop_map(|entity| Transition::DispawnAliveEntity(entity))
                    .boxed()
            );
        }

        Union::new(options).boxed()
    }

    fn apply(
        mut state: Self::State,
        transition: &Self::Transition,
    ) -> Self::State {
        // Asserts here are just to catch any bug in the test implementation lol
        match transition {
            &Transition::SpawnEntity(uid) | &Transition::SpawnComponentEntity(uid) => {
                assert!(state.entities.insert(uid, default()).is_none());
            },
            &Transition::DispawnAliveEntity(uid) => {
                assert!(state.entities.remove(&uid).is_some());
                state.dead_entities.push(uid);

                // 'unregister' the entity as a component
                state.component_entities.retain(|&_k, &mut v| v != uid);
                for components in state.entities.values_mut() {
                    components.remove(&uid);
                }
            },
        }
        state
    }

    fn preconditions(
        state: &Self::State,
        transition: &Self::Transition,
    ) -> bool {
        match transition {
            Transition::SpawnEntity(uid) | Transition::SpawnComponentEntity(uid)
                => !state.entities.contains_key(uid) && !state.dead_entities.contains(uid),
            Transition::DispawnAliveEntity(uid)
                => state.entities.contains_key(&uid),
        }
    }
}

struct WorldMachine {
    world: World,
    /// Entities and uids are never removed from this
    entity_mapping_u2e: HashMap<Uid, Entity>,
    /// Entities and uids are never removed from this
    entity_mapping_e2u: HashMap<Entity, Uid>,
}

impl StateMachineTest for WorldMachine {
    type SystemUnderTest = WorldMachine;
    type Reference = ReferenceWorld;

    fn init_test(
        ref_state: &<Self::Reference as ReferenceStateMachine>::State,
    ) -> Self::SystemUnderTest {
        let world = World::new();

        assert_eq!(ref_state.component_entities.len(), 1);
        assert_eq!(ref_state.entities.len(), 1);

        let ccuid = ref_state.component_entities[&TypeId::of::<ComponentStorageComponent>()];
        let cce = world.try_component::<ComponentStorageComponent>()
            .expect("the ComponentStorageComponent should be registered from the start")
            .0;

        WorldMachine {
            world,
            entity_mapping_u2e: [
                (ccuid, cce),
            ].into(),
            entity_mapping_e2u: [
                (cce, ccuid),
            ].into(),
        }
    }

    fn apply(
        mut state: Self::SystemUnderTest,
        _ref_state: &<Self::Reference as ReferenceStateMachine>::State,
        transition: Transition,
    ) -> Self::SystemUnderTest {
        match transition {
            Transition::SpawnEntity(uid) => {
                let entity = state.world.spawn();
                assert!(state.entity_mapping_u2e.insert(uid, entity).is_none(), "Error in test implementation");
                assert!(state.entity_mapping_e2u.insert(entity, uid).is_none(), "Same entity returned multiple times from World::spawn");
            },
            Transition::SpawnComponentEntity(uid) => {
                let entity = state.world.spawn_component().0;
                assert!(state.entity_mapping_u2e.insert(uid, entity).is_none(), "Error in test implementation");
                assert!(state.entity_mapping_e2u.insert(entity, uid).is_none(), "Same entity returned multiple times from World::spawn");
            },
            Transition::DispawnAliveEntity(uid) => {
                let &entity = state.entity_mapping_u2e.get(&uid).unwrap();
                state.world.dispawn(entity).unwrap();
            },
        }
        state
    }

    fn check_invariants(
        state: &Self::SystemUnderTest,
        ref_state: &<Self::Reference as ReferenceStateMachine>::State,
    ) {
        // Asserts that component that should be registered are registered
        // and inversely with World::try_component
        for reference in ref_state.typed_components_references.iter() {
            // Calls World::try_component
            let entity = reference.try_get_entity_from_world(&state.world);
            let expected_entity = ref_state.component_entities.get(&reference.dynvec_metadata().type_id)
                .map(|uid| *state.entity_mapping_u2e.get(&uid).unwrap());

            let component_label = reference.dynvec_metadata().type_name;

            match (entity, expected_entity) {
                (None, None) => (),
                (None, Some(_)) => panic!("Component '{component_label}' was expected to be registered was not returned from World::try_component"),
                (Some(_), None) => panic!("Component '{component_label}' was *not* expected to be registered was returned with an entity from World::try_component"),
                (Some(ComponentEntity(gotten)), Some(expected)) => {
                    assert_eq!(gotten, expected, "Wrong entity returned from World::try_component<{component_label}>");
                },
            }
        }

        // Asserts that entities have the component we expect them to have
        // and not the one we don't expect them to have
        for (entity_uid, components) in ref_state.entities.iter()
            .map(|(uid, comps)| (*uid, Some(comps)))
            .chain(ref_state.dead_entities.iter().map(|uid| (*uid, None)))
        {
            let default_comps = Default::default();
            let entity_expected_to_be_alive = components.is_some();
            let components = components.unwrap_or(&default_comps);
            let entity = *state.entity_mapping_u2e.get(&entity_uid).unwrap();

            if entity_expected_to_be_alive {
                assert_eq!(state.world.generation_at_index(entity.index()), entity.generation());
            }
            else {
                // We don't do enough insertion to make the generation wrap anyway
                assert!(state.world.generation_at_index(entity.index()) > entity.generation());
            }

            assert_eq!(state.world.alive(entity), entity_expected_to_be_alive);

            // any entity can be a component so we check all entities
            // Test with untyped world apis
            // which are:
            //  - World::get_component
            //  - World::has_component
            for (is_component_alive, &component_entity_uid) in
                chain(zip(repeat(true), ref_state.entities.keys()), zip(repeat(false), ref_state.dead_entities.iter()))
            {
                let component_entity = ComponentEntity(*state.entity_mapping_u2e.get(&component_entity_uid).unwrap());
                let expected_value = components.get(&component_entity_uid).map(Option::as_ref);
                let component_label = ref_state.component_label_from_uid(component_entity_uid);

                match (expected_value, state.world.has_component(entity, component_entity)) {
                    (None, HasComponent::Present) => panic!("World::has_component returned Present but the component '{component_label}' is expected to be absent of this entity"),
                    (Some(_), HasComponent::NotPresent) => panic!("World::has_component returned NotPresent but the component '{component_label}' is expected to be present in this entity"),

                    (_, HasComponent::EntityIsNotAlive) => assert!(!entity_expected_to_be_alive),
                    (_, HasComponent::ComponentIsNotAlive) => assert!(!is_component_alive),

                    (None, HasComponent::NotPresent) |
                    (Some(_), HasComponent::Present) => assert!(entity_expected_to_be_alive && is_component_alive),
                }
                
                let actual_value = match state.world.get_component(entity, component_entity) {
                    Ok(value) => Some(Some(value)),
                    Err(GetComponentError::EntityIsNotAlive { entity: e }) => {
                        assert_eq!(e, entity);
                        assert!(!entity_expected_to_be_alive);
                        None
                    },
                    Err(GetComponentError::ComponentIsNotAlive { component: c }) => {
                        assert_eq!(c, component_entity);
                        assert!(!is_component_alive);
                        None
                    },
                    Err(GetComponentError::ComponentNotPresent { component: c, entity: e }) => {
                        assert_eq!(c, component_entity);
                        assert_eq!(e, entity);

                        assert!(entity_expected_to_be_alive);
                        assert!(is_component_alive);
                        None
                    },
                    Err(GetComponentError::ComponentHasNoStorage { component: c, entity: e }) => {
                        assert_eq!(c, component_entity);
                        assert_eq!(e, entity);

                        assert!(entity_expected_to_be_alive);
                        assert!(is_component_alive);
                        Some(None)
                    },
                };

                match (expected_value, actual_value) {
                    (Some(Some(expected)), Some(Some(gotten))) => {
                        assert!(expected.eq_with(gotten.as_any()));
                        assert_matches!(expected.assert_eq_with_dynvec_ref(gotten), Ok(true));
                    },
                    (None, None) | (Some(None), Some(None)) => (),
                    (expected, got) => panic!("Expected component value {expected:?} but got {:?}", got.map(|p| p.map(|r| r.as_any()))),
                }
            }

            // Test with typed world apis
            // which are:
            //  - World::get
            //  - World::has
            for reference in &ref_state.typed_components_references {
                let type_id = reference.dynvec_metadata().type_id;
                let component_entity_uid = *ref_state.component_entities.get(&type_id).unwrap();
                let component_entity = state.entity_mapping_u2e.get(&component_entity_uid).copied();
                let expect_component_to_be_registered = component_entity.is_some();

                let expected_value = components.get(&component_entity_uid).map(Option::as_ref);

                match reference.has_component_from_world(&state.world, entity) {
                    HasComponentTyped::EntityIsNotAlive => assert!(!entity_expected_to_be_alive),
                    HasComponentTyped::UnknownComponent => assert!(!expect_component_to_be_registered),
                    HasComponentTyped::NotPresent => {
                        assert!(expected_value.is_none());
                        assert!(entity_expected_to_be_alive);
                        assert!(expect_component_to_be_registered);
                    },
                    HasComponentTyped::Present => {
                        assert!(expected_value.is_some());
                        assert!(entity_expected_to_be_alive);
                        assert!(expect_component_to_be_registered);
                    },
                }

                match expected_value {
                    Some(Some(expected)) => assert!(expected.assert_eq_with_from_world(&state.world, entity).expect("Both entities are expected to exist")),
                    Some(None) => unreachable!("It is not possible for a typed value to have None storage, this is a problem with the Test"),
                    None => match reference.assert_get_is_err_from_world(&state.world, entity) {
                        Ok(()) => panic!("Expected component to not be present but got a value"),
                        Err(GetComponentTypedError::EntityIsNotAlive { entity: e }) => {
                            assert_eq!(e, entity);
                            assert!(!entity_expected_to_be_alive);
                        },
                        Err(GetComponentTypedError::UnknownComponent { type_name }) => {
                            assert_eq!(type_name, reference.dynvec_metadata().type_name);
                            assert!(!expect_component_to_be_registered);
                        },
                        Err(GetComponentTypedError::ComponentNotPresent { type_name, entity: e }) => {
                            assert_eq!(type_name, reference.dynvec_metadata().type_name);
                            assert_eq!(e, entity);
                            assert!(entity_expected_to_be_alive);
                            assert!(expect_component_to_be_registered);
                        },
                    },
                }
            }
        }
    }
}
