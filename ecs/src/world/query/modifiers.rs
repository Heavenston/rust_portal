use super::{
    QueryParameterImpl, QueryParameterImmutableImpl, QueryParameter,
    ImmutableIterParameters, ImmutableWorldRef, ColumnSelectParameters,
    MutableIterParameters, MutableGetParameters,

    AllQueryError, AnyQueryError,
};
use crate::world::{ ArchetypId, Entity, World };

use utils::prelude::*;
use utils::itertools::{ izip, chain };

// FIXME: Remove its use here (change iter_table to use an ImmutableWorldRef)
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

#[derive(Debug, Clone, Copy)]
pub struct Optional<C>
    where C: QueryParameter,
{
    child: C,
}

impl<C> QueryParameterImpl for Optional<C>
    where C: QueryParameterImpl,
{
    type CreationConfig = C::CreationConfig;
    type CreationError = C::CreationError;

    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId> + use<'w, C>;
    type ArchetypMatchBool = True;

    type TableColumns = Either<C::TableColumns, Empty<usize>>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
    type Value<'a> = Option<C::Value<'a>>;

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Result<Self, Self::CreationError> {
        Ok(Self {
            child: C::new(world, creation_cfg)?,
        })
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        world.archetypes.indices()
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        True
    }

    fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        self.child.match_archetyp(&parameters.world, parameters.archetyp_id).is_true()
            .then(|| self.child.table_columns(parameters))
            .left_or(empty())
    }

    fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        if self.child.match_archetyp(&parameters.world, parameters.archetyp_id).is_true() {
            Either::Left(self.child.iter_table_mut(parameters).map(Some))
        }
        else {
            Either::Right(
                repeat_with(|| None)
                    .take(parameters.table_entities.len())
            )
        }
    }

    fn get_mut<'w, I>(&self, parameters: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    {
        if self.child.match_archetyp(&parameters.world, parameters.archetyp_id).is_true() {
            Some(self.child.get_mut(parameters))
        }
        else {
            None
        }
    }
}

impl<C> QueryParameterImmutableImpl for Optional<C>
    where C: QueryParameterImmutableImpl,
{
    type ValueIterator<'w> = impl Iterator<Item = Option<C::Value<'w>>>;

    fn iter_table<'w>(&self, parameters @ ImmutableIterParameters {
        world, archetyp_id, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.child.match_archetyp(&borrow_world!(world), archetyp_id).is_true()
            .then(|| self.child.iter_table(parameters).map(Some))
            .left_or_else(||
                repeat_with(|| None)
                    .take(ix!(world.tables[table_id].sparse_map.len()))
            )
    }

    fn get<'w>(&self, parameters @ ImmutableIterParameters {
        world, archetyp_id, table_id, ..
    }: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
        self.child.match_archetyp(&borrow_world!(world), archetyp_id).is_true()
            .then(|| self.child.get(parameters, entity))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoFetch<C>
    where C: QueryParameter,
{
    child: C,
}

impl<C> QueryParameterImpl for NoFetch<C>
    where C: QueryParameterImpl,
{
    type CreationConfig = C::CreationConfig;
    type CreationError = C::CreationError;

    type ArchetypIterator<'s, 'w> = C::ArchetypIterator<'s, 'w>;
    type ArchetypMatchBool = C::ArchetypMatchBool;

    type TableColumns = Empty<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = RepeatN<()>;
    type Value<'a> = ();

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Result<Self, Self::CreationError> {
        Ok(Self {
            child: C::new(world, creation_cfg)?,
        })
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        C::matching_archetypes(&self.child, world)
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        C::match_archetyp(&self.child, world, archetyp_id)
    }

    fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        empty()
    }

    fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        repeat_n((), parameters.table_entities.len())
    }

    fn get_mut<'w, I>(&self, parameters: &mut MutableGetParameters<'w, I>)
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    { }
}

impl<C> QueryParameterImmutableImpl for NoFetch<C>
    where C: QueryParameterImmutableImpl,
{
    type ValueIterator<'a> = RepeatN<()>;

    fn iter_table(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'_>) -> RepeatN<()> {
        repeat_n((), ix!(world.tables[table_id].sparse_map.len()))
    }

    fn get(&self, parameters: ImmutableIterParameters<'_>, entity: Entity) { }
}

#[derive(Debug, Clone, Copy)]
pub struct Not<C>
    where C: QueryParameter,
{
    child: C,
}

impl<C> QueryParameterImpl for Not<C>
    where C: QueryParameterImpl,
{
    type CreationConfig = C::CreationConfig;
    type CreationError = C::CreationError;

    type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId>;
    type ArchetypMatchBool = BoolNot<C::ArchetypMatchBool>;

    type TableColumns = Empty<usize>;

    type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = RepeatN<()>;
    type Value<'a> = ();

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Result<Self, Self::CreationError> {
        Ok(Self {
            child: C::new(world, creation_cfg)?,
        })
    }

    fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
        let world = world.reborrow();

        world.archetypes.indices()
            .filter(move |&archetyp_id| self.match_archetyp(&world, archetyp_id).is_true())
    }

    fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        C::match_archetyp(&self.child, world, archetyp_id).not()
    }

    fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
        empty()
    }

    fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
        where I: Iterator<Item = &'w mut dynvec::DynVec>,
    {
        repeat_n((), parameters.table_entities.len())
    }

    fn get_mut<'w, I>(&self, parameters: &mut MutableGetParameters<'w, I>)
        where I: Iterator<Item = &'w mut dynvec::DynVec>
    { }
}

