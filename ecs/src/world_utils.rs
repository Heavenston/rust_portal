use crate::world::{
    component::Component, AddComponent, AddComponentWithError, Entity,
    GetComponentError, HasComponent, OptionalComponentRef, RemoveComponentError,
    SetComponentWithError, World,
};

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
pub enum GetComponentTypedError {
    #[error("Tried to get component of dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Component from type '{type_name}' was never registred")]
    UnknownComponent {
        type_name: &'static str,
    },
    #[error("Component from type '{type_name}' is not present in the entity {entity}")]
    ComponentNotPresent {
        type_name: &'static str,
        entity: Entity,
    },
    #[error("Cannot get this component's value: {reason}")]
    Forbidden {
        reason: &'static str,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum AddComponentTypedError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Adding this component to this entity is forbidden: {reason}")]
    Forbidden {
        reason: &'static str
    },
}

#[derive(Debug, thiserror::Error)]
pub enum GetComponentOrDefaultError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Adding this component to this entity is forbidden: {reason}")]
    Forbidden {
        reason: &'static str
    },
}

#[derive(Debug, thiserror::Error)]
pub enum SetComponentTypedError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Adding this component to this entity is forbidden: {reason}")]
    Forbidden {
        reason: &'static str
    },
}

#[derive(Debug, thiserror::Error)]
pub enum RemoveComponentTypedError {
    #[error("Tried to remove a component from a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("Component from type '{type_name}' was never registred")]
    UnknownComponent {
        type_name: &'static str,
    },
    #[error("Component from type '{type_name}' is not present in the entity {entity}")]
    ComponentNotPresent {
        type_name: &'static str,
        entity: Entity,
    },
    #[error("Removing this component from this entity is forbidden by the implementation: {reason}")]
    Forbidden {
        reason: &'static str,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum GetSingletonError {
    #[error("Component from type '{type_name}' was never registred")]
    UnknownComponent {
        type_name: &'static str,
    },
    #[error("Component's entity from type '{type_name}' does not have itself as component")]
    ComponentNotPresent {
        type_name: &'static str,
    },
    #[error("Getting this singleton's value is forbidden: {reason}")]
    Forbidden {
        reason: &'static str,
    },
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
            return Err(GetComponentTypedError::UnknownComponent {
                type_name: type_name::<C>(),
            });
        };

