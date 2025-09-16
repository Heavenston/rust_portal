#![allow(dead_code)]
#![allow(unused_variables)]

mod runtime_parameters;
pub use runtime_parameters::*;
mod typed_parameters;
pub use typed_parameters::*;
mod modifiers;
pub use modifiers::*;

use super::{
    World, TableId, ArchetypId, Table, Archetyp,
    entity_storage::{ EntityStorage, Entity, EntityIndex },
    ComponentSet, ComponentEntity,
};
use crate::index_map::IndexMap;

use std::collections::HashMap;
use std::any::TypeId;
use utils::prelude::*;

macro_rules! borrow_world {
    ($world: expr) => {
        ImmutableWorldRef {
            entity_storage: &$world.entity_storage,
            entities_archetypes: &$world.entities_archetypes,
            archetypes: &$world.archetypes,
            components_to_archetypes: &$world.components_to_archetypes,
            components_typeid_to_entity: &$world.components_typeid_to_entity,
            components_entity_to_typeid: &$world.components_entity_to_typeid,
            components_set_to_archetyp: &$world.components_set_to_archetyp,
            components_set_to_table: &$world.components_set_to_table,
        }
    };
}

mod private {
    use super::*;

    #[derive(Debug)]
    pub struct ImmutableWorldRef<'w> {
        pub(super) entity_storage: &'w EntityStorage,
        pub(super) entities_archetypes: &'w IndexMap<ArchetypId, EntityIndex>,
        pub(super) archetypes: &'w IndexMap<Archetyp, ArchetypId>,
        pub(super) components_to_archetypes: &'w IndexMap<BitSet<ArchetypId>, EntityIndex>,
        pub(super) components_typeid_to_entity: &'w HashMap<TypeId, ComponentEntity>,
        pub(super) components_entity_to_typeid: &'w HashMap<ComponentEntity, TypeId>,
        pub(super) components_set_to_archetyp: &'w HashMap<ComponentSet, ArchetypId>,
        pub(super) components_set_to_table: &'w HashMap<ComponentSet, TableId>,
    }

    impl<'w> ImmutableWorldRef<'w> {
        pub fn reborrow(&self) -> Self {
            Self {
                entity_storage: self.entity_storage,
                entities_archetypes: self.entities_archetypes,
                archetypes: self.archetypes,
                components_to_archetypes: self.components_to_archetypes,
                components_typeid_to_entity: self.components_typeid_to_entity,
                components_entity_to_typeid: self.components_entity_to_typeid,
                components_set_to_archetyp: self.components_set_to_archetyp,
                components_set_to_table: self.components_set_to_table,
            }
        }
    }

    #[derive(Debug)]
    pub struct ColumnSelectParameters<'w> {
        pub(super) world: ImmutableWorldRef<'w>,
        pub(super) archetyp_id: ArchetypId,
        pub(super) archetyp: &'w Archetyp,
        pub(super) table_id: TableId,
        pub(super) table: &'w Table,
    }

    #[derive(Debug)]
    pub struct MutableIterParameters<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        pub(super) world: ImmutableWorldRef<'w>,
        pub(super) archetyp_id: ArchetypId,
        pub(super) archetyp: &'w Archetyp,
        pub(super) table_id: TableId,
        pub(super) table_entities: &'w [EntityIndex],
        pub(super) table_columns: I,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct ImmutableIterParameters<'w> {
        pub(super) world: &'w World,
        pub(super) archetyp_id: ArchetypId,
        pub(super) table_id: TableId,
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

    // NOTE: 'static is Required to make the GATs work easily, no query
    // parameter uses lifetimes but if that usecase comes up, it probably would
    // require a *lot* of rust tinkering to make this trait work
    pub trait QueryParameterImpl: 'static /* */ {
        type CreationConfig;
        type ArchetypIterator<'s, 'w>: Iterator<Item = ArchetypId>;
        type ArchetypMatchBool: PartialBool;

        type TableColumns: Iterator<Item = usize>;

        type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>>: Iterator<Item = Self::Value<'a>>;
        type Value<'a>;

        fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self;
        fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w>;
        fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool;

        fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns;

        /// Only consume from parameters.table_columns the *exact* same amount of
        /// columns returned by Self::table_column
        fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
            where I: Iterator<Item = &'w mut dynvec::DynVec>;
    }

    pub trait QueryParameterImmutableImpl: QueryParameterImpl {
        type ValueIterator<'a>: Iterator<Item = Self::Value<'a>>;

        fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w>;
    }
}
use private::*;

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

#[derive(Debug, Clone)]
struct CachedQueryData {
    matching_archetypes: BitSet<ArchetypId>,
}

#[derive(Debug)]
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
        let world = &borrow_world!(world);
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

    fn archetypes<'s, 'b, 'w>(&'s self, world: &'b ImmutableWorldRef<'w>) -> impl Iterator<Item = ArchetypId> + use<'s, 'w, P> {
        let world = world.reborrow();

        self.cache.as_ref()
            .map(|cache| cache.matching_archetypes.iter())
            .left_or_else(move || self.parameters.matching_archetypes(&world))
    }

    fn tables<'a, 'w>(&'a self, world: &'w World) -> impl Iterator<Item = (ArchetypId, TableId)> + use<'a, 'w, P> {
        self.archetypes(&borrow_world!(world))
            .map(|archetyp_id| (archetyp_id, world.archetypes[archetyp_id].table_id))
    }

    pub fn entities<'a>(&'a self, world: &'a World) -> impl Iterator<Item = Entity> {
        self.archetypes(&borrow_world!(world))
            .flat_map(|archetyp| world.archetypes[archetyp].entities.iter())
            .map(|index| Entity::new(index, world.generation_at_index(index)))
    }

    pub fn iter<'s, 'w>(&'s self, world: &'w World) -> impl Iterator<Item = P::Value<'w>>
        where P: QueryParameterImmutable,
    {
        self.tables(world)
            .flat_map(|(archetyp_id, table_id)| self.parameters.iter_table(ImmutableIterParameters {
                world, archetyp_id, table_id,
            }))
    }

    pub fn iter_mut<'s, 'w>(&'s self, world: &'w mut World) -> impl Iterator<Item = P::Value<'w>> {
        let tables = &mut world.tables;
        let world = borrow_world!(world);

        // FIXME: Archetyp ids are sorted but when getting their table
        // ids, if any sort of id reuse is done they will **NOT** be sorted
        // so this will break at some point.
        let matched_tables = tables.get_sorted_disjoint_mut(
            self.archetypes(&world)
                .map(|archetyp_id| {
                    let table_id = world.archetypes[archetyp_id].table_id;
                    (archetyp_id, table_id)
                })
        );

        matched_tables.flat_map(move |(archetyp_id, table_id, table)| {
            let archetyp = &world.archetypes[archetyp_id];

            let asked_columns = self.parameters.table_columns(&ColumnSelectParameters {
                world: world.reborrow(),
                archetyp_id,
                archetyp,
                table_id,
                table: &table,
            });
            let (entities, dense_values) = table.sparse_set.split();

            // FIXME: Annoying allocation here, not sure how to fix it
            // especialy without unsafe
            let mut columns_refs = dense_values.columns_mut().iter_mut()
                .map(Some)
                .collect_vec();

            self.parameters.iter_table_mut(&mut MutableIterParameters {
                world: world.reborrow(),
                archetyp_id,
                archetyp,
                table_id,
                table_entities: entities.as_slice(),
                table_columns: asked_columns.map(move |column_idx| {
                    columns_refs[column_idx].take().expect("Cannot use a component column multiple time in the same query")
                }),
            })
        })
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