// Always immutable event with mutable parameter (the value is dropped)
impl<C> QueryParameterImmutableImpl for Not<C>
    where C: QueryParameterImpl,
{
    type ValueIterator<'a> = RepeatN<()>;

    fn iter_table(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'_>) -> RepeatN<()> {
        repeat_n((), ix!(world.tables[table_id].sparse_map.len()))
    }

    fn get(&self, parameters: ImmutableIterParameters<'_>, entity: Entity) { }
}

type ErrAnd<A, B> = <A as AnyQueryError>::And<B>;

macro_rules! err_and_all {
    ($first: ty) => { $first };
    ($first: ty $(, $rest: ty)+) => {
        ErrAnd<$first, err_and_all!($($rest),*)>
    };
}

macro_rules! bool_or_all {
    ($first: ty) => { Bool<<$first as PartialBool>::T, <$first as PartialBool>::F> };
    ($first: ty $(, $rest: ty)+) => {
        BoolOr<$first, bool_or_all!($($rest),*)>
    };
}

macro_rules! bool_and_all {
    ($first: ty) => { Bool<<$first as PartialBool>::T, <$first as PartialBool>::F> };
    ($first: ty $(, $rest: ty)+) => {
        BoolAnd<$first, bool_and_all!($($rest),*)>
    };
}

macro_rules! bool_or_else {
    ($first: expr) => { $first.into_bool() };
    ($first: expr $(, $rest: expr)+) => {
        PartialBool::or_else($first, || bool_or_else!($($rest),*))
    };
}

macro_rules! bool_and_then {
    () => { True.into_bool() };
    ($first: expr) => { $first.into_bool() };
    ($first: expr $(, $rest: expr)+) => {
        PartialBool::and_then($first, || bool_and_then!($($rest),*))
    };
}

macro_rules! bool_and_then_skip_first {
    ($first: expr $(, $rest: expr)*) => {
        bool_and_then!($($rest),*)
    };
}

// Like itertools::izip but when there is only one element it wraps it
// into a single element tuple
macro_rules! special_izip {
    ($val: expr) => { ($val).map(|val| (val,)) };
    ($($vals:expr),*) => { izip!($($vals),*) };
}

// macro_rules! skip_first {
//     ($i: tt $(, $T:tt)*) => { $($T),* };
// }

#[derive(Default, Debug, Clone, Copy)]
pub struct And<Tuple> {
    tuple: Tuple,
}

macro_rules! impl_and {
    ($($T:ident),*) => {
        impl<$($T,)*> QueryParameterImpl for And<($($T,)*)>
            where $($T: QueryParameterImpl,)*
        {
            type CreationConfig = ($($T::CreationConfig,)*);
            type CreationError = err_and_all!($($T::CreationError),*);

            type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId>;
            type ArchetypMatchBool = bool_and_all!($($T::ArchetypMatchBool),*);

            type TableColumns = impl Iterator<Item = usize>;

            type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = impl Iterator<Item = Self::Value<'a>>;
            type Value<'a> = ($($T::Value<'a>,)*);

            fn new(world: &World, creation_cfg: Self::CreationConfig) -> Result<Self, Self::CreationError> {
                Ok(Self {
                    tuple: ($($T::new(world, creation_cfg.${index()})?,)*),
                })
            }

            fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
                let world = world.reborrow();

                // FIXME: There could be a strategy as to how we chose which one
                // we use here
                self.tuple.0.matching_archetypes(&world)
                    .filter(move |&archetyp_id| bool_and_then_skip_first!($($T::match_archetyp(&self.tuple.${index()}, &world, archetyp_id)),*).is_true())
            }

            fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
                bool_and_then!($($T::match_archetyp(&self.tuple.${index()}, world, archetyp_id)),*)
            }

            fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
                chain!($($T::table_columns(&self.tuple.${index()}, parameters)),*)
            }
 
            fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
                where I: Iterator<Item = &'w mut dynvec::DynVec>,
            {
                special_izip!($($T::iter_table_mut(&self.tuple.${index()}, parameters)),*)
            }

            fn get_mut<'w, I>(&self, parameters: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
                where I: Iterator<Item = &'w mut dynvec::DynVec>
            {
                ($($T::get_mut(&self.tuple.${index()}, parameters),)*)
            }
        }

        impl<$($T,)*> QueryParameterImmutableImpl for And<($($T,)*)>
            where $($T: QueryParameterImmutableImpl,)*
        {
            type ValueIterator<'a> = impl Iterator<Item = Self::Value<'a>>;

            fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
                special_izip!($($T::iter_table(&self.tuple.${index()}, parameters)),*)
            }

            fn get<'w>(&self, parameters: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
                ($($T::get(&self.tuple.${index()}, parameters, entity),)*)
            }
        }
    };
}
variadics_please::all_tuples!(impl_and, 1, 3, T);

