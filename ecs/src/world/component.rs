mod storage;
pub(super) use storage::*;

use super::{ World, Entity, EntityGeneration, EntityIndex };

use std::{ any::Any, fmt::Display };
use derive_more::{ From, Into };
use dynvec::DynVecMetadata;

pub trait Component: 'static + Any {
    /// Called when registering the component
    /// This is not allowed to modify the [`ComponentStorageComponent`]
    /// only add/remove other components on the component entity such as
    /// component tags.
    fn on_register(world: &mut World, entity: ComponentEntity);
}

impl<T: 'static> Component for T {
    default fn on_register(world: &mut World, entity: ComponentEntity) {
        let _ = world;
        let _ = entity;
    }
}

pub mod component_tags {
    /// If added to a component's entity, a mutable reference cannot be acquired
    /// to its storage data. It can still be removed and added again though.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Readonly;

    impl Readonly {
        pub const REASON: &str = "This component is read-only (it has the Readonly component tag)";
    }
}
use component_tags as tags;

/// When added to entities, describes how to store data for this component
/// into a DynVec.
///
/// Automatically added when registering components through 
#[derive(Debug, Clone)]
pub struct ComponentStorageComponent {
    pub dynvec_meta: DynVecMetadata,
}

impl Component for ComponentStorageComponent {
    fn on_register(world: &mut World, entity: ComponentEntity) {
        // NOTE: This is hard-coded in `World::component_storage` so it needs
        // to be changed there too
        world.add(entity, tags::Readonly).unwrap();
    }
}

/// Newtype for Entities for component used for clarity, no checks are done to
/// verify that the Entity is a component.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Into)]
pub struct ComponentEntity(pub Entity);

impl ComponentEntity {
    pub fn index(self) -> EntityIndex {
        self.0.index()
    }

    pub fn generation(self) -> EntityGeneration {
        self.0.generation()
    }
}

impl<'a> Into<Entity> for &'a ComponentEntity {
    fn into(self) -> Entity {
        self.0
    }
}

impl Display for ComponentEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Component({})", self.0)
    }
}