        match self.get_component(entity, component) {
            Ok(component_ref) => Ok(component_ref.as_typed().expect("Type is correct")),
            Err(GetComponentError::EntityIsNotAlive { entity }) => {
                return Err(GetComponentTypedError::EntityIsNotAlive { entity });
            },
            Err(GetComponentError::ComponentNotPresent { component: _, entity }) => {
                return Err(GetComponentTypedError::ComponentNotPresent {
                    type_name: type_name::<C>(),
                    entity,
                });
            },
            Err(GetComponentError::Forbidden { reason }) => {
                return Err(GetComponentTypedError::Forbidden { reason });
            },

            Err(GetComponentError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentHasNoStorage { .. }) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_mut<C: Component>(&mut self, entity: impl Into<Entity>) -> Result<&mut C, GetComponentTypedError> {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(GetComponentTypedError::UnknownComponent {
                type_name: type_name::<C>(),
            });
        };

        match self.get_component_mut(entity, component) {
            Ok(component_ref) => Ok(component_ref.as_typed().expect("Type is correct")),
            Err(GetComponentError::EntityIsNotAlive { entity }) => {
                return Err(GetComponentTypedError::EntityIsNotAlive { entity });
            },
            Err(GetComponentError::ComponentNotPresent { component: _, entity }) => {
                return Err(GetComponentTypedError::ComponentNotPresent {
                    type_name: type_name::<C>(),
                    entity,
                });
            },
            Err(GetComponentError::Forbidden { reason }) => {
                return Err(GetComponentTypedError::Forbidden { reason });
            },

            Err(GetComponentError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentHasNoStorage { .. }) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_or_default<C: Component + Default>(&mut self, entity: Entity) -> Result<&mut C, GetComponentOrDefaultError> {
        match self.add_with::<C, _>(entity, default) {
            Ok(AddComponent::Added | AddComponent::AlreadyPresent) => (),
            Err(AddComponentTypedError::EntityIsNotAlive { entity }) =>
                return Err(GetComponentOrDefaultError::EntityIsNotAlive { entity }),
            Err(AddComponentTypedError::Forbidden { reason }) =>
                return Err(GetComponentOrDefaultError::Forbidden { reason }),
        }

        match self.get_mut::<C>(entity) {
            Ok(ref_mut) => Ok(ref_mut),
            Err(GetComponentTypedError::EntityIsNotAlive { entity }) =>
                Err(GetComponentOrDefaultError::EntityIsNotAlive { entity }),
            Err(GetComponentTypedError::Forbidden { reason }) =>
                Err(GetComponentOrDefaultError::Forbidden { reason }),

            Err(GetComponentTypedError::UnknownComponent { .. }) |
            Err(GetComponentTypedError::ComponentNotPresent { .. }) =>
                unreachable!("Just "),
        }
    }
    
    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: impl Into<Entity>, f: F) -> Result<AddComponent, AddComponentTypedError>
        where F: FnOnce() -> C,
              C: Component,
    {
        let component = self.component::<C>();

        match self.add_component_with(entity, component, f) {
            Ok(result) => Ok(result),
            Err(AddComponentWithError::EntityIsNotAlive { entity }) =>
                Err(AddComponentTypedError::EntityIsNotAlive { entity }),
            Err(AddComponentWithError::Forbidden { reason }) =>
                Err(AddComponentTypedError::Forbidden { reason }),

            Err(AddComponentWithError::TypeMismatched { .. }) |
            Err(AddComponentWithError::ComponentDoesNotHaveStorage { .. }) =>
                unreachable!("World::component should return an entity with the correct storage"),
            Err(AddComponentWithError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
        }
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> Result<AddComponent, AddComponentTypedError> {
        self.add_with(entity, || component)
    }

    /// Sets the value for the given component on the given entity, overrides
    /// the component's value if the entity already has it.
    pub fn set<C: Component>(&mut self, entity: impl Into<Entity>, value: C) -> Result<AddComponent, SetComponentTypedError> {
        let component = self.component::<C>();

        match self.set_component_with(entity, component, || value) {
            Ok(result) => Ok(result),
            Err(SetComponentWithError::EntityIsNotAlive { entity }) =>
                Err(SetComponentTypedError::EntityIsNotAlive { entity }),
            Err(SetComponentWithError::Forbidden { reason }) =>
                Err(SetComponentTypedError::Forbidden { reason }),

            Err(SetComponentWithError::TypeMismatched { .. }) |
            Err(SetComponentWithError::ComponentDoesNotHaveStorage { .. }) =>
                unreachable!("World::component should return an entity with the correct storage"),
            Err(SetComponentWithError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
        }
    }

    pub fn remove<C>(&mut self, entity: impl Into<Entity>) -> Result<C, RemoveComponentTypedError>
        where C: Component,
    {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(RemoveComponentTypedError::UnknownComponent {
                type_name: type_name::<C>(),
            })
        };

        match self.remove_component(entity, component) {
            Ok(OptionalComponentRef::HasStorage(value)) =>
                Ok(value.into_typed::<C>().expect("Correct component type")),
            Ok(OptionalComponentRef::NoStorage) =>
                unreachable!("Typed components all have storage"),
            Err(RemoveComponentError::EntityIsNotAlive { entity }) =>
                Err(RemoveComponentTypedError::EntityIsNotAlive {
                    entity
                }),

            Err(RemoveComponentError::ComponentIsNotAlive { component: _ }) =>
                unreachable!("World::try_component should not return a dead entity"),
            Err(RemoveComponentError::ComponentNotPresent { component: _, entity }) =>
                Err(RemoveComponentTypedError::ComponentNotPresent {
                    type_name: type_name::<C>(),
                    entity,
                }),
            Err(RemoveComponentError::Forbidden { reason }) =>
                Err(RemoveComponentTypedError::Forbidden { reason }),
        }
    }

    pub fn set_singleton<C: Component>(&mut self, value: C) -> () {
        let entity = self.component::<C>();

        match self.set(entity, value) {
            Ok(_) => (),
            Err(SetComponentTypedError::Forbidden { reason }) =>
                panic!("Set a singleton returned forbidden: {reason}"),
            Err(SetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
        }
    }

    pub fn get_singleton<C: Component>(&self) -> Result<&C, GetSingletonError> {
        let Some(entity) = self.try_component::<C>()
        else {
            return Err(GetSingletonError::UnknownComponent {
                type_name: type_name::<C>()
            })
        };

        match self.get::<C>(entity) {
            Ok(component_ref) => Ok(component_ref),
            Err(GetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
            Err(GetComponentTypedError::UnknownComponent { .. }) =>
                unreachable!("Just checked this"),
            Err(GetComponentTypedError::ComponentNotPresent { type_name, .. }) =>
                return Err(GetSingletonError::ComponentNotPresent { type_name }),
            Err(GetComponentTypedError::Forbidden { reason, .. }) =>
                return Err(GetSingletonError::Forbidden { reason }),
        }
    }

    pub fn get_singleton_mut<C: Component>(&mut self) -> Result<&mut C, GetSingletonError> {
        let Some(entity) = self.try_component::<C>()
        else {
            return Err(GetSingletonError::UnknownComponent {
                type_name: type_name::<C>()
            })
        };

        match self.get_mut::<C>(entity) {
            Ok(component_ref) => Ok(component_ref),
            Err(GetComponentTypedError::EntityIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
            Err(GetComponentTypedError::UnknownComponent { .. }) =>
                unreachable!("Just checked this"),
            Err(GetComponentTypedError::ComponentNotPresent { type_name, .. }) =>
                return Err(GetSingletonError::ComponentNotPresent { type_name }),
            Err(GetComponentTypedError::Forbidden { reason, .. }) =>
                return Err(GetSingletonError::Forbidden { reason }),
        }
    }
}
