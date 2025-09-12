use super::{
    QueryParameterImpl,
    QueryParameterImmutableImpl,
    ImmutableIterParameters,
    runtime_parameters::*,
};
use crate::world::{ ArchetypId, World, Component, };

use std::{iter::empty, marker::PhantomData};
use derive_where::derive_where;
use utils::prelude::*;

#[derive_where(Debug, Clone, Copy)]
pub struct Ref<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    // If the component was not registered we use None
    child: Option<ComponentRef>,
}

impl<C> QueryParameterImpl for Ref<C>
    where C: Component,
{
    type CreationConfig = ();
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId> + 'w;
    type ArchetypMatchBool = Bool;

    type Value<'a> = &'a C;

    fn new(world: &World, (): ()) -> Self {
        Self {
            _component_type: PhantomData,
            child: world.try_component::<C>().map(|c| ComponentRef::new(world, c)),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        self.child.as_ref().map(|child| child.matching_archetypes(world))
            .left_or(empty())
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.child.is_some_and(|child| child.match_archetyp(world, archetyp_id).is_true()).into()
    }
}

impl<C> QueryParameterImmutableImpl for Ref<C>
    where C: Component,
{
    type ValueIterator<'w> = impl Iterator<Item = &'w C>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.child.as_ref().map(|child| {
            // TODO
            assert!(world.component_fragments_tables(child.component));

            let Some(comp_idx) = world.tables[table_id].table_components.index_of(child.component)
            else { unreachable!() };

        world.tables[table_id].sparse_set.dense_values()
            .column(comp_idx).typed::<C>()
            .expect("World::try_component should return components with the correct type")
            .as_slice().iter()
        }).left_or(empty())
    }
}

pub type Has<C> = super::NoFetch<Ref<C>>;

#[derive_where(Debug, Clone, Copy)]
pub struct RefMut<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    // If the component was not registered we use None
    child: Option<ComponentRefMut>,
}

impl<C> QueryParameterImpl for RefMut<C>
    where C: Component,
{
    type CreationConfig = ();
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId> + 'w;
    type ArchetypMatchBool = Bool;

    type Value<'a> = &'a C;

    fn new(world: &World, (): ()) -> Self {
        Self {
            _component_type: PhantomData,
            child: world.try_component::<C>().map(|c| ComponentRefMut::new(world, c)),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        self.child.as_ref().map(|child| child.matching_archetypes(world))
            .left_or(empty())
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.child.is_some_and(|child| child.match_archetyp(world, archetyp_id).is_true()).into()
    }
}
