//! Implementing here [`QueryParameter`]s that modify other parameters

use super::{
    QueryParameterImpl, QueryParameterImmutableImpl,
    QueryParameter, QueryParameterImmutable,
};
use crate::world::{ ArchetypId, EntityIndex, World };

use utils::prelude::*;
use std::marker::PhantomData;

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

    // fn match_archetyp(&self, world: &World, archtyp_id: ArchetypId) -> bool {
    //     !self.child.match_archetyp(world, archtyp_id)
    // }
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

    pub struct QueryParameterTupleAccepter;
    impl<T: QueryParameter> tuple_trait::AcceptTupleValue<T> for QueryParameterTupleAccepter
    { }

    pub struct QueryParameterImmutableTupleAccepter;
    impl<T: QueryParameterImmutable> tuple_trait::AcceptTupleValue<T> for QueryParameterImmutableTupleAccepter
    { }

    pub struct NewByWorld<'a> {
        pub world: &'a World,
    }
    impl<'a, const N: usize, T> tuple_trait::TupleCreator<N, T> for NewByWorld<'a>
        where T: QueryParameterImpl,
    {
        fn create(&mut self) -> T {
            T::new(self.world)
        }
    }

    pub struct AndValueMutTupleMapper<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
    impl<'a, const N: usize, T> tuple_trait::TupleMapper<N, T> for AndValueMutTupleMapper<'a>
        where T: QueryParameterImpl,
    {
        type Output = T::ValueMut<'a>;

        fn map(&mut self, current: T) -> Self::Output {
            unimplemented!()
        }
    }

    pub struct AndValueTupleMapper<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
    impl<'a, const N: usize, T> tuple_trait::TupleMapper<N, T> for AndValueTupleMapper<'a>
        where T: QueryParameterImmutableImpl,
    {
        type Output = T::Value<'a>;

        fn map(&mut self, current: T) -> Self::Output {
            unimplemented!()
        }
    }

    pub struct AndRequiresPerEntityMatchingTupleSelecter {
        pub requires: bool,
    }
    impl<'a, const N: usize, T> tuple_trait::TupleSelecter<N, &'a T> for AndRequiresPerEntityMatchingTupleSelecter
        where T: QueryParameterImmutableImpl,
    {
        type Output = ();

        fn select(&mut self, current: &'a T) -> Option<Self::Output> {
            self.requires |= current.requires_per_entity_matching();
            
            self.requires.then_some(())
        }
    }

    pub struct OrValueMutTupleSelecter<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
    impl<'a, const N: usize, T> tuple_trait::TupleSelecter<N, T> for OrValueMutTupleSelecter<'a>
        where T: QueryParameterImpl,
    {
        type Output = T::ValueMut<'a>;

        fn select(&mut self, current: T) -> Option<Self::Output> {
            unimplemented!()
        }
    }

    pub struct OrValueTupleSelecter<'a>(PhantomData<fn(&'a ()) -> &'a ()>);
    impl<'a, const N: usize, T> tuple_trait::TupleSelecter<N, T> for OrValueTupleSelecter<'a>
        where T: QueryParameterImmutableImpl,
    {
        type Output = T::Value<'a>;

        fn select(&mut self, current: T) -> Option<Self::Output> {
            unimplemented!()
        }
    }
}
use private::*;

pub trait QueryParameterTuple = 'static +
    tuple_trait::AcceptedTuple<QueryParameterTupleAccepter> +

    tuple_trait::SelectableTuple<AndRequiresPerEntityMatchingTupleSelecter> +

    for<'a> tuple_trait::MappableTuple<AndValueMutTupleMapper<'a>> +
    for<'a> tuple_trait::SelectableTuple<OrValueMutTupleSelecter<'a>> +
    for<'a> tuple_trait::CreatableTuple<NewByWorld<'a>>
;
pub trait QueryParameterTupleImmutable =
    QueryParameterTuple +
    tuple_trait::AcceptedTuple<QueryParameterImmutableTupleAccepter> +
    for<'a> tuple_trait::MappableTuple<AndValueTupleMapper<'a>> +
    for<'a> tuple_trait::SelectableTuple<OrValueTupleSelecter<'a>>
