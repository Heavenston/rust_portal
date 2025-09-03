#![allow(dead_code)]
#![allow(unused_variables)]

mod parameters;
pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use super::*;

mod private {
    use super::*;

    pub trait QueryParameterImpl {
        type ValueMut<'a>;

        fn new(world: &World) -> Self;
        fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> bool;
    }

    pub trait QueryParameterImmutableImpl: QueryParameterImpl {
        type Value<'a>;
    }
}
use private::{ QueryParameterImpl, QueryParameterImmutableImpl };

pub trait QueryParameter = QueryParameterImpl;
pub trait QueryParameterImmutable = QueryParameterImmutableImpl;

#[derive(Debug, thiserror::Error)]
pub enum QueryGetError {
    #[error("Tried to get components of a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
}

pub struct Query<P: QueryParameter> {
    parameters: P,
    archetypes: Vec<ArchetypId>,
}

impl<P: QueryParameterImpl> Query<P> {
    pub fn new(world: &World) -> Self {
        let parameters = P::new(world);

        let archetypes = world.archetypes.indices()
            .filter(|&archetyp_id| parameters.match_archetyp(world, archetyp_id))
            .collect();

        Self {
            parameters: P::new(&*world),
            archetypes,
        }
    }

    pub fn get<'a, 'b>(&'a self, world: &'b World, entity: Entity) -> Result<P::Value<'b>, QueryGetError>
        where P: QueryParameterImmutable,
    {

        todo!()
    }
}
