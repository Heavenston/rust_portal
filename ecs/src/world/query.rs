#![allow(dead_code)]
#![allow(unused_variables)]

mod runtime_parameters;
pub use runtime_parameters::*;
mod typed_parameters;
pub use typed_parameters::*;
mod modifiers;
pub use modifiers::*;

use super::{ World, TableId, ArchetypId, Entity };

use std::iter::empty;
use utils::prelude::*;

mod private {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    pub struct ImmutableIterParameters<'w> {
        pub world: &'w World,
        pub archetyp_id: ArchetypId,
        pub table_id: TableId,
    }

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
        type ArchetypIterator<'a>: Iterator<Item = ArchetypId> + 'a
            where Self: 'a;
        type ArchetypMatchBool: PartialBool;

        type Value<'a>;

        fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self;
        fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
            where 's: 'r, 'w: 'r;
        fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool;
    }

    pub trait QueryParameterImmutableImpl: QueryParameterImpl {
        type ValueIterator<'a>: Iterator<Item = Self::Value<'a>>;

        fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w>;
    }
}
use private::{
    OnlyUnit, QueryParameterImpl, ImmutableIterParameters,
    QueryParameterImmutableImpl
};

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

#[derive(Clone)]
struct CachedQueryData {
    matching_archetypes: BitSet<ArchetypId>,
}

pub struct Query<P: QueryParameter> {
    parameters: P,
    cache: Option<CachedQueryData>,
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

    fn generate_cache(&self, world: &World) -> CachedQueryData {
        let matching_archetypes: BitSet<ArchetypId> = self.parameters.matching_archetypes(world).collect();
        if cfg!(test) {
            assert!(matching_archetypes.iter()
                .all(|archetyp_id| self.parameters.match_archetyp(world, archetyp_id).is_true()));
        }

        debug_assert!(
            matching_archetypes.iter()
                .map(|archetyp_id| world.archetypes[archetyp_id].table_id)
                .all_unique(),
            "Queries do not support non-table fragmenting component yet...",
        );

        CachedQueryData {
            matching_archetypes,
        }
    }

    pub fn cache_matches(&mut self, world: &World) {
        if self.cache.is_some() { return }

        self.cache = Some(self.generate_cache(world));
    }

    fn archetypes<'a>(&'a self, world: &'a World) -> impl Iterator<Item = ArchetypId> + 'a {
        self.cache.as_ref()
            .map(|cache| cache.matching_archetypes.iter())
            .left_or_else(|| self.parameters.matching_archetypes(world))
    }

    fn tables<'a>(&'a self, world: &'a World) -> impl Iterator<Item = (ArchetypId, TableId)> + 'a {
        self.archetypes(world)
            .map(|archetyp_id| (archetyp_id, world.archetypes[archetyp_id].table_id))
    }

    pub fn entities<'a>(&'a self, world: &'a World) -> impl Iterator<Item = Entity> + 'a {
        self.archetypes(world)
            .flat_map(|archetyp| world.archetypes[archetyp].entities.iter())
            .map(|index| Entity::new(index, world.generation_at_index(index)))
    }

    pub fn iter<'s, 'w, 'r>(&'s self, world: &'w World) -> impl Iterator<Item = P::Value<'w>> + 'r
        where P: QueryParameterImmutable,
              's: 'r, 'w: 'r,
    {
        self.tables(world)
            .flat_map(|(archetyp_id, table_id)| self.parameters.iter_table(ImmutableIterParameters {
                world, archetyp_id, table_id,
            }))
    }

    pub fn iter_mut<'w>(&self, world: &'w mut World) -> impl Iterator<Item = P::Value<'w>> {
        todo!();
        empty()
    }

    pub fn get<'a, 'b>(&'a self, world: &'b World, entity: Entity) -> Result<P::Value<'b>, QueryGetError>
        where P: QueryParameterImmutable,
    {
        if !world.alive(entity) {
            return Err(QueryGetError::EntityIsNotAlive { entity });
        }

        todo!()
    }
}
