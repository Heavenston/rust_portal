mod storage;
pub(super) use storage::*;

use super::{ Entity, EntityGeneration, EntityIndex };

use std::{ any::Any, fmt::Display };
use derive_more::{From, Into};
use dynvec::DynVecMetadata;

pub trait Component: 'static + Any { }
impl<T: 'static> Component for T { }

/// When added to entities, describes how to store data for this component
/// into a DynVec.
///
/// Automatically added when registering components through 
#[derive(Debug, Clone)]
pub struct ComponentStorageComponent {
    pub dynvec_meta: DynVecMetadata,
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
