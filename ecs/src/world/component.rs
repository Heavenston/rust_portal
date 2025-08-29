mod storage;
pub use storage::*;

use super::{ Entity, EntityGeneration, EntityIndex };

use std::any::Any;
use derive_more::{From, Into};
use dynvec::DynVecMetadata;

pub trait Component: 'static + Any { }
impl<T: 'static> Component for T { }

/// Component given to all entities of components
#[derive(Clone)]
pub struct ComponentComponent {
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
