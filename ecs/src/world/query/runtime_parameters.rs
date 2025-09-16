use super::{
    QueryParameterImpl, QueryParameterImmutableImpl, ImmutableWorldRef,
    MutableIterParameters, ImmutableIterParameters, ColumnSelectParameters,
    MutableGetParameters,
};
use crate::world::{
    component::ComponentEntity, ArchetypId, Entity, World
};

use utils::prelude::*;
use std::iter::{ empty, Empty, once, Once };

#[derive(Default, Debug, Clone, Copy)]
pub struct EntityHandle;

impl QueryParameterImpl for EntityHandle {
    type CreationConfig = ();
    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w>;
    type ArchetypMatchBool = True;

    type TableColumns = Empty<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = Entity;

    fn new(world: &World, (): ()) -> Self {
        Self
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        world.archetypes.indices()
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        True
    }

    fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        empty()
    }

    fn iter_table_mut<'w, I>(&self, MutableIterParameters {
        world, table_entities, table_columns, ..
    }: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        table_entities.iter()
            .map(|&index| Entity::new(index, world.entity_storage.generation_at_index(index)))
    }

    fn get_mut<'w, I>(&self, MutableGetParameters {
        entity, ..
    }: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        *entity
    }
}

impl QueryParameterImmutableImpl for EntityHandle {
    type ValueIterator<'a> = impl Iterator<Item = Entity>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        world.tables[table_id].sparse_set.sparse_indices()
            .map(|index| Entity::new(index, world.generation_at_index(index)))
    }

    fn get<'w>(&self, parameters: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
        entity
    }
}

pub type Always = super::NoFetch<EntityHandle>;
pub type Never = super::Not<Always>;

#[derive(Debug, Clone, Copy)]
pub struct ComponentRef {
    component: ComponentEntity,
}

impl QueryParameterImpl for ComponentRef {
    type CreationConfig = ComponentEntity;
    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w>;
    type ArchetypMatchBool = Bool;

    type TableColumns = Once<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = dynvec::DynVecValueRef<'a>;

    fn new(world: &World, component: ComponentEntity) -> Self {
        // TODO
        assert!(world.component_fragments_tables(component));
        Self {
            component,
        }
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        let comp = self.component;
        world.components_to_archetypes[self.component.index()].iter()
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        world.archetypes[archetyp_id].components.has(self.component).into()
    }

    fn table_columns(&self, ColumnSelectParameters {
        table, ..
    }: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        match table.table_components.index_of(self.component) {
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
        let column = table_columns.next().expect("Requested column should be given");
        column.iter()
    }

    fn get_mut<'w, I>(&self, MutableGetParameters {
        table_columns, entity_dense_idx, ..
    }: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        let column = table_columns.next().expect("Requested column should be given");
        column.get(ix!(*entity_dense_idx)).expect("Correct index")
    }
}

impl QueryParameterImmutableImpl for ComponentRef {
    type ValueIterator<'a> = impl Iterator<Item = dynvec::DynVecValueRef<'a>>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        // TODO
        debug_assert!(world.component_fragments_tables(self.component));

        let Some(comp_idx) = world.tables[table_id].table_components.index_of(self.component)
        else { unreachable!() };

        world.tables[table_id].sparse_set.dense_values()
            .columns()[comp_idx].iter()
    }

    fn get<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
        let table = &world.tables[table_id];

        let Some(comp_idx) = table.table_components.index_of(self.component)
        else { unreachable!() };

        table.sparse_set.get(entity.index())
            .expect("Entity is in this table")
            .for_component(comp_idx)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentRefMut {
    component: ComponentEntity,
}

impl QueryParameterImpl for ComponentRefMut {
    type CreationConfig = ComponentEntity;
    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w>;
    type ArchetypMatchBool = Bool;

    type TableColumns = Once<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = dynvec::DynVecValueRefMut<'a>;

    fn new(world: &World, component: ComponentEntity) -> Self {
        // TODO
        assert!(world.component_fragments_tables(component));
        Self {
            component,
        }
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        world.components_to_archetypes[self.component.index()].iter()
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        world.archetypes[archetyp_id].components.has(self.component).into()
    }

    fn table_columns(&self, ColumnSelectParameters {
        table, ..
    }: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        match table.table_components.index_of(self.component) {
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
        let column = table_columns.next().expect("Requested column should be given");
        column.iter_mut()
    }

    fn get_mut<'w, I>(&self, MutableGetParameters {
        table_columns, entity_dense_idx, ..
    }: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        let column = table_columns.next().expect("Requested column should be given");
        column.get_mut(ix!(*entity_dense_idx)).expect("Correct index")
    }
}

pub type HasComponent = super::NoFetch<ComponentRef>;
