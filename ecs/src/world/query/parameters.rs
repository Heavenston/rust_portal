use super::{ QueryParameterImpl, QueryParameterImmutableImpl };
use crate::world::{
    component::{ Component, ComponentEntity }, ArchetypId, Entity, EntityIndex, World
};

use std::marker::PhantomData;
use derive_where::derive_where;
use utils::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct Always;
impl QueryParameterImpl for Always {
    type ValueMut<'a> = ();
    type ArchetypMatch = ();

    fn new(world: &World) -> Always {
        Always
    }

    fn requires_per_entity_matching(&self) -> bool {
        false
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<()> {
        Some(())
    }

    fn match_entity(
        &self,
        world: &World,
        &(): &(),
        entity: EntityIndex,
    ) -> bool {
        unreachable!()
    }
}

impl QueryParameterImmutableImpl for Always {
    type Value<'a> = ();

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        &component_index: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        ()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Never;
impl QueryParameterImpl for Never {
    type ValueMut<'a> = ();
    type ArchetypMatch = !;

    fn new(world: &World) -> Never {
        Never
    }

    fn requires_per_entity_matching(&self) -> bool {
        false
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<!> {
        None
    }

    fn match_entity(
        &self,
        world: &World,
        &_: &!,
        entity: EntityIndex,
    ) -> bool {
        unreachable!()
    }
}

impl QueryParameterImmutableImpl for Never {
    type Value<'a> = ();

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        &component_index: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        ()
    }
}

#[derive_where(Debug, Clone, Copy)]
pub struct Ref<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    component: Option<ComponentEntity>,
    requires_per_entity_matching: bool,
}

impl<C> QueryParameterImpl for Ref<C>
    where C: Component,
{
    type ValueMut<'a> = &'a C;
    type ArchetypMatch = usize;

    fn new(world: &World) -> Self {
        let comp = world.try_component::<C>();
        Self {
            _component_type: PhantomData,
            component: comp,
            requires_per_entity_matching: comp.is_some_and(|comp|
                !world.component_fragments_tables(comp)),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.requires_per_entity_matching
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<usize> {
        let comp = self.component?;
        let index = world.archetypes[archetyp_id].components.index_of(comp)?;

        Some(index)
    }

    fn match_entity(
        &self,
        world: &World,
        _: &usize,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching);
        let comp = self.component.expect("Only sets requires_per_entity_matching if the component exists");
        world.archetypes[world.entities_archetypes[entity]].components.has(comp)
    }
}

impl<C> QueryParameterImmutableImpl for Ref<C>
    where C: Component,
{
    type Value<'a> = &'a C;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        &component_index: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        world.tables[world.archetypes[archetyp_id].table_id].sparse_set
            .get(entity).expect("Is inside")
            .for_component(component_index)
            .as_typed::<C>().expect("Must be the correct type")
    }
}

#[derive_where(Debug, Clone, Copy)]
pub struct RefMut<C>
    where C: Component,
{
    _component_type: PhantomData<fn(C) -> C>,
    component: Option<ComponentEntity>,
    requires_per_entity_matching: bool,
}

impl<C> QueryParameterImpl for RefMut<C>
    where C: Component,
{
    type ValueMut<'a> = &'a mut C;
    type ArchetypMatch = usize;

    fn new(world: &World) -> Self {
        let comp = world.try_component::<C>();
        Self {
            _component_type: PhantomData,
            component: comp,
            requires_per_entity_matching: comp.is_some_and(|comp|
                !world.component_fragments_tables(comp)),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.requires_per_entity_matching
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch> {
        let comp = self.component?;
        let index = world.archetypes[archetyp_id].components.index_of(comp)?;

        Some(index)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching);
        let comp = self.component.expect("Only sets requires_per_entity_matching if the component exists");
        world.archetypes[world.entities_archetypes[entity]].components.has(comp)
    }
}

pub type Has<C> = super::NoFetch<Ref<C>>;

impl QueryParameterImpl for Entity {
    type ValueMut<'a> = Entity;
    type ArchetypMatch = ();

    fn new(world: &World) -> Self {
        default()
    }

    fn requires_per_entity_matching(&self) -> bool {
        false
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<()> {
        Some(())
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        true
    }
}

impl QueryParameterImmutableImpl for Entity {
    type Value<'a> = Entity;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity_index: EntityIndex,
    ) -> Self::Value<'a> {
        Entity::new(entity_index, world.generation_at_index(entity_index))
    }
}
