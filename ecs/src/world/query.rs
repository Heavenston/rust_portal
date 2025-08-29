mod parameters;
pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use std::marker::PhantomData;

pub trait QueryParameter {
    type ValueMut<'a>;
}

pub trait QueryParameterImmutable: QueryParameter {
    type Value<'a>;
}

pub struct Query<P: QueryParameter> {
    _parameters: PhantomData<fn(P) -> P>,
}

impl<P: QueryParameter> Query<P> {
    
}
