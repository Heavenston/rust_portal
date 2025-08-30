#![allow(dead_code)]
#![allow(unused_variables)]

mod parameters;
pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use super::*;

use std::marker::PhantomData;

pub trait QueryParameter {
    type ValueMut<'a>;
}

pub trait QueryParameterImmutable: QueryParameter {
    type Value<'a>;
}

pub struct Query<P: QueryParameter> {
    _parameters: PhantomData<fn(P) -> P>,
    archetypes: Vec<ArchetypId>,
}

impl<P: QueryParameter> Query<P> {
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
