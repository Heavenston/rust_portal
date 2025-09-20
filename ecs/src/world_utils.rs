use crate::world::{ *, component::* };

use std::any::type_name;
use derive_more::IsVariant;
use utils::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, IsVariant)]
pub enum HasComponentTyped {
    /// The entity is dead
    EntityIsNotAlive,
    /// The component was never registred
    UnknownComponent,
    /// The component is not present in this entity's archetyp
    NotPresent,
    /// The component *is* present in this entity's archtyp
    Present,
}

#[derive(Debug, thiserror::Error)]
#[error("Component from type '{type_name}' was never registred")]
pub struct UnknownComponentError {
    pub type_name: &'static str,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum GetComponentTypedError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    UnknownComponent(#[from] UnknownComponentError),
    ComponentNotPresent(#[from] ComponentNotPresentError),
    Forbidden(#[from] ForbiddenError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum AddComponentTypedError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    Forbidden(#[from] ForbiddenError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum GetComponentOrDefaultError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    Forbidden(#[from] ForbiddenError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum SetComponentTypedError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    Forbidden(#[from] ForbiddenError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum RemoveComponentTypedError {
    EntityIsNotAlive(#[from] EntityIsNotAliveError),
    UnknownComponent(#[from] UnknownComponentError),
    ComponentNotPresent(#[from] ComponentNotPresentError),
    Forbidden(#[from] ForbiddenError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum GetSingletonError {
    UnknownComponent(#[from] UnknownComponentError),
    ComponentNotPresent(#[from] ComponentNotPresentError),
    Forbidden(#[from] ForbiddenError),
}

impl World {
    pub fn has<C: Component>(&self, entity: impl Into<Entity>) -> HasComponentTyped {
        let Some(component) = self.try_component::<C>()
        else { return HasComponentTyped::UnknownComponent; };

        match self.has_component(entity, component) {
            HasComponent::EntityIsNotAlive => HasComponentTyped::EntityIsNotAlive,
            HasComponent::ComponentIsNotAlive
                => unreachable!("try_component always return alive entities"),
            HasComponent::NotPresent => HasComponentTyped::NotPresent,
            HasComponent::Present => HasComponentTyped::Present,
        }
    }

    pub fn get<C: Component>(&self, entity: impl Into<Entity>) -> Result<&C, GetComponentTypedError> {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(UnknownComponentError {
                type_name: type_name::<C>(),
            }.into());
        };

        match self.get_component(entity, component) {
            Ok(component_ref) => Ok(component_ref.as_typed().expect("Type is correct")),
            Err(GetComponentError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(GetComponentError::ComponentNotPresent(e)) => Err(e.into()),
            Err(GetComponentError::Forbidden(e)) => Err(e.into()),

            Err(GetComponentError::ComponentIsNotAlive(_)) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentDoesNotHaveStorage(_)) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_mut<C: Component>(&mut self, entity: impl Into<Entity>) -> Result<&mut C, GetComponentTypedError> {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(UnknownComponentError {
                type_name: type_name::<C>(),
            }.into());
        };

        match self.get_component_mut(entity, component) {
            Ok(component_ref) => Ok(component_ref.as_typed().expect("Type is correct")),
            Err(GetComponentError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(GetComponentError::ComponentNotPresent(e)) => Err(e.into()),
            Err(GetComponentError::Forbidden(e)) => Err(e.into()),

            Err(GetComponentError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentDoesNotHaveStorage { .. }) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_or_default<C: Component + Default>(&mut self, entity: Entity) -> Result<&mut C, GetComponentOrDefaultError> {
        match self.add_with::<C, _>(entity, default) {
            Ok(AddComponentOutcome::Added | AddComponentOutcome::AlreadyPresent) => (),
            Err(AddComponentTypedError::EntityIsNotAlive(e)) => return Err(e.into()),
            Err(AddComponentTypedError::Forbidden(e)) => return Err(e.into()),
        }

        match self.get_mut::<C>(entity) {
            Ok(ref_mut) => Ok(ref_mut),
            Err(GetComponentTypedError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(GetComponentTypedError::Forbidden(e)) => Err(e.into()),

            Err(GetComponentTypedError::UnknownComponent { .. }) |
            Err(GetComponentTypedError::ComponentNotPresent { .. }) =>
                unreachable!("Just checked"),
        }
    }
    
    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: impl Into<Entity>, f: F) -> Result<AddComponentOutcome, AddComponentTypedError>
        where F: FnOnce() -> C,
              C: Component,
    {
        match self.add_bundle(entity, LazyBundle((f,))) {
            Ok(AddBundleOutcome::AtLeastOneWasAdded) => Ok(AddComponentOutcome::Added),
            Ok(AddBundleOutcome::AllWasAlreadyPresent) => Ok(AddComponentOutcome::AlreadyPresent),
            Err(LazyBundleError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(LazyBundleError::Forbidden(e)) => Err(e.into()),
        }
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> Result<AddComponentOutcome, AddComponentTypedError> {
        match self.add_bundle(entity, (component,)) {
            Ok(AddBundleOutcome::AtLeastOneWasAdded) => Ok(AddComponentOutcome::Added),
            Ok(AddBundleOutcome::AllWasAlreadyPresent) => Ok(AddComponentOutcome::AlreadyPresent),
            Err(TypedBundleError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(TypedBundleError::Forbidden(e)) => Err(e.into()),
        }
    }

    /// Sets the value for the given component on the given entity, overrides
    /// the component's value if the entity already has it.
    pub fn set<C: Component>(&mut self, entity: impl Into<Entity>, value: C) -> Result<AddComponentOutcome, SetComponentTypedError> {
        match self.set_bundle(entity, (value,)) {
            Ok(AddBundleOutcome::AtLeastOneWasAdded) => Ok(AddComponentOutcome::Added),
            Ok(AddBundleOutcome::AllWasAlreadyPresent) => Ok(AddComponentOutcome::AlreadyPresent),
            Err(TypedBundleError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(TypedBundleError::Forbidden(e)) => Err(e.into()),
        }
    }

    pub fn remove<C>(&mut self, entity: impl Into<Entity>) -> Result<C, RemoveComponentTypedError>
        where C: Component,
    {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(UnknownComponentError {
                type_name: type_name::<C>(),
            }.into())
        };

        match self.remove_component(entity, component) {
            Ok(OptionalComponentRef::HasStorage(value)) =>
                Ok(value.into_typed::<C>().expect("Correct component type")),
            Ok(OptionalComponentRef::NoStorage) =>
                unreachable!("Typed components all have storage"),

            Err(RemoveComponentError::EntityIsNotAlive(e)) => Err(e.into()),
            Err(RemoveComponentError::ComponentNotPresent(e)) => Err(e.into()),
            Err(RemoveComponentError::Forbidden(e)) => Err(e.into()),

            Err(RemoveComponentError::ComponentIsNotAlive(_)) =>
                unreachable!("World::try_component should not return a dead entity"),
        }
    }

    pub fn set_singleton<C: Component>(&mut self, value: C) -> () {
        let entity = self.component::<C>();

        match self.set(entity, value) {
            Ok(_) => (),
            Err(SetComponentTypedError::Forbidden(e)) =>
                panic!("Set a singleton returned forbidden: {e}"),
            Err(SetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
        }
    }

    pub fn get_singleton<C: Component>(&self) -> Result<&C, GetSingletonError> {
        let Some(entity) = self.try_component::<C>()
        else {
            return Err(UnknownComponentError {
                type_name: type_name::<C>()
            }.into())
        };

        match self.get::<C>(entity) {
            Ok(component_ref) => Ok(component_ref),
            Err(GetComponentTypedError::ComponentNotPresent(e)) => Err(e.into()),
            Err(GetComponentTypedError::Forbidden(e)) => Err(e.into()),

            Err(GetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
            Err(GetComponentTypedError::UnknownComponent { .. }) =>
                unreachable!("Just checked this"),
        }
    }

    pub fn get_singleton_mut<C: Component>(&mut self) -> Result<&mut C, GetSingletonError> {
        let Some(entity) = self.try_component::<C>()
        else {
            return Err(UnknownComponentError {
                type_name: type_name::<C>()
            }.into())
        };

        match self.get_mut::<C>(entity) {
            Ok(component_ref) => Ok(component_ref),
            Err(GetComponentTypedError::ComponentNotPresent(e)) => Err(e.into()),
            Err(GetComponentTypedError::Forbidden(e)) => Err(e.into()),

            Err(GetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
            Err(GetComponentTypedError::UnknownComponent { .. }) =>
                unreachable!("Just checked this"),
        }
    }
}
