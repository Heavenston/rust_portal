use super::{ QueryParameterImpl, QueryParameterImmutableImpl };
use crate::world::{
    component::{ ComponentEntity }, ArchetypId, Entity, EntityIndex, World
};

use utils::prelude::*;

impl QueryParameterImpl for Entity {
    type CreationConfig = ();
    type RequiresPerEntityMatchingBool = False;
    type ValueMut<'a> = Entity;
    type ArchetypMatch = ();
    type ArchetypMatchError = !;

    fn new(world: &World, (): ()) -> Self {
        default()
    }

    fn requires_per_entity_matching(&self) -> False {
        False
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<(), !> {
        Ok(())
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

pub type Always = super::NoFetch<Entity>;
pub type Never = super::Not<Always>;

#[derive(Debug, Clone, Copy)]
pub struct ComponentRef {
    component: ComponentEntity,
    requires_per_entity_matching: bool,
}

impl QueryParameterImpl for ComponentRef {
    type CreationConfig = ComponentEntity;
    type RequiresPerEntityMatchingBool = bool;
    type ValueMut<'a> = dynvec::DynVecValueRef<'a>;
    type ArchetypMatch = usize;
    type ArchetypMatchError = ();

    fn new(world: &World, component: ComponentEntity) -> Self {
        Self {
            component,
            requires_per_entity_matching: !world.component_fragments_tables(component),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.requires_per_entity_matching
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<usize, ()> {
        Ok(world.archetypes[archetyp_id].components.index_of(self.component).ok_or(())?)
    }

    fn match_entity(
        &self,
        world: &World,
        _: &usize,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching);
        world.archetypes[world.entities_archetypes[entity]]
            .components.has(self.component)
    }
}

impl QueryParameterImmutableImpl for ComponentRef {
    type Value<'a> = dynvec::DynVecValueRef<'a>;

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
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ComponentRefMut {
    component: ComponentEntity,
    requires_per_entity_matching: bool,
}

impl QueryParameterImpl for ComponentRefMut {
    type RequiresPerEntityMatchingBool = bool;
    type CreationConfig = ComponentEntity;
    type ValueMut<'a> = dynvec::DynVecValueRefMut<'a>;
    type ArchetypMatch = usize;
    type ArchetypMatchError = ();

    fn new(world: &World, component: ComponentEntity) -> Self {
        Self {
            component,
            requires_per_entity_matching: !world.component_fragments_tables(component),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.requires_per_entity_matching
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::ArchetypMatch, ()> {
        Ok(world.archetypes[archetyp_id].components.index_of(self.component).ok_or(())?)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching);
        world.archetypes[world.entities_archetypes[entity]]
            .components.has(self.component)
    }
}

pub type HasComponent = super::NoFetch<ComponentRef>;
