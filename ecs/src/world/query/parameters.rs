use super::{ QueryParameter, QueryParameterImmutable };
use crate::world::component::Component;

use std::marker::PhantomData;
use derive_where::derive_where;

#[derive_where(Default, Debug, Clone, Copy)]
pub struct Ref<C>
    where C: Component,
{
    _component: PhantomData<fn(C) -> C>,
}

impl<C> QueryParameter for Ref<C>
    where C: Component,
{
    type ValueMut<'a> = &'a C;
}

impl<C> QueryParameterImmutable for Ref<C>
    where C: Component,
{
    type Value<'a> = &'a C;
}

#[derive_where(Default, Debug, Clone, Copy)]
pub struct RefMut<C>
    where C: Component,
{
    _component: PhantomData<fn(C) -> C>,
}

impl<C> QueryParameter for RefMut<C>
    where C: Component,
{
    type ValueMut<'a> = &'a mut C;
}

#[derive_where(Default, Debug, Clone, Copy)]
pub struct Has<C>
    where C: Component,
{
    _component: PhantomData<fn(C) -> C>,
}

impl<C> QueryParameter for Has<C>
    where C: Component,
{
    type ValueMut<'a> = ();
}

impl<C> QueryParameterImmutable for Has<C>
    where C: Component,
{
    type Value<'a> = ();
}
