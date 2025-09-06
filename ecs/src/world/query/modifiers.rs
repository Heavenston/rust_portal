//! Implementing here [`QueryParameter`]s that modify other parameters

use super::{
    QueryParameterImpl, QueryParameterImmutableImpl,
    QueryParameter, QueryParameterImmutable,
};
use crate::world::{ ArchetypId, EntityIndex, World };

use utils::prelude::*;

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
    type RequiresPerEntityMatchingBool = False;
    type ValueMut<'a> = Option<C::ValueMut<'a>>;
    type ArchetypMatch = Result<C::ArchetypMatch, C::ArchetypMatchError>;
    type ArchetypMatchError = !;

    fn new(world: &World, config: C::CreationConfig) -> Self {
        Self {
            child: C::new(world, config),
        }
    }

    fn requires_per_entity_matching(&self) -> False {
        // we never need to check per entity as we always match it (but return None)
        False
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::ArchetypMatch, !> {
        Ok(self.child.match_archetyp(world, archetyp_id))
    }

    fn match_entity(
        &self, world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        // should not be called
        unreachable!();
    }
}

impl<C> QueryParameterImmutableImpl for Optional<C>
    where C: QueryParameterImmutableImpl,
{
    type Value<'a> = Option<C::Value<'a>>;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Option<C::Value<'a>> {
        let child_archetyp_match = archetyp_match.as_ref().ok()?;

        if self.child.requires_per_entity_matching().is_true() &&
            !self.child.match_entity(world, child_archetyp_match, entity){
            return None;
        }

        Some(self.child.get(world, child_archetyp_match, archetyp_id, entity))
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
    type RequiresPerEntityMatchingBool = C::RequiresPerEntityMatchingBool;
    type ValueMut<'a> = C::ValueMut<'a>;
    type ArchetypMatch = C::ArchetypMatch;
    type ArchetypMatchError = C::ArchetypMatchError;

    fn new(world: &World, config: C::CreationConfig) -> Self {
        Self {
            child: C::new(world, config),
        }
    }

    fn requires_per_entity_matching(&self) -> C::RequiresPerEntityMatchingBool {
        self.child.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<C::ArchetypMatch, C::ArchetypMatchError> {
        self.child.match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self, world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        self.child.match_entity(world, archetyp_match, entity)
    }
}

impl<C> QueryParameterImmutableImpl for NoFetch<C>
    where C: QueryParameterImmutableImpl,
{
    type Value<'a> = ();

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &C::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) { }
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
    type RequiresPerEntityMatchingBool = C::RequiresPerEntityMatchingBool;
    type ValueMut<'a> = ();
    type ArchetypMatch = Result<C::ArchetypMatch, C::ArchetypMatchError>;
    type ArchetypMatchError = <C::RequiresPerEntityMatchingBool as PartialBool>::F;

    fn new(world: &World, config: C::CreationConfig) -> Self {
        Self {
            child: C::new(world, config),
        }
    }

    fn requires_per_entity_matching(&self) -> C::RequiresPerEntityMatchingBool {
        self.child.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::ArchetypMatch, Self::ArchetypMatchError> {
        match (
            self.child.requires_per_entity_matching().into_bool(),
            self.child.match_archetyp(world, archetyp_id),
        ) {
            // The archetype matched but may still refuse some entities so we
            // still need to go through the whole archetyp
            (Bool::True(t), Ok(matched)) => {
                Ok(Ok(matched))
            },
            // There is no per-entity matching so the WHOLE archetyp matched
            // which means we will be able to entierly skip it
            (Bool::False(f), Ok(matched)) => {
                Err(f)
            },
            // If it didn't match the archetyp, this always means none of the
            // entitie inside matches its filter, so for us it means we will match
            // all of them regardless
            (Bool::True(_) | Bool::False(_), Err(e)) => {
                Ok(Err(e))
            },
        }
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        match archetyp_match {
            Ok(child_match) => !self.child.match_entity(world, child_match, entity),
            Err(_) => true,
        }
    }
}

// Always immutable event with mutable parameter (the value is dropped)
impl<C> QueryParameterImmutableImpl for Not<C>
    where C: QueryParameterImpl,
{
    type Value<'a> = ();

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) { }
}

mod private {
    use super::*;

    pub trait QueryParameterTupleImpl {
        type RequiresPerEntityMatchingBool: PartialBool;
        type CreationConfig;

        type ArchetypMatchBoolValueTuple: TupleOfBoolValues;

