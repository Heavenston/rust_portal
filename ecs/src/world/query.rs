#![allow(dead_code)]
#![allow(unused_variables)]

mod parameters;
use std::iter::{repeat, zip};

pub use parameters::*;
mod modifiers;
pub use modifiers::*;

use super::*;

use utils::itertools::izip;
use utils::prelude::*;

mod private {
    use super::*;

    /// Implemented for all tuples of values that all implement OnlyUnit
    pub trait OnlyUnit { }
    macro_rules! impl_only_unit {
        ($($T: ident),*) => {
            impl<$($T,)*> OnlyUnit for ($($T,)*)
                where $($T: OnlyUnit,)*
            { }
        };
    }
    variadics_please::all_tuples!(impl_only_unit, 0, 16, T);

    pub trait QueryParameterImpl {
        type CreationConfig;
        type ValueMut<'a>;
        type ArchetypMatch: Clone;

        fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self;

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
use private::{ OnlyUnit, QueryParameterImpl, QueryParameterImmutableImpl };

trait_alias!(pub trait QueryParameter = QueryParameterImpl);
trait_alias!(pub trait QueryParameterImmutable = QueryParameterImmutableImpl);

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

struct CachedQueryData<P: QueryParameter> {
    matching_archetypes: BitSet<ArchetypId>,
    match_parameters: Vec<P::ArchetypMatch>,
}

pub struct Query<P: QueryParameter> {
    parameters: P,
    cache: Option<CachedQueryData<P>>,
}

impl<P: QueryParameterImpl> Query<P> {
    pub fn new(world: &World) -> Self
        where P::CreationConfig: OnlyUnit + Default,
    {
        Self::new_with_config(world, default())
    }

    pub fn new_with_config(world: &World, config: P::CreationConfig) -> Self {
        Self {
            parameters: P::new(world, config),
            cache: None,
        }
    }

    fn generate_archetypes(&self, world: &World) -> impl Iterator<Item = (ArchetypId, P::ArchetypMatch)> {
        // FIXME: With lots of archtyps this could get slow
        world.archetypes.indices()
            .filter_map(|archetyp_id| Some(archetyp_id).zip(self.parameters.match_archetyp(world, archetyp_id)))
    }

    pub fn cache_matches(&mut self, world: &World) {
        if self.cache.is_some() { return }

        let (matching_archetypes, match_parameters) = self.generate_archetypes(world).unzip();

        self.cache = Some(CachedQueryData {
            matching_archetypes,
            match_parameters,
        });
    }

    fn archetypes(&self, world: &World) -> impl Iterator<Item = (ArchetypId, Cow<'_, P::ArchetypMatch>)> {
        if let Some(cache) = &self.cache {
            Either::Left(zip(
                &cache.matching_archetypes,
                &cache.match_parameters,
            ).tuple_map_nth::<1, _, _>(Cow::Borrowed))
        }
        else {
            Either::Right(self.generate_archetypes(world)
                .tuple_map_nth::<1, _, _>(Cow::Owned))
        }
    }

    /// Internal iterator into archetypes and entities that matches this query
    fn matched_entities<'s, 'w, 'c>(
        &'s self,
        world: &'w World
    ) -> impl Iterator<Item = (ArchetypId, Cow<'s, P::ArchetypMatch>, EntityIndex)> + 'c
        where 's: 'c, 'w: 'c,
    {
        let requires_per_entity_matching = self.parameters.requires_per_entity_matching();

        self.archetypes(world)
            .flat_map(move |(archetyp_id, matched)| izip!(
                repeat(archetyp_id),
                repeat(matched),
                world.archetypes[archetyp_id].entities.iter(),
            ))
            .filter(move |&(archetyp_id, ref matched, entity)|
                !requires_per_entity_matching ||
                    self.parameters.match_entity(world, &matched, entity)
            )
    }

    pub fn entities<'s, 'w, 'c>(&'s self, world: &'w World) -> impl Iterator<Item = Entity> + 'c
        where 's: 'c, 'w: 'c,
    {
        self.matched_entities(world)
            .map(|(_, _, entity_index)|
                Entity::new(entity_index, world.generation_at_index(entity_index))
            )
    }

    pub fn iter<'s, 'w, 'c>(&'s self, world: &'w World) -> impl Iterator<Item = P::Value<'w>> + 'c
        where P: QueryParameterImmutable,
              's: 'c, 'w: 'c,
    {
        self.matched_entities(world)
            .map(|(archetyp_id, archetyp_match, entity)|
                self.parameters.get(world, &archetyp_match, archetyp_id, entity)
            )
    }

    pub fn iter_mut<'w>(&self, world: &'w World) -> impl Iterator<Item = P::ValueMut<'w>> {
        todo!();
        empty()
    }

    pub fn get<'a, 'b>(&'a self, world: &'b World, entity: Entity) -> Result<P::Value<'b>, QueryGetError>
        where P: QueryParameterImmutable,
    {
        if !world.alive(entity) {
            return Err(QueryGetError::EntityIsNotAlive { entity });
        }

        let archetyp_id = world.entities_archetypes[entity.index()];

        let Some(archetyp_match) = (if let Some(cache) = &self.cache {
            cache.matching_archetypes.has(archetyp_id)
                .then(|| Cow::Borrowed(
                    &cache.match_parameters[cache.matching_archetypes.index_of(archetyp_id)]
                ))
        } else {
            self.parameters.match_archetyp(world, archetyp_id)
                .map(Cow::Owned)
        }) else {
            return Err(QueryGetError::NotMatched { entity });
        };

        if self.parameters.requires_per_entity_matching() &&
            !self.parameters.match_entity(world, &archetyp_match, entity.index())
        {
            return Err(QueryGetError::NotMatched { entity });
        }

        Ok(self.parameters.get(
            world,
            &archetyp_match,
            archetyp_id,
            entity.index(),
        ))
    }
}