#[derive(Default, Debug, Clone, Copy)]
pub struct Or<Tuple> {
    tuple: Tuple,
}

macro_rules! impl_or {
    ($($T:ident),*) => {
        impl<$($T,)*> QueryParameterImpl for Or<($($T,)*)>
            where $($T: QueryParameterImpl,)*
        {
            type CreationConfig = ($($T::CreationConfig,)*);
            type CreationError = AllQueryError;

            type ArchetypIterator<'s, 'w> = impl Iterator<Item = ArchetypId>;
            type ArchetypMatchBool = bool_or_all!($($T::ArchetypMatchBool),*);

            type TableColumns = either_of!($($T::TableColumns),*);

            type ValueMutIterator<'a, I: Iterator<Item = &'a mut dynvec::DynVec>> = EitherIterator<either_of!($($T::ValueMutIterator<'a, I>),*)>;
            type Value<'a> = either_of!($($T::Value<'a>),*);

            fn new(world: &World, creation_cfg: Self::CreationConfig) -> Result<Self, Self::CreationError> {
                Ok(Self {
                    tuple: ($($T::new(world, creation_cfg.${index()})
                        .map_err(Into::<AllQueryError>::into)?,)*),
                })
            }

            fn matching_archetypes<'s, 'w>(&'s self, world: &ImmutableWorldRef<'w>) -> Self::ArchetypIterator<'s, 'w> {
                let world = world.reborrow();

                // FIXME: Better algorithm ?
                world.archetypes.indices()
                    .filter(move |&archetyp_id| {
                        bool_or_else!($($T::match_archetyp(&self.tuple.${index()}, &world, archetyp_id)),*).is_true()
                    })
            }

            fn match_archetyp(&self, world: &ImmutableWorldRef<'_>, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
                bool_or_else!($($T::match_archetyp(&self.tuple.${index()}, world, archetyp_id)),*)
            }

            fn table_columns(&self, parameters: &ColumnSelectParameters<'_>) -> Self::TableColumns {
                $(if $T::match_archetyp(&self.tuple.${index()}, &parameters.world, parameters.archetyp_id).is_true() {
                    EitherFor::<${index()}>::either_from(
                        $T::table_columns(&self.tuple.${index()}, parameters)
                    )
                } else)* {
                    unreachable!()
                }
            }

            fn iter_table_mut<'w, I>(&self, parameters: &mut MutableIterParameters<'w, I>) -> Self::ValueMutIterator<'w, I>
                where I: Iterator<Item = &'w mut dynvec::DynVec>,
            {
                EitherIterator(
                    $(if $T::match_archetyp(&self.tuple.${index()}, &parameters.world, parameters.archetyp_id).is_true() {
                        EitherFor::<${index()}>::either_from(
                            $T::iter_table_mut(&self.tuple.${index()}, parameters)
                        )
                    } else)* {
                        unreachable!()
                    }
                )
            }

            fn get_mut<'w, I>(&self, parameters: &mut MutableGetParameters<'w, I>) -> Self::Value<'w>
                where I: Iterator<Item = &'w mut dynvec::DynVec>
            {
                $(if $T::match_archetyp(&self.tuple.${index()}, &borrow_world!(parameters.world), parameters.archetyp_id).is_true() {
                    EitherFor::<${index()}>::either_from(
                        $T::get_mut(&self.tuple.${index()}, parameters)
                    )
                } else)* {
                    unreachable!()
                }
            }
        }

        impl<$($T,)*> QueryParameterImmutableImpl for Or<($($T,)*)>
            where $($T: QueryParameterImmutableImpl,)*
        {
            type ValueIterator<'a> = EitherIterator<either_of!($($T::ValueIterator<'a>),*)>;

            fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
                EitherIterator(
                    $(if $T::match_archetyp(&self.tuple.${index()}, &borrow_world!(parameters.world), parameters.archetyp_id).is_true() {
                        EitherFor::<${index()}>::either_from(
                            $T::iter_table(&self.tuple.${index()}, parameters)
                        )
                    } else)* {
                        unreachable!()
                    }
                )
            }

            fn get<'w>(&self, parameters: ImmutableIterParameters<'w>, entity: Entity) -> Self::Value<'w> {
                $(if $T::match_archetyp(&self.tuple.${index()}, &borrow_world!(parameters.world), parameters.archetyp_id).is_true() {
                    EitherFor::<${index()}>::either_from(
                        $T::get(&self.tuple.${index()}, parameters, entity)
                    )
                } else)* {
                    unreachable!()
                }
            }
        }
    };
}
variadics_please::all_tuples!(impl_or, 1, 3, T);

pub type Xor<Tuple> = And<(Or<Tuple>, Not<And<Tuple>>)>;
