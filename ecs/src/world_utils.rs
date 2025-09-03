use crate::{
    world::{
        component::Component,
        AddComponent, AddComponentWithError, Entity,
        GetComponentError, HasComponent, OptionalComponentRef, RemoveComponentError, World
    }
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
}

#[derive(Debug, thiserror::Error)]
pub enum AddComponentTypedError {
    #[error("Tried to add a component to a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
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

            Err(GetComponentError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentHasNoStorage { .. }) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_mut<C: Component>(&mut self, entity: Entity) -> Result<&mut C, GetComponentTypedError> {
        let Some(component) = self.try_component::<C>()
        else {
            return Err(GetComponentTypedError::UnknownComponent {
                type_name: type_name::<C>(),
            });
        };

        match self.get_component_mut(entity, component) {
            Ok(mut component_ref) => Ok(component_ref.as_typed().expect("Type is correct")),
            Err(GetComponentError::EntityIsNotAlive { entity }) => {
                return Err(GetComponentTypedError::EntityIsNotAlive { entity });
            },
            Err(GetComponentError::ComponentNotPresent { component: _, entity }) => {
                return Err(GetComponentTypedError::ComponentNotPresent {
                    type_name: type_name::<C>(),
                    entity,
                });
            },

            Err(GetComponentError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::try_component should not return dead entities."),
            Err(GetComponentError::ComponentHasNoStorage { .. }) =>
                unreachable!("Getting components from types (with World::try_component) will always create storage."),
        }
    }

    pub fn get_or_default<C: Component + Default>(&mut self, entity: Entity) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError> {
        self.add_with(entity, default)
    }
    
    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: impl Into<Entity>, f: F) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError>
        where F: FnOnce() -> C,
              C: Component,
    {
        let component = self.component::<C>();

        match self.add_component_with(entity, component, f) {
            Ok(result) => Ok(result),
            Err(AddComponentWithError::EntityIsNotAlive { entity }) =>
                Err(AddComponentTypedError::EntityIsNotAlive { entity }),
            Err(AddComponentWithError::TypeMismatched { .. }) |
            Err(AddComponentWithError::ComponentDoesNotHaveStorage { .. }) =>
                unreachable!("World::component should return an entity with the correct storage"),
            Err(AddComponentWithError::ComponentIsNotAlive { .. }) =>
                unreachable!("World::component should not return a dead entity"),
        }
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError> {
        self.add_with(entity, || component)
    }

    /// Sets the value for the given component on the given entity, overrides
    /// the component's value if the entity already has it.
    pub fn set<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError> {
        let mut value = Some(component);
        let result = self.add_with::<C, _>(entity, || value.take().expect("Took once"))?;
        if let Some(value) = value {
            debug_assert_eq!(result.was_added, false);
            *result.component_ref = value;
        }
        Ok(result)
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
        }
    }
}
