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
    type ValueMut<'a> = Option<C::ValueMut<'a>>;
    type ArchetypMatch = Option<C::ArchetypMatch>;

    fn new(world: &World) -> Self {
        Self {
            child: C::new(world),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        // we never need to check per entity as we always match it (but return None)
        false
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Option<C::ArchetypMatch>> {
        Some(self.child.match_archetyp(world, archetyp_id))
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
        archetyp_match: &Option<C::ArchetypMatch>,
        archetyp_id: ArchetypId,
        entity: EntityIndex,
    ) -> Option<C::Value<'a>> {
        let child_archetyp_match = archetyp_match.as_ref()?;

        if self.child.requires_per_entity_matching() && !self.child.match_entity(world, child_archetyp_match, entity){
            return None;
        }

        Some(self.child.get(world, child_archetyp_match, archetyp_id, entity))
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
    type ValueMut<'a> = ();
    type ArchetypMatch = Option<C::ArchetypMatch>;

    fn new(world: &World) -> Self {
        Self {
            child: C::new(world),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.child.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Option<C::ArchetypMatch>> {
        // We cannot make any assumption about the whole archetyp if it requires
        // fined-grain filtering
        if self.child.requires_per_entity_matching() {
            Some(self.child.match_archetyp(world, archetyp_id))
        }
        else {
            match self.child.match_archetyp(world, archetyp_id) {
                // If the whole archetyp matches we can skip the whole thing
                Some(_) => None,
                // If it does not match we must go through it
                None => Some(None),
            }
        }
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        match archetyp_match {
            Some(child_match) => !self.child.match_entity(world, child_match, entity),
            // the child did not match the whole archetyp so the whole archtyp matches
            None => true,
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
    ) -> Self::Value<'a> {
        todo!()
    }
}

mod private {
    use super::*;

    pub trait QueryParameterTupleImpl {
        type AndValueMut<'a>;
        type AndArchetypMatch;

        type OrValueMut<'a>: EitherOfN;
        type OrArchetypMatch;

        fn new(world: &World) -> Self;

        fn requires_per_entity_matching(&self) -> bool;

        fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::AndArchetypMatch>;
        fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::OrArchetypMatch>;

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

    macro_rules! impl_query_parameter_tuple {
        ($($T:ident),*) => {
            impl<$($T),*> QueryParameterTupleImpl for ($($T,)*)
                where $($T: QueryParameter,)*
            {
                type AndValueMut<'a> = ($($T::ValueMut::<'a>,)*);
                type AndArchetypMatch = ($($T::ArchetypMatch,)*);

                type OrValueMut<'a> = either_of!($($T::ValueMut::<'a>),*);
                type OrArchetypMatch = either_of!($($T::ArchetypMatch),*);

                fn new(world: &World) -> Self {
                    ($($T::new(world),)*)
                }

                fn requires_per_entity_matching(&self) -> bool {
                    ($($T::requires_per_entity_matching(&self.${index()}))||*)
                }

                fn and_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::AndArchetypMatch> {
                    Some(($($T::match_archetyp(&self.${index()}, world, archetyp_id)?,)*))
                }

                fn or_match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::OrArchetypMatch> {
                    $(if let Some(matching) = $T::match_archetyp(&self.${index()}, world, archetyp_id) {
                        Some(EitherFor::<${index()}>::either_from(matching))
                    } else )* {
                        None
                    }
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
    variadics_please::all_tuples!(impl_query_parameter_tuple, 1, 15, T);

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
    variadics_please::all_tuples!(impl_query_parameter_tuple_immutable, 1, 15, T);

}
use private::*;

pub trait QueryParameterTuple = 'static + QueryParameterTupleImpl;
pub trait QueryParameterTupleImmutable = QueryParameterTuple + QueryParameterTupleImmutableImpl;

#[derive(Default, Debug, Clone, Copy)]
pub struct And<Tuple>
    where Tuple: QueryParameterTuple,
{
    tuple: Tuple,
}

impl<Tuple> QueryParameterImpl for And<Tuple>
    where Tuple: QueryParameterTuple,
{
    type ValueMut<'a> = Tuple::AndValueMut<'a>;
    type ArchetypMatch = Tuple::AndArchetypMatch;

    fn new(world: &World) -> Self {
        Self {
            tuple: Tuple::new(world),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.tuple.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch> {
        self.tuple.and_match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching());

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
    type ValueMut<'a> = Tuple::OrValueMut<'a>;
    type ArchetypMatch = Tuple::OrArchetypMatch;

    fn new(world: &World) -> Self {
        Self {
            tuple: Tuple::new(world),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        self.tuple.requires_per_entity_matching()
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch> {
        self.tuple.or_match_archetyp(world, archetyp_id)
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching());

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
