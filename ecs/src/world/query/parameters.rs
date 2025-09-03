use super::{ QueryParameterImpl, QueryParameterImmutableImpl };
use crate::world::{
    component::{ Component, ComponentEntity }, ArchetypId, World
};

use std::marker::PhantomData;
use derive_where::derive_where;

#[derive_where(Debug, Clone, Copy)]
pub struct Ref<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    component: Option<ComponentEntity>,
}

impl<C> QueryParameterImpl for Ref<C>
    where C: Component,
{
    type ValueMut<'a> = &'a C;

    fn new(world: &World) -> Self {
        Self {
            _component_type: PhantomData,
            component: world.try_component::<C>(),
        }
    }

    fn match_archetyp(&self, world: &World, archtyp_id: ArchetypId) -> bool {
        self.component.is_some_and(|component|
            world.archetypes[archtyp_id].components.has(component)
        )
    }
}

impl<C> QueryParameterImmutableImpl for Ref<C>
    where C: Component,
{
    type Value<'a> = &'a C;
}

#[derive_where(Debug, Clone, Copy)]
pub struct RefMut<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    component: Option<ComponentEntity>,
}

impl<C> QueryParameterImpl for RefMut<C>
    where C: Component,
{
    type ValueMut<'a> = &'a mut C;

    fn new(world: &World) -> Self {
        Self {
            _component_type: PhantomData,
            component: world.try_component::<C>(),
        }
    }

    fn match_archetyp(&self, world: &World, archtyp_id: ArchetypId) -> bool {
        self.component.is_some_and(|component|
            world.archetypes[archtyp_id].components.has(component)
        )
    }
}

#[derive_where(Debug, Clone, Copy)]
pub struct Has<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    component: Option<ComponentEntity>,
}

impl<C> QueryParameterImpl for Has<C>
    where C: Component,
{
    type ValueMut<'a> = ();

    fn new(world: &World) -> Self {
        Self {
            _component_type: PhantomData,
            component: world.try_component::<C>(),
        }
    }

    fn match_archetyp(&self, world: &World, archtyp_id: ArchetypId) -> bool {
        self.component.is_some_and(|component|
            world.archetypes[archtyp_id].components.has(component)
        )
    }
}

impl<C> QueryParameterImmutableImpl for Has<C>
    where C: Component,
{
    type Value<'a> = ();
}
