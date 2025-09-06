use super::{
    QueryParameterImpl,
    QueryParameterImmutableImpl,
    runtime_parameters::*
};
use crate::world::{
    component::{ Component }, ArchetypId, EntityIndex, World
};

use std::marker::PhantomData;
use derive_where::derive_where;

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
    type RequiresPerEntityMatchingBool = bool;
    type ValueMut<'a> = &'a C;
    type ArchetypMatch = <ComponentRef as QueryParameterImpl>::ArchetypMatch;
    type ArchetypMatchError = ();

    fn new(world: &World, (): ()) -> Self {
        Self {
            _component_type: PhantomData,
            child: world.try_component::<C>().map(|component| ComponentRef::new(world, component)),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.child.is_some_and(|has_component| has_component.requires_per_entity_matching())
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<usize, ()> {
        self.child.ok_or(())?.match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        matched: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        self.child.is_some_and(|has_component| has_component.match_entity(world, matched, entity))
    }
}

impl<C> QueryParameterImmutableImpl for Ref<C>
    where C: Component,
{
    type Value<'a> = &'a C;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        matched: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        self.child.as_ref().unwrap()
            .get(world, matched, archetyp_id, entity)
            .as_typed::<C>().expect("Must be the correct type")
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
    type RequiresPerEntityMatchingBool = bool;
    type ValueMut<'a> = &'a mut C;
    type ArchetypMatch = <ComponentRefMut as QueryParameterImpl>::ArchetypMatch;
    type ArchetypMatchError = ();

    fn new(world: &World, (): ()) -> Self {
        Self {
            _component_type: PhantomData,
            child: world.try_component::<C>().map(|component| ComponentRefMut::new(world, component)),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.child.is_some_and(|has_component| has_component.requires_per_entity_matching())
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<usize, ()> {
        self.child.ok_or(())?.match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        matched: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        self.child.is_some_and(|has_component| has_component.match_entity(world, matched, entity))
    }
}
