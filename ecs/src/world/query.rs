#![allow(dead_code)]
#![allow(unused_variables)]

mod parameters;
pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use super::*;

use std::marker::PhantomData;

mod private {
    pub trait QueryParameterImpl {
        type ValueMut<'a>;
    }

    pub trait QueryParameterImmutableImpl: QueryParameterImpl {
        type Value<'a>;
    }
}
use private::{ QueryParameterImpl, QueryParameterImmutableImpl };

pub trait QueryParameter = QueryParameterImpl;
pub trait QueryParameterImmutable = QueryParameterImmutableImpl;

pub struct Query<P: QueryParameter> {
    _parameters: PhantomData<fn(P) -> P>,
    archetypes: Vec<ArchetypId>,
}

impl<P: QueryParameterImpl> Query<P> {
    pub fn new(world: &mut World) -> Self {
        Self {
            _parameters: PhantomData,
            archetypes: world.archetypes.iter()
                .filter(|&(archetyp_id, archtyp)| {
                    todo!()
                })
                .map(|(id, _)| id)
                .collect(),
        }
    }
}
