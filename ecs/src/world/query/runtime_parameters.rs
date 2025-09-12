use super::{ QueryParameterImpl, QueryParameterImmutableImpl, ImmutableIterParameters };
use crate::world::{
    component::ComponentEntity, ArchetypId, Entity, World
};

use utils::prelude::*;

#[derive(Default, Debug, Clone, Copy)]
pub struct EntityHandle;

impl QueryParameterImpl for EntityHandle {
    type CreationConfig = ();
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId> + 'w;
    type ArchetypMatchBool = True;

    type Value<'a> = Entity;

    fn new(world: &World, (): ()) -> Self {
        Self
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        world.archetypes.indices()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        True
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
}

pub type Always = super::NoFetch<EntityHandle>;
pub type Never = super::Not<Always>;

#[derive(Debug, Clone, Copy)]
pub struct ComponentRef {
    pub(super) component: ComponentEntity,
}

impl QueryParameterImpl for ComponentRef {
    type CreationConfig = ComponentEntity;
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId>;
    type ArchetypMatchBool = Bool;

    type Value<'a> = dynvec::DynVecValueRef<'a>;

    fn new(world: &World, component: ComponentEntity) -> Self {
        // TODO
        assert!(world.component_fragments_tables(component));
        Self {
            component,
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        let comp = self.component;
        world.archetypes.iter()
            .filter(move |(_, archetyp)| archetyp.components.has(comp))
            .map(|(aid, _)| aid)
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        world.archetypes[archetyp_id].components.has(self.component).into()
    }
}

impl QueryParameterImmutableImpl for ComponentRef {
    type ValueIterator<'a> = impl Iterator<Item = dynvec::DynVecValueRef<'a>>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        // TODO
        assert!(world.component_fragments_tables(self.component));

        let Some(comp_idx) = world.tables[table_id].table_components.index_of(self.component)
        else { unreachable!() };

        world.tables[table_id].sparse_set.dense_values()
            .column(comp_idx).iter()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentRefMut {
    pub(super) component: ComponentEntity,
}

impl QueryParameterImpl for ComponentRefMut {
    type CreationConfig = ComponentEntity;
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId> + 'w;
    type ArchetypMatchBool = Bool;

    type Value<'a> = dynvec::DynVecValueRefMut<'a>;

    fn new(world: &World, component: ComponentEntity) -> Self {
        // TODO
        assert!(world.component_fragments_tables(component));
        Self {
            component,
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        let comp = self.component;
        world.archetypes.iter()
            .filter(move |(_, archetyp)| archetyp.components.has(comp))
            .map(|(aid, _)| aid)
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        world.archetypes[archetyp_id].components.has(self.component).into()
    }
}

pub type HasComponent = super::NoFetch<ComponentRef>;