        type AndValueMut<'a>;
        type AndArchetypMatch: Clone;
        type AndArchetypMatchError: BoolValue = <Self::ArchetypMatchBoolValueTuple as TupleOfBoolValues>::Or;

        type OrValueMut<'a>: EitherOfN;
        type OrArchetypMatch: Clone;
        type OrArchetypMatchError: BoolValue = <Self::ArchetypMatchBoolValueTuple as TupleOfBoolValues>::And;

        fn new(world: &World, cfg: Self::CreationConfig) -> Self;

        fn requires_per_entity_matching(&self) -> Self::RequiresPerEntityMatchingBool;

        fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::AndArchetypMatch, Self::AndArchetypMatchError>;
        fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::OrArchetypMatch, Self::OrArchetypMatchError>;

        fn and_match_entity(
            &self,
            world: &World,
            archetyp_match: &Self::AndArchetypMatch,
            entity: EntityIndex,
        ) -> bool;

        fn or_match_entity(
            &self,
            orld: &World,
            archetyp_match: &Self::OrArchetypMatch,
            entity: EntityIndex,
        ) -> bool;
    }

    macro_rules! bool_or_all {
        ($first: ty) => { Bool<<$first as PartialBool>::T, <$first as PartialBool>::F> };
        ($first: ty $(, $rest: ty)+) => {
            BoolOr<$first, bool_or_all!($($rest),*)>
        };
    }

    macro_rules! bool_or_else {
        ($first: expr) => { $first.into_bool() };
        ($first: expr $(, $rest: expr)+) => {
            PartialBool::or_else($first, || bool_or_else!($($rest),*))
        };
    }

