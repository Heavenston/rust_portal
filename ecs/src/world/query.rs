#![allow(dead_code)]
#![allow(unused_variables)]

mod parameters;
pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use super::*;

mod private {
    use super::*;

    pub trait QueryParameterImpl {
        type ValueMut<'a>;
        type ArchetypMatch;

        fn new(world: &World) -> Self;

        fn requires_per_entity_matching(&self) -> bool;

        fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch>;

        /// Only called if the entity's archetypes matches (giving the value back).
        /// The entity is guarenteed to be alive.
        fn match_entity(
            &self,
            world: &World,
            archetyp_match: &Self::ArchetypMatch,
            entity: EntityIndex,
        ) -> bool;

        // TODO
        // /// The entity is guarenteed to be alive a matching
        // fn get_mut<'s, 'a>(
        //     &'s self,
        //     world: &'a mut World,
        //     archetyp_match: &Self::ArchetypMatch,
        //     entity: Entity,
        // ) -> Self::ValueMut<'a> {
        //     todo!()
        // }
    }

    pub trait QueryParameterImmutableImpl: QueryParameterImpl {
        type Value<'a>: Copy;

        /// The entity is guarenteed to be alive a matching
        fn get<'s, 'a>(
            &'s self,
            world: &'a World,
            archetyp_match: &Self::ArchetypMatch,
            archetyp_id: ArchetypId,
            entity: EntityIndex,
        ) -> Self::Value<'a>;
    }
}
use private::{ QueryParameterImpl, QueryParameterImmutableImpl };

pub trait QueryParameter = QueryParameterImpl;
pub trait QueryParameterImmutable = QueryParameterImmutableImpl;

#[derive(Debug, thiserror::Error)]
pub enum QueryGetError {
    #[error("Tried to get components of a dead entity {entity}")]
    EntityIsNotAlive {
        entity: Entity,
    },
    #[error("{entity} does not match the query.")]
    NotMatched {
        entity: Entity,
    },
}

pub struct Query<P: QueryParameter> {
    parameters: P,
    requires_per_entity_matching: bool,
    archetypes_matches: Vec<P::ArchetypMatch>,
    archetypes: BitSet<ArchetypId>,
}

impl<P: QueryParameterImpl> Query<P> {
    pub fn new(world: &World) -> Self {
        let parameters = P::new(world);

        // FIXME: With lots of archtyps this could get slow
        let (archetypes, archetypes_matches) = world.archetypes.indices()
            .filter_map(|archetyp_id| Some(archetyp_id).zip(parameters.match_archetyp(world, archetyp_id)))
            .unzip();

        let requires_per_entity_matching = parameters.requires_per_entity_matching();

        Self {
            parameters: P::new(&*world),
            requires_per_entity_matching,
            archetypes_matches,
            archetypes,
        }
    }

    pub(in crate::world) fn archetypes(&self) -> impl Iterator<Item = ArchetypId> {
        self.archetypes.iter()
    }

    pub fn entities(&self, world: &World) -> impl Iterator<Item = Entity> {
        let entities = self.archetypes.iter().enumerate()
            .flat_map(|(archetyp_index, archetyp_id)| world.archetypes[archetyp_id].entities.iter()
                .zip(std::iter::repeat(archetyp_index)));

        // an attempt at an optimization but should change nothing from just
        // checking `self.requires_per_entity_matching` in the function of the filter
        let filter_entities = if self.requires_per_entity_matching {
            Either::Left(entities
                .filter(|&(index, archetyp_index)| {
                    self.parameters.match_entity(world, &self.archetypes_matches[archetyp_index], index)
                }))
        }
        else {
            Either::Right(entities)
        };

        filter_entities
            .map(|(index, _)| index)
            .map(|entity_index| Entity::new(entity_index, world.generation_at_index(entity_index)))
    }

    pub fn get<'a, 'b>(&'a self, world: &'b World, entity: Entity) -> Result<P::Value<'b>, QueryGetError>
        where P: QueryParameterImmutable,
    {
        if !world.alive(entity) {
            return Err(QueryGetError::EntityIsNotAlive { entity });
        }

        let archetyp_id = world.entities_archetypes[entity.index()];
        if !self.archetypes.has(archetyp_id) {
            return Err(QueryGetError::NotMatched { entity });
        }

        let archetyp_match = &self.archetypes_matches[self.archetypes.index_of(archetyp_id)];

        if self.requires_per_entity_matching && !self.parameters.match_entity(world, archetyp_match, entity.index()) {
            return Err(QueryGetError::NotMatched { entity });
        }

        Ok(self.parameters.get(
            world,
            archetyp_match,
            archetyp_id,
            entity.index(),
        ))
    }
}
