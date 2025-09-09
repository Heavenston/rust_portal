use std::assert_matches::assert_matches;
use std::iter::{chain, repeat, zip};
use std::sync::Arc;

use crate::world::*;
use crate::world_utils::*;

use proptest::sample::select;
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

    fn arbitrary_value(&self) -> BoxedStrategy<Box<dyn TestComponent>>;
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

    fn arbitrary_value(&self) -> BoxedStrategy<Box<dyn TestComponent>> {
        any::<Self>()
            .prop_map(|comp| Box::new(comp) as Box<dyn TestComponent>)
            .boxed()
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

    fn arbitrary_value(&self) -> BoxedStrategy<Box<dyn TestComponent>> {
        select(vec![
            Self { dynvec_meta: DynVecMetadata::new::<ComponentStorageComponent>() },
            Self { dynvec_meta: DynVecMetadata::new::<TestComponent1>() },
            Self { dynvec_meta: DynVecMetadata::new::<TestComponent2>() },
            Self { dynvec_meta: DynVecMetadata::new::<TestComponentZST>() },
        ]).prop_map(|value| Box::new(value) as Box<dyn TestComponent>).boxed()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
struct TestComponent1(f32);

impl Arbitrary for TestComponent1 {
    type Parameters = <f32 as Arbitrary>::Parameters;
    type Strategy = impl Strategy<Value = Self>;

    fn arbitrary_with(parameters: Self::Parameters) -> Self::Strategy {
        f32::arbitrary_with(parameters).prop_map(|value| Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TestComponent2(u32);

impl Arbitrary for TestComponent2 {
    type Parameters = <u32 as Arbitrary>::Parameters;
    type Strategy = impl Strategy<Value = Self>;

    fn arbitrary_with(parameters: Self::Parameters) -> Self::Strategy {
        u32::arbitrary_with(parameters).prop_map(|value| Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct TestComponentZST;

impl Arbitrary for TestComponentZST {
    type Parameters = ();
    type Strategy = Just<Self>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        Just(Self)
    }
}

struct ReferenceWorld;

#[derive(Clone)]
struct ReferenceWorldState {
    typed_components_references: Box<[Box<dyn TestComponent>]>,

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

    /// Assumes the entity is alive
    fn get_entity_storage(&self, entity: Uid) -> Option<usize> {
        let components = self.entities.get(&entity).unwrap();
        let ccuid = self.component_entities[&TypeId::of::<ComponentStorageComponent>()];
        let component = components.get(&ccuid)?.as_ref().expect("It has storage and cannot lose it");
        let any_component: &dyn Any = component.as_ref();
        let storage = any_component.downcast_ref::<ComponentStorageComponent>().unwrap();

        let idx = self.typed_components_references.iter()
            .position(|p| p.dynvec_metadata().type_id == storage.dynvec_meta.type_id)
            .unwrap();
        Some(idx)
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
enum AddingComponentValue {
    Add {
        registered_component_entity_uid: Uid,
        value: Box<dyn TestComponent>,
        // true if World::set should be used instead of World::add
        use_set: bool,
    },
    AddComponentWith {
        component_entity_uid: Uid,
        value: Box<dyn TestComponent>,
    },
    AddComponent {
        component_entity_uid: Uid,
    },
}

#[derive(Clone, Debug)]
enum AddComponentResult {
    Success { added: bool, },
    EntityIsNotAlive,
    ComponentIsNotAlive,
    ComponentNeedsValue,
    TypeMismatched,
    ComponentDoesNotHaveStorage,
}

#[derive(Clone, Debug)]
enum RemovingComponentWay {
    Typed {
        reference: Box<dyn TestComponent>,
    },
    Untyped {
        component_entity_uid: Uid,
    },
}

#[derive(Clone, Debug)]
enum RemoveComponentResult {
    Success { value: Box<dyn TestComponent> },
    EntityIsNotAlive,
    ComponentIsNotAlive,
    UnknownComponent,
    ComponentNotPresent,
    Forbidden,
}

#[derive(Clone, Debug)]
enum Transition {
    SpawnEntity {
        uid: Uid,
        /// Wether to use World::spawn_component
        /// or just World::spawn
        use_spawn_component: bool,
    },
    DispawnEntity {
        uid: Uid,
        was_alive: bool,
    },
    RegisterComponent {
        was_already_registered: bool,
        expected_component_uid: Uid,
        reference_idx: usize,
    },
    AddComponentToEntity {
        entity_uid: Uid,
        value: AddingComponentValue,
        expected_one_of_results: Vec<AddComponentResult>,
    },
    RemoveComponentFromEntity {
        entity_uid: Uid,
        component: RemovingComponentWay,
        expected: Box<dyn TestComponent>,
    },
}

// Implementation of the reference state machine that drives the test. That is,
// it's used to generate a sequence of transitions the `StateMachineTest`.
impl ReferenceStateMachine for ReferenceWorld {
    type State = Arc<ReferenceWorldState>;
    type Transition = Transition;

    fn init_state() -> BoxedStrategy<Self::State> {
        let ccid = Uid::new();
        Just(Arc::new(ReferenceWorldState {
            typed_components_references: [
                Box::new(ComponentStorageComponent { dynvec_meta: DynVecMetadata::new::<()>() /* dummy value */ }) as Box<dyn TestComponent>,
                Box::new(TestComponent1(0.)) as Box<dyn TestComponent>,
                Box::new(TestComponent2(0)) as Box<dyn TestComponent>,
                Box::new(TestComponentZST) as Box<dyn TestComponent>,
            ].into(),

            // ComponentStorageComponent is the only component expected to be
            // registered automatically with the creation of the World
            component_entities: [
                (TypeId::of::<ComponentStorageComponent>(), ccid),
            ].into(),
            entities: [
                (ccid, [(ccid, Some(Box::new(ComponentStorageComponent { dynvec_meta: DynVecMetadata::new::<ComponentStorageComponent>() }) as Box<dyn TestComponent>))].into()),
            ].into(),
            dead_entities: vec![],
        })).boxed()
    }

    fn transitions(state: &Self::State) -> BoxedStrategy<Self::Transition> {
        let alive_entity_strategy = (!state.entities.is_empty())
            .then(|| Union::new(state.entities.keys().copied().map(Just)));
        let dead_entity_strategy = (!state.dead_entities.is_empty())
            .then(|| Union::new(state.dead_entities.iter().copied().map(Just)));
        let alive_or_dead_entity_trategy = match (
            alive_entity_strategy.clone().zip(Some(Just(true))), 
            dead_entity_strategy.clone().zip(Some(Just(false)))
        ) {
            (None, None) => None,
            (None, Some(dead)) => Some(Union::new([dead])),
            (Some(alive), None) => Some(Union::new([alive])),
            (Some(alive), Some(dead)) => Some(Union::new([alive, dead])),
        };

        let mut options = Vec::<BoxedStrategy<Self::Transition>>::new();

        // Entity spawn
        options.push(
            (LazyJust::new(Uid::new), any::<bool>())
                .prop_map(|(uid, use_spawn_component)| Transition::SpawnEntity {
                    uid, use_spawn_component,
                }).boxed(),
        );

        // Alive entity dispawn
        if let Some(entity_strategy) = alive_or_dead_entity_trategy.clone() {
            options.push(entity_strategy
                .prop_map(|(uid, was_alive)| Transition::DispawnEntity {
                    uid, was_alive,
                })
                .boxed());
        }

        // Register component
        options.push(Union::new(
            state.typed_components_references.iter().enumerate()
                .map(|(reference_idx, reference)| {
                    let is_registered = state.component_entities.get(&reference.dynvec_metadata().type_id)
                        .copied();
                    Just(Transition::RegisterComponent {
                        was_already_registered: is_registered.is_some(),
                        expected_component_uid: is_registered.unwrap_or_default(),
                        reference_idx,
                    })
                })
        ).boxed());

        // Add component to entity
        if let Some(alive_or_dead_entity_strategy) = alive_or_dead_entity_trategy.clone() {
            let component_value_strategy = Union::new(
                state.typed_components_references.iter()
                .map(|reference| {
                    let type_id = reference.dynvec_metadata().type_id;
                    let component_entity_uid = state.component_entities.get(&type_id).copied();
                    reference.arbitrary_value().prop_map(move |value| (value, component_entity_uid))
                })
            );

            let shared_state = Arc::clone(state);
            options.push(
                (alive_or_dead_entity_strategy.clone(), component_value_strategy.clone(), any::<bool>())
                .prop_map(move |((entity_uid, is_entity_alive), (value, maybe_component_entity_uid), use_set)| {
                    let created_component_entity_uid = maybe_component_entity_uid
                        .unwrap_or_else(Uid::new);
                    let value = AddingComponentValue::Add {
                        registered_component_entity_uid: created_component_entity_uid,
                        value,
                        use_set,
                    };

                    let expected_result = if is_entity_alive {
                        AddComponentResult::Success {
                            added: shared_state.entities.get(&entity_uid).unwrap()
                                .contains_key(&created_component_entity_uid)
                        }
                    } else {
                        AddComponentResult::EntityIsNotAlive
                    };

                    Transition::AddComponentToEntity {
                        entity_uid,
                        value,
                        expected_one_of_results: vec![expected_result],
                    }
                })
                .boxed()
            );

            let shared_state = Arc::clone(state);
            options.push(
                (alive_or_dead_entity_strategy.clone(), alive_or_dead_entity_strategy.clone(), component_value_strategy.clone())
                .prop_map(move |((entity_uid, is_entity_alive), (component_entity_uid, is_component_alive), (value, _))| {
                    let mut expected_one_of_results = vec![];

                    match () {
                        () if !is_component_alive || !is_entity_alive => {
                            if !is_component_alive
                            { expected_one_of_results.push(AddComponentResult::ComponentIsNotAlive); }
                            if !is_entity_alive
                            { expected_one_of_results.push(AddComponentResult::EntityIsNotAlive); }
                        },
                        () => match shared_state.get_entity_storage(component_entity_uid) {
                        None => {
                            expected_one_of_results.push(AddComponentResult::ComponentDoesNotHaveStorage);
                        },
                        Some(index) if shared_state.typed_components_references[index].dynvec_metadata().type_id != value.dynvec_metadata().type_id => {
                            expected_one_of_results.push(AddComponentResult::TypeMismatched);
                        },

                        Some(_) => {
                            expected_one_of_results.push(AddComponentResult::Success {
                                added: shared_state.entities.get(&entity_uid).unwrap()
                                    .contains_key(&component_entity_uid),
                            });
                        },
                    }}

                    let value = AddingComponentValue::AddComponentWith {
                        component_entity_uid,
                        value,
                    };

                    Transition::AddComponentToEntity {
                        entity_uid,
                        value,
                        expected_one_of_results,
                    }
                })
                .boxed()
            );

            let shared_state = Arc::clone(state);
            options.push(
                (alive_or_dead_entity_strategy.clone(), alive_or_dead_entity_strategy.clone())
                .prop_map(move |((entity_uid, is_entity_alive), (component_entity_uid, is_component_alive))| {
                    let mut expected_one_of_results = vec![];

                    match () {
                        () if !is_component_alive || !is_entity_alive => {
                            if !is_component_alive
                            { expected_one_of_results.push(AddComponentResult::ComponentIsNotAlive); }
                            if !is_entity_alive
                            { expected_one_of_results.push(AddComponentResult::EntityIsNotAlive); }
                        },
                        () => match shared_state.get_entity_storage(component_entity_uid) {
                        Some(index) if shared_state.typed_components_references[index].dynvec_metadata().default_fn.is_none() => {
                            expected_one_of_results.push(AddComponentResult::ComponentNeedsValue);
                        },

                        Some(_) | None => {
                            expected_one_of_results.push(AddComponentResult::Success {
                                added: shared_state.entities.get(&entity_uid).unwrap()
                                    .contains_key(&component_entity_uid),
                            });
                        },
                    }}

                    let value = AddingComponentValue::AddComponent {
                        component_entity_uid,
                    };

                    Transition::AddComponentToEntity {
                        entity_uid,
                        value,
                        expected_one_of_results,
                    }
                })
                .boxed()
            )
        }

        Union::new(options).boxed()
    }

    fn preconditions(
        state: &Self::State,
        transition: &Self::Transition,
    ) -> bool {
        // let ccuid = state.component_entities[&TypeId::of::<ComponentStorageComponent>()];

        // TODO: It it needed?
        match transition {
            &Transition::SpawnEntity { uid, use_spawn_component: _ } => {
                !state.entities.contains_key(&uid) && !state.dead_entities.contains(&uid)
            },
            &Transition::DispawnEntity { uid, was_alive } => was_alive == state.dead_entities.contains(&uid),
            _ => true,
        }
    }

    fn apply(
        mut arc_state: Self::State,
        transition: &Self::Transition,
    ) -> Self::State {
        let state = Arc::make_mut(&mut arc_state);
        let ccuid = state.component_entities[&TypeId::of::<ComponentStorageComponent>()];

        // Asserts here are just to catch any bug in the test implementation lol
        match transition {
            &Transition::SpawnEntity { uid, use_spawn_component: _ /* Doesn't change anything */ } => {
                state.entities.insert(uid, default());
            },
            &Transition::DispawnEntity { uid, was_alive } => if was_alive {
                state.entities.remove(&uid);
                state.entities.values_mut().for_each(|h| h.retain(|&k, _| k != uid));
                state.dead_entities.push(uid);
            },
            &Transition::RegisterComponent { was_already_registered, expected_component_uid, reference_idx } => if !was_already_registered {
                let reference = &state.typed_components_references[reference_idx];
                let dynvec_metadata = reference.dynvec_metadata();
                state.component_entities.insert(
                    dynvec_metadata.type_id,
                    expected_component_uid,
                );
                assert!(state.entities.contains_key(&expected_component_uid));
                state.entities.insert(expected_component_uid, [
                    (ccuid, Some(Box::new(ComponentStorageComponent {
                        dynvec_meta: dynvec_metadata,
                    }) as Box<dyn TestComponent>)),
                ].into());
            },
            Transition::AddComponentToEntity { entity_uid, value, expected_one_of_results } => if expected_one_of_results.iter()
                .any(|result| matches!(result, AddComponentResult::Success { added: _ }))
            {
                match value {
                    &AddingComponentValue::Add { registered_component_entity_uid, ref value, use_set } => {
                        let already_has = state.entities.get(entity_uid).unwrap()
                            .get(&registered_component_entity_uid).is_some();
                        
                        // only override if using `set`
                        if !already_has || !use_set {
                            state.entities.get_mut(entity_uid).unwrap()
                                .insert(registered_component_entity_uid, Some(value.clone()));
                        }
                    },
                    AddingComponentValue::AddComponentWith { component_entity_uid, value } => {
                        
                    },
                    AddingComponentValue::AddComponent { component_entity_uid } => {
                        
                    },
                }
            },
            Transition::RemoveComponentFromEntity { entity_uid, component, expected } => {
                
            },
        }

        arc_state
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
            _ => todo!(),
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
