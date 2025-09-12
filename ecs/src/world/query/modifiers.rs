//! Implementing here [`QueryParameter`]s that modify other parameters

use std::iter::{ repeat_n, repeat_with, RepeatN };

use super::{
    QueryParameterImpl, QueryParameterImmutableImpl,
    QueryParameter, QueryParameterImmutable,
    ImmutableIterParameters,
};
use crate::world::{ ArchetypId, World };

use utils::prelude::*;
use utils::itertools::izip;

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
    type ArchetypIterator<'a> = impl Iterator<Item = ArchetypId> + 'a
        where Self: 'a;
    type ArchetypMatchBool = True;

    type Value<'a> = Option<C::Value<'a>>;

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
        Self {
            child: C::new(world, creation_cfg),
        }
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

impl<C> QueryParameterImmutableImpl for Optional<C>
    where C: QueryParameterImmutableImpl,
{
    type ValueIterator<'w> = impl Iterator<Item = Option<C::Value<'w>>>;

    fn iter_table<'w>(&self, parameters @ ImmutableIterParameters {
        world, archetyp_id, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.child.match_archetyp(world, archetyp_id).is_true()
            .then(|| self.child.iter_table(parameters).map(Some))
            .left_or_else(||
                repeat_with(|| None)
                    .take(ix!(world.tables[table_id].sparse_set.len()))
            )
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
    type ArchetypIterator<'w> = C::ArchetypIterator<'w>
        where Self: 'w;
    type ArchetypMatchBool = C::ArchetypMatchBool;

    type Value<'a> = ();

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
        Self {
            child: C::new(world, creation_cfg),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        C::matching_archetypes(&self.child, world)
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        C::match_archetyp(&self.child, world, archetyp_id)
    }
}

impl<C> QueryParameterImmutableImpl for NoFetch<C>
    where C: QueryParameterImmutableImpl,
{
    type ValueIterator<'a> = RepeatN<()>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        repeat_n((), ix!(world.tables[table_id].sparse_set.len()))
    }
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
    type ArchetypIterator<'w> = impl Iterator<Item = ArchetypId> + 'w
        where Self: 'w;
    type ArchetypMatchBool = BoolNot<C::ArchetypMatchBool>;

    type Value<'a> = ();

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
        Self {
            child: C::new(world, creation_cfg),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        world.archetypes.indices()
            .filter(|&archetyp_id| self.match_archetyp(world, archetyp_id).is_true())
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        C::match_archetyp(&self.child, world, archetyp_id).not()
    }
}

// Always immutable event with mutable parameter (the value is dropped)
impl<C> QueryParameterImmutableImpl for Not<C>
    where C: QueryParameterImpl,
{
    type ValueIterator<'a> = RepeatN<()>;

    fn iter_table<'w>(&self, ImmutableIterParameters {
        world, table_id, ..
    }: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        repeat_n((), ix!(world.tables[table_id].sparse_set.len()))
    }
}

mod private {
    use super::*;

    pub trait QueryParameterTupleImpl: Sized {
        type CreationConfig;

        type AndArchetypIterator<'a>: Iterator<Item = ArchetypId> + 'a
            where Self: 'a;
        type AndArchetypMatchBool: PartialBool;

        type OrArchetypIterator<'a>: Iterator<Item = ArchetypId> + 'a
            where Self: 'a;
        type OrArchetypMatchBool: PartialBool;

        type AndValue<'a>;
        type OrValue<'a>;

        fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self;

        fn and_matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::AndArchetypIterator<'r>
            where 's: 'r, 'w: 'r;
        fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::AndArchetypMatchBool;

        fn or_matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::OrArchetypIterator<'r>
            where 's: 'r, 'w: 'r;
        fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::OrArchetypMatchBool;
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
        () => { False.into_bool() };
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

    macro_rules! impl_query_parameter_tuple {
        ($($T:ident),*) => {
            impl<$($T),*> QueryParameterTupleImpl for ($($T,)*)
                where $($T: QueryParameter,)*
            {
                type CreationConfig = ($($T::CreationConfig,)*);

                type AndArchetypIterator<'a> = impl Iterator<Item = ArchetypId>
                    where Self: 'a;
                type AndArchetypMatchBool = bool_and_all!($($T::ArchetypMatchBool),*);

                type OrArchetypIterator<'a> = impl Iterator<Item = ArchetypId>
                    where Self: 'a;
                type OrArchetypMatchBool = bool_or_all!($($T::ArchetypMatchBool),*);

                type AndValue<'a> = ($($T::Value<'a>,)*);
                type OrValue<'a> = either_of!($($T::Value<'a>),*);

                fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
                    ($($T::new(world, creation_cfg.${index()}),)*)
                }

                fn and_matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::AndArchetypIterator<'r>
                    where 's: 'r, 'w: 'r
                {
                    // FIXME: There could be a strategy as to how we chose which one
                    // we use here
                    self.0.matching_archetypes(world)
                        .filter(|&archetyp_id| bool_and_then_skip_first!($($T::match_archetyp(&self.${index()}, world, archetyp_id)),*).is_true())
                }

                fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::AndArchetypMatchBool {
                    bool_and_then!($($T::match_archetyp(&self.${index()}, world, archetyp_id)),*)
                }

                fn or_matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::OrArchetypIterator<'r>
                    where 's: 'r, 'w: 'r
                {
                    // FIXME: There could be a strategy as to how we chose which one
                    // we use here
                    world.archetypes.indices()
                        .filter(|&archetyp_id| {
                            bool_or_else!($($T::match_archetyp(&self.${index()}, world, archetyp_id)),*).is_true()
                        })
                }

                fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::OrArchetypMatchBool {
                    bool_or_else!($($T::match_archetyp(&self.${index()}, world, archetyp_id)),*)
                }
            }
        };
    }
    variadics_please::all_tuples!(impl_query_parameter_tuple, 1, 15, T);

    pub trait QueryParameterTupleImmutableImpl: QueryParameterTupleImpl {
        type AndValueIterator<'a>: Iterator<Item = Self::AndValue<'a>>;
        type OrValueIterator<'a>: Iterator<Item = Self::OrValue<'a>>;

        fn and_iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::AndValueIterator<'w>;
        fn or_iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::OrValueIterator<'w>;
    }

    macro_rules! impl_query_parameter_tuple_immutable {
        ($($T:ident),*) => {
            impl<$($T),*> QueryParameterTupleImmutableImpl for ($($T,)*)
                where $($T: QueryParameterImmutable,)*
            {
                type AndValueIterator<'a> = impl Iterator<Item = Self::AndValue<'a>>;
                type OrValueIterator<'a> = impl Iterator<Item = Self::OrValue<'a>>;

                fn and_iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::AndValueIterator<'w> {
                    special_izip!(
                        $($T::iter_table(&self.${index()}, parameters)),*
                    )
                }

                fn or_iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::OrValueIterator<'w> {
                    let archetyp_id = parameters.archetyp_id;
                    let either_iter: either_of!($($T::ValueIterator<'w>),*) =
                        $(if $T::match_archetyp(&self.${index()}, parameters.world, parameters.archetyp_id).is_true() {
                            EitherFor::<${index()}>::either_from($T::iter_table(&self.${index()}, parameters))
                        } else )* {
                            unreachable!()
                        };

                    either_iter.into_either_iter()
                }
            }
        };
    }
    variadics_please::all_tuples!(impl_query_parameter_tuple_immutable, 1, 15, T);

}
use private::*;

trait_alias!(pub trait QueryParameterTuple = 'static + QueryParameterTupleImpl);
trait_alias!(pub trait QueryParameterTupleImmutable = QueryParameterTuple + QueryParameterTupleImmutableImpl);

#[derive(Default, Debug, Clone, Copy)]
pub struct And<Tuple>
    where Tuple: QueryParameterTuple,
{
    tuple: Tuple,
}

impl<Tuple> QueryParameterImpl for And<Tuple>
    where Tuple: QueryParameterTuple,
{
    type CreationConfig = Tuple::CreationConfig;
    type ArchetypIterator<'a> = Tuple::AndArchetypIterator<'a>
        where Self: 'a;
    type ArchetypMatchBool = Tuple::AndArchetypMatchBool;

    type Value<'a> = Tuple::AndValue<'a>;

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
        Self {
            tuple: Tuple::new(world, creation_cfg),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        self.tuple.and_matching_archetypes(world)
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.tuple.and_match_archetyp(world, archetyp_id)
    }
}

impl<Tuple> QueryParameterImmutableImpl for And<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type ValueIterator<'a> = Tuple::AndValueIterator<'a>;

    fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.tuple.and_iter_table(parameters)
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct Or<Tuple>
    where Tuple: QueryParameterTuple,
{
    tuple: Tuple,
}

impl<Tuple> QueryParameterImpl for Or<Tuple>
    where Tuple: QueryParameterTuple,
{
    type CreationConfig = Tuple::CreationConfig;
    type ArchetypIterator<'a> = Tuple::OrArchetypIterator<'a>
        where Self: 'a;
    type ArchetypMatchBool = Tuple::OrArchetypMatchBool;

    type Value<'a> = Tuple::OrValue<'a>;

    fn new(world: &World, creation_cfg: Self::CreationConfig) -> Self {
        Self {
            tuple: Tuple::new(world, creation_cfg),
        }
    }

    fn matching_archetypes<'s, 'w, 'r>(&'s self, world: &'w World) -> Self::ArchetypIterator<'r>
        where 's: 'r, 'w: 'r
    {
        self.tuple.or_matching_archetypes(world)
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Self::ArchetypMatchBool {
        self.tuple.or_match_archetyp(world, archetyp_id)
    }
}

impl<Tuple> QueryParameterImmutableImpl for Or<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type ValueIterator<'a> = Tuple::OrValueIterator<'a>;

    fn iter_table<'w>(&self, parameters: ImmutableIterParameters<'w>) -> Self::ValueIterator<'w> {
        self.tuple.or_iter_table(parameters)
    }
}

pub type Xor<Tuple> = And<(Or<Tuple>, Not<And<Tuple>>)>;
