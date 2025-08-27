mod storage;
pub use storage::*;

use std::any::Any;

use dynvec::DynVecMetadata;

pub trait Component: 'static + Any { }
impl<T: 'static> Component for T { }

/// Component given to all entities of components
#[derive(Clone)]
pub struct ComponentComponent {
    pub dynvec_meta: DynVecMetadata,
}