    macro_rules! impl_query_parameter_tuple {
        ($($T:ident),*) => {
            impl<$($T),*> QueryParameterTupleImpl for ($($T,)*)
                where $($T: QueryParameter,)*
            {
                type RequiresPerEntityMatchingBool = bool_or_all!($($T::RequiresPerEntityMatchingBool),*);
                type CreationConfig = ($($T::CreationConfig,)*);

                type ArchetypMatchBoolValueTuple = ($($T::ArchetypMatchError,)*);

                type AndValueMut<'a> = ($($T::ValueMut::<'a>,)*);
                type AndArchetypMatch = ($($T::ArchetypMatch,)*);
                type OrValueMut<'a> = either_of!($($T::ValueMut::<'a>),*);
                type OrArchetypMatch = either_of!($($T::ArchetypMatch),*);

                fn new(world: &World, config: Self::CreationConfig) -> Self {
                    ($($T::new(world, config.${index()}),)*)
                }

                fn requires_per_entity_matching(&self) -> Self::RequiresPerEntityMatchingBool {
                    bool_or_else!($($T::requires_per_entity_matching(&self.${index()})),*)
                }

                fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::AndArchetypMatch, Self::AndArchetypMatchError> {
                    Ok(($(match $T::match_archetyp(&self.${index()}, world, archetyp_id) {
                        Ok(val) => val,
                        Err(e) => return Err(<Self::ArchetypMatchBoolValueTuple as TupleOfBoolValuesOrFrom<${index()}, $T::ArchetypMatchError>>::from(e)),
                    },)*))
                }

                fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::OrArchetypMatch, Self::OrArchetypMatchError> {
                    Err(<Self::ArchetypMatchBoolValueTuple as TupleOfBoolValues>::and_from_all((
                        $(match $T::match_archetyp(&self.${index()}, world, archetyp_id) {
                            Ok(matching) => return Ok(EitherFor::<${index()}>::either_from(matching)),
                            Err(e) => e,
                        },)*
                    )))
                }

                fn and_match_entity(
                    &self,
                    world: &World,
                    archetyp_match: &Self::AndArchetypMatch,
                    entity: EntityIndex,
                ) -> bool {
                    $($T::match_entity(&self.${index()}, world, &archetyp_match.${index()}, entity))&&*
                }

                fn or_match_entity(
                    &self,
                    world: &World,
                    archetyp_match: &Self::OrArchetypMatch,
                    entity: EntityIndex,
                ) -> bool {
                    $(if let Some(matching) = EitherFor::<${index()}>::either_for(archetyp_match) {
                        $T::match_entity(&self.${index()}, world, matching, entity)
                    } else )* {
                        unreachable!()
                    }
                }
            }
        };
    }
    variadics_please::all_tuples!(impl_query_parameter_tuple, 1, 3, T);

    pub trait QueryParameterTupleImmutableImpl: QueryParameterTupleImpl {
        type AndValue<'a>: Copy;
        type OrValue<'a>: Copy + EitherOfN;

        fn and_get<'s, 'a>(
            &'s self,
            world: &'a World,
            archetyp_match: &Self::AndArchetypMatch,
            archetyp_id: ArchetypId,
            entity: EntityIndex,
        ) -> Self::AndValue<'a>;

        fn or_get<'s, 'a>(
            &'s self,
            world: &'a World,
            archetyp_match: &Self::OrArchetypMatch,
            archetyp_id: ArchetypId,
            entity: EntityIndex,
        ) -> Self::OrValue<'a>;
    }

    macro_rules! impl_query_parameter_tuple_immutable {
        ($($T:ident),*) => {
            impl<$($T),*> QueryParameterTupleImmutableImpl for ($($T,)*)
                where $($T: QueryParameterImmutable,)*
            {
                type AndValue<'a> = ($($T::Value::<'a>,)*);
                type OrValue<'a> = either_of!($($T::Value::<'a>),*);

                fn and_get<'s, 'a>(
                    &'s self,
                    world: &'a World,
                    archetyp_match: &Self::AndArchetypMatch,
                    archetyp_id: ArchetypId,
                    entity: EntityIndex,
                ) -> Self::AndValue<'a> {
                    ($($T::get(&self.${index()}, world, &archetyp_match.${index()}, archetyp_id, entity),)*)
                }

                fn or_get<'s, 'a>(
                    &'s self,
                    world: &'a World,
                    archetyp_match: &Self::OrArchetypMatch,
                    archetyp_id: ArchetypId,
                    entity: EntityIndex,
                ) -> Self::OrValue<'a> {
                    $(if let Some(matching) = EitherFor::<${index()}>::either_for(archetyp_match) {
                        EitherFor::<${index()}>::either_from($T::get(&self.${index()}, world, matching, archetyp_id, entity))
                    } else )* {
                        unreachable!()
                    }
                }
            }
        };
    }
    variadics_please::all_tuples!(impl_query_parameter_tuple_immutable, 1, 3, T);

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
    type RequiresPerEntityMatchingBool = Tuple::RequiresPerEntityMatchingBool;
    type ValueMut<'a> = Tuple::AndValueMut<'a>;
    type ArchetypMatch = Tuple::AndArchetypMatch;
    type ArchetypMatchError = Tuple::AndArchetypMatchError;

    fn new(world: &World, config: Self::CreationConfig) -> Self {
        Self {
            tuple: Tuple::new(world, config),
        }
    }

    fn requires_per_entity_matching(&self) -> Tuple::RequiresPerEntityMatchingBool {
        self.tuple.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::ArchetypMatch, Self::ArchetypMatchError> {
        self.tuple.and_match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching().is_true());

        self.tuple.and_match_entity(world, archetyp_match, entity)
    }
}

impl<Tuple> QueryParameterImmutableImpl for And<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type Value<'a> = Tuple::AndValue<'a>;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        self.tuple.and_get(world, archetyp_match, archetyp_id, entity)
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
    type RequiresPerEntityMatchingBool = Tuple::RequiresPerEntityMatchingBool;
    type ValueMut<'a> = Tuple::OrValueMut<'a>;
    type ArchetypMatch = Tuple::OrArchetypMatch;
    type ArchetypMatchError = Tuple::OrArchetypMatchError;

    fn new(world: &World, config: Tuple::CreationConfig) -> Self {
        Self {
            tuple: Tuple::new(world, config),
        }
    }

    fn requires_per_entity_matching(&self) -> Tuple::RequiresPerEntityMatchingBool {
        self.tuple.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Result<Self::ArchetypMatch, Self::ArchetypMatchError> {
        self.tuple.or_match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching().is_true());

        self.tuple.or_match_entity(world, archetyp_match, entity)
    }
}

impl<Tuple> QueryParameterImmutableImpl for Or<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type Value<'a> = Tuple::OrValue<'a>;

    fn get<'s, 'a>(
        &'s self,
        world: &'a World,
        archetyp_match: &Self::ArchetypMatch,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Self::Value<'a> {
        self.tuple.or_get(world, archetyp_match, archetyp_id, entity)
    }
}

pub type Xor<Tuple> = And<(Or<Tuple>, Not<And<Tuple>>)>;
