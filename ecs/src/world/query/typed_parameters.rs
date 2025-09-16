use super::{
    QueryParameterImpl, QueryParameterImmutableImpl, ImmutableIterParameters,
    // runtime_parameters::*,
    ColumnSelectParameters, ImmutableWorldRef, MutableIterParameters,
    MutableGetParameters,
};
use crate::world::{ component::ComponentEntity, ArchetypId, Component, Entity, World };

use derive_where::derive_where;
use utils::prelude::*;
use std::iter::{ empty, once, Once };
use std::marker::PhantomData;

#[derive_where(Debug, Clone, Copy)]
pub struct Ref<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    // If the component was not registered we use None
    component: Option<ComponentEntity>,
}

impl<C> QueryParameterImpl for Ref<C>
    where C: Component,
{
    type CreationConfig = ();
    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w, C>;
    type ArchetypMatchBool = bool;

    type TableColumns = Once<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = &'a C;

    fn new(world: &World, (): ()) -> Self {
        let component = world.try_component::<C>();
        // TODO
        assert!(component.is_none_or(|component| world.component_fragments_tables(component)));
        Self {
            _component_type: PhantomData,
            component,
        }
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        self.component.map(|component|
            world.components_to_archetypes[component.index()].iter()
        ).left_or(empty())
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.component.is_some_and(|component| {
            world.archetypes[archetyp_id].components.has(component)
        })
    }

    fn table_columns(&self, ColumnSelectParameters {
        table, ..
    }: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        let Some(component) = self.component
        else {
            let mut m = once(0);
            // we consume the Once to actually return an empty iterator
            m.next();
            return m;
        };

        match table.table_components.index_of(component) {
            Some(p) => once(p),
            // TODO: Make this error more explicit
            None => panic!("Components with None storage cannot be queried for a value. Though this error should be handled before."),
        }
    }

    fn iter_table_mut<'w, I>(&self, MutableIterParameters {
        table_columns, ..
    }: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        self.component.is_some().then(|| {
            let column = table_columns.next().expect("Requested column should be given");
            let typed_column = column.typed::<C>()
                .expect("Should have requested the column with the correct type");
            typed_column.as_slice().iter()
        }).left_or(empty())
    }

    fn get_mut<'w, I>(&self, MutableGetParameters {
        table_columns, entity_dense_idx, ..
    }: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        let component = self.component.expect("Wouldn't be matching if this was None");
        let column = table_columns.next().expect("Requested column should be given");

        column.get(ix!(*entity_dense_idx)).expect("Correct index")
            .as_typed::<C>()
            .expect("Should have requested the column with the correct type")
    }
}

impl<C> QueryParameterImmutableImpl for Ref<C>
    where C: Component,
{
    type ValueIterator<'a> = impl Iterator<Item = &'a C>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.component.map(|component| {
            // TODO
            debug_assert!(world.component_fragments_tables(component));

            let comp_idx = world.tables[table_id].table_components.index_of(component)
                .expect("This component should be in this table");

            world.tables[table_id].sparse_map.dense_values()
                .columns()[comp_idx].typed::<C>().expect("This column should have this type")
                .as_slice().iter()
        }).left_or(empty())
    }

    fn get<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
        let table = &world.tables[table_id];

        let component = self.component
            .expect("Entity would not match if this was None");
        let comp_idx = world.tables[table_id].table_components.index_of(component)
            .expect("This component should be in this table");

        table.sparse_map.get(entity.index())
            .expect("Entity is in this table")
            .for_component(comp_idx)
            .as_typed::<C>()
            .expect("This column should have this type")
    }
}

pub type Has<C> = super::NoFetch<Ref<C>>;

#[derive_where(Debug, Clone, Copy)]
pub struct RefMut<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    // If the component was not registered we use None
    component: Option<ComponentEntity>,
}

impl<C> QueryParameterImpl for RefMut<C>
    where C: Component,
{
    type CreationConfig = ();
    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w, C>;
    type ArchetypMatchBool = bool;

    type TableColumns = Once<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = &'a mut C;

    fn new(world: &World, (): ()) -> Self {
        let component = world.try_component::<C>();
        // TODO
        assert!(component.is_none_or(|component| world.component_fragments_tables(component)));
        Self {
            _component_type: PhantomData,
            component,
        }
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        self.component.map(|component|
            world.components_to_archetypes[component.index()].iter()
        ).left_or(empty())
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.component.is_some_and(|component| {
            world.archetypes[archetyp_id].components.has(component)
        })
    }

    fn table_columns(&self, ColumnSelectParameters {
        table, table_id, archetyp_id, ..
    }: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        let Some(component) = self.component
        else {
            let mut m = once(usize::MAX /* dummy value */);
            // we consume the Once to actually return an empty iterator
            m.next();
            return m;
        };

        match table.table_components.index_of(component) {
            Some(p) => once(p),
            // TODO: Make this error more explicit
            None => panic!("Components with None storage cannot be queried for a value. Though this error should be handled before."),
        }
    }

    fn iter_table_mut<'w, I>(&self, MutableIterParameters {
        table_columns, ..
    }: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        self.component.is_some().then(|| {
            let column = table_columns.next().expect("Requested column should be given");
            let mut typed_column = column.typed_mut::<C>()
                .expect("Should have requested the column with the correct type");
            typed_column.as_mut_slice().iter_mut()
        }).left_or(empty())
    }

    fn get_mut<'w, I>(&self, MutableGetParameters {
        table_columns, entity_dense_idx, ..
    }: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        let component = self.component.expect("Wouldn't be matching if this was None");
        let column = table_columns.next().expect("Requested column should be given");

        column.get_mut(ix!(*entity_dense_idx)).expect("Correct index")
            .as_typed::<C>()
            .expect("Should have requested the column with the correct type")
    }
}
