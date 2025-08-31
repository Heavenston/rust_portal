use crate::{dyn_option::FunDynOption, world::{
    component::{Component, ComponentDenseStorageInput}, AddComponent, ComponentStorageKind, Entity, GetComponentError, HasComponent, OptionalComponentRef, World
}};

use std::{ any::type_name, assert_matches::debug_assert_matches, iter::once };
use utils::prelude::*;

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

impl World {
    pub fn has<C: Component>(&self, entity: impl Into<Entity>) -> HasComponent {
        let Some(component) = self.try_component::<C>()
        else { return HasComponent::UnknownComponent; };

        self.has_component(entity, component)
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
    /// does not have the component, it is inserted with the given value.
    pub fn add<C: Component>(&mut self, entity: impl Into<Entity>, component: C) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError> {
        self.add_with(entity, || component)
    }

    /// Gets the component of the given type for the given entity, if the entity
    /// does not have the component, then the given function is called
    /// for adding the component to the entity.
    pub fn add_with<C, F>(&mut self, entity: impl Into<Entity>, f: F) -> Result<AddComponent<&'_ mut C>, AddComponentTypedError>
        where F: FnOnce() -> C,
              C: Component,
    {
        let entity = entity.into();

        if !self.alive(entity) {
            return Err(AddComponentTypedError::EntityIsNotAlive { entity });
        }

        let mut fun_dyn_option = FunDynOption::new(f);
        let input = once(ComponentDenseStorageInput::DynOption(&mut fun_dyn_option));

        let component = self.component::<C>();
        let component_storage = self.component_storage(component)
            .expect("World::component should not return a dead entity.");

        debug_assert_matches!(component_storage, ComponentStorageKind::Table { has_default: _ });

        let result = self.add_component_internal(entity, component_storage, component, input);

        Ok(AddComponent {
            component_ref: match result.component_ref {
                OptionalComponentRef::HasStorage(mut r) => r.as_typed::<C>()
                    .expect("Correctly typed"),
                OptionalComponentRef::NoStorage => unreachable!("Created from type so must have storage"),
            },
            was_added: result.was_added,
        })
    }

    pub fn remove<C>(&mut self, entity: impl Into<Entity>) -> Option<C>
        where C: Component,
    {
        let component = self.try_component::<C>()?;

        Some(
            self.remove_component(entity, component)?.into_typed::<C>()
            .expect("Correct component type")
        )
    }
}