;

#[derive(Default, Debug, Clone, Copy)]
pub struct And<Tuple>
    where Tuple: QueryParameterTuple,
{
    tuple: Tuple,
}

impl<Tuple> QueryParameterImpl for And<Tuple>
    where Tuple: QueryParameterTuple,
{
    type ValueMut<'a> = <&'a Tuple as tuple_trait::MappableTuple<AndValueMutTupleMapper<'a>>>::Output;
    type ArchetypMatch = ();

    fn new(world: &World) -> Self {
        Self {
            tuple: tuple_trait::CreatableTuple::create_tuple(&mut NewByWorld { world }),
        }
    }

    fn requires_per_entity_matching(&self) -> bool {
        let mut requires = AndRequiresPerEntityMatchingTupleSelecter {
            requires: false,
        };
        tuple_trait::SelectableTuple::select_tuple(&self.tuple, &mut requires);
        requires.requires
    }

    fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch> {
        todo!()
    }

    fn match_entity(
        &self,
        world: &World,
        archetyp_match: &Self::ArchetypMatch,
        entity: EntityIndex,
    ) -> bool {
        debug_assert!(self.requires_per_entity_matching());
        todo!()
    }
}

// impl<Tuple> QueryParameterImmutableImpl for And<Tuple>
//     where Tuple: QueryParameterTupleImmutable,
// {
//     type Value<'a> = Tuple::Value<'a>;

//     fn get<'s, 'a>(
//         &'s self,
//         world: &'a World,
//         archetyp_match: &Self::ArchetypMatch,
//         archetyp_id: ArchetypId,
//         entity: EntityIndex,
//     ) -> Self::Value<'a> {
//         todo!()
//     }
// }

// #[derive(Default, Debug, Clone, Copy)]
// pub struct Or<Tuple>
//     where Tuple: QueryParameterTuple,
// {
//     tuple: Tuple,
// }

// impl<Tuple> QueryParameterImpl for Or<Tuple>
//     where Tuple: QueryParameterTuple,
// {
//     type ValueMut<'a> = Tuple::EitherOfMut<'a>;
//     type ArchetypMatch = Tuple::OrArchetypMatch;

//     fn new(world: &World) -> Self {
//         Self {
//             tuple: Tuple::new(world),
//         }
//     }

//     fn requires_per_entity_matching(&self) -> bool {
//         todo!()
//     }

//     fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> Option<Self::ArchetypMatch> {
//         todo!()
//     }

//     fn match_entity(
//         &self,
//         world: &World,
//         archetyp_match: &Self::ArchetypMatch,
//         entity: EntityIndex,
//     ) -> bool {
//         todo!()
//     }

//     // fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> bool {
//     //     struct Reducer<'a> {
//     //         world: &'a World,
//     //         archetyp_id: ArchetypId,
//     //         matches: bool,
//     //     }

//     //     impl<'a> QueryParameterVisiter for Reducer<'a> {
//     //         fn reduce(&mut self, current: &impl QueryParameter) -> ControlFlow<()> {
//     //             self.matches = self.matches && current.match_archetyp(self.world, self.archetyp_id);

//     //             if self.matches {
//     //                 ControlFlow::Continue(())
//     //             }
//     //             else {
//     //                 ControlFlow::Break(())
//     //             }
//     //         }
//     //     }

//     //     let mut reducer = Reducer { world, archetyp_id, matches: true };
//     //     self.tuple.reduce(&mut reducer);
//     //     reducer.matches
//     // }
// }

// impl<Tuple> QueryParameterImmutableImpl for Or<Tuple>
//     where Tuple: QueryParameterTupleImmutable,
// {
//     type Value<'a> = Tuple::EitherOf<'a>;

//     fn get<'s, 'a>(
//         &'s self,
//         world: &'a World,
//         archetyp_match: &Self::ArchetypMatch,
//         archetyp_id: ArchetypId,
//         entity: EntityIndex,
//     ) -> Self::Value<'a> {
//         todo!()
//     }
// }
