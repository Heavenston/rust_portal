//! Implementing here [`QueryParameter`]s that modify other parameters

use super::{
    QueryParameterImpl, QueryParameterImmutableImpl, QueryParameter,
};
use crate::world::{ ArchetypId, EntityIndex, World };

use utils::prelude::*;
use std::ops::ControlFlow;

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

    pub trait QueryParameterVisiter {
        fn reduce(&mut self, current: &impl QueryParameter) -> ControlFlow<()>;
    }

    pub trait QueryParameterTupleImpl {
        type ValueMut<'a>;
        type EitherOfMut<'a>: EitherOfN;

        fn new(world: &World) -> Self;
        fn reduce<R: QueryParameterVisiter>(&self, reducer: &mut R);
    }

    pub trait QueryParameterTupleImmutableImpl: QueryParameterTupleImpl {
        type Value<'a>: Copy;
        type EitherOf<'a>: Copy + EitherOfN;
    }
}
use private::{ QueryParameterVisiter, QueryParameterTupleImpl, QueryParameterTupleImmutableImpl };

pub trait QueryParameterTuple = QueryParameterTupleImpl;
pub trait QueryParameterTupleImmutable = QueryParameterTupleImmutableImpl;

macro_rules! parameters_impl {
    ($($name: ident),*) => {
        impl<$($name,)*> QueryParameterTupleImpl for ($($name,)*)
            where $($name: QueryParameterImpl,)*
        {
            type ValueMut<'a> = ($($name::ValueMut<'a>,)*);
            type EitherOfMut<'a> = either_of!($($name::ValueMut<'a>),*);

            fn new(world: &World) -> Self {
                ($($name::new(world),)*)
            }

            fn reduce<R: QueryParameterVisiter>(&self, reducer: &mut R) {
                $(
                    let _f: $name;
                    match reducer.reduce(&self.${index()}) {
                        ControlFlow::Break(()) => return,
                        ControlFlow::Continue(()) => (),
                    };
                )*
            }
        }

        impl<$($name,)*> QueryParameterTupleImmutableImpl for ($($name,)*)
            where $($name: QueryParameterImmutableImpl,)*
        {
            type Value<'a> = ($($name::Value<'a>,)*);
            type EitherOf<'a> = either_of!($($name::Value<'a>),*);
        }
    };
}

parameters_impl!(A);
parameters_impl!(A, B);
parameters_impl!(A, B, C);
parameters_impl!(A, B, C, D);
parameters_impl!(A, B, C, D, E);
parameters_impl!(A, B, C, D, E, F);
parameters_impl!(A, B, C, D, E, F, G);
parameters_impl!(A, B, C, D, E, F, G, H);
parameters_impl!(A, B, C, D, E, F, G, H, I);
parameters_impl!(A, B, C, D, E, F, G, H, I, J);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K, L);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
parameters_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

#[derive(Default, Debug, Clone, Copy)]
pub struct And<Tuple>
    where Tuple: QueryParameterTuple,
{
    tuple: Tuple,
}

impl<Tuple> QueryParameterImpl for And<Tuple>
    where Tuple: QueryParameterTuple,
{
    type ValueMut<'a> = Tuple::ValueMut<'a>;

    fn new(world: &World) -> Self {
        Self {
            tuple: Tuple::new(world),
        }
    }

    // fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> bool {
    //     struct Reducer<'a> {
    //         world: &'a World,
    //         archetyp_id: ArchetypId,
    //         matches: bool,
    //     }

    //     impl<'a> QueryParameterVisiter for Reducer<'a> {
    //         fn reduce(&mut self, current: &impl QueryParameter) -> ControlFlow<()> {
    //             self.matches = self.matches && current.match_archetyp(self.world, self.archetyp_id);

    //             if self.matches {
    //                 ControlFlow::Continue(())
    //             }
    //             else {
    //                 ControlFlow::Break(())
    //             }
    //         }
    //     }

    //     let mut reducer = Reducer { world, archetyp_id, matches: true };
    //     self.tuple.reduce(&mut reducer);
    //     reducer.matches
    // }
}

impl<Tuple> QueryParameterImmutableImpl for And<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type Value<'a> = Tuple::Value<'a>;
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
    type ValueMut<'a> = Tuple::EitherOfMut<'a>;

    fn new(world: &World) -> Self {
        Self {
            tuple: Tuple::new(world),
        }
    }

    // fn match_archetyp(&self, world: &World, archetyp_id: ArchetypId) -> bool {
    //     struct Reducer<'a> {
    //         world: &'a World,
    //         archetyp_id: ArchetypId,
    //         matches: bool,
    //     }

    //     impl<'a> QueryParameterVisiter for Reducer<'a> {
    //         fn reduce(&mut self, current: &impl QueryParameter) -> ControlFlow<()> {
    //             self.matches = self.matches && current.match_archetyp(self.world, self.archetyp_id);

    //             if self.matches {
    //                 ControlFlow::Continue(())
    //             }
    //             else {
    //                 ControlFlow::Break(())
    //             }
    //         }
    //     }

    //     let mut reducer = Reducer { world, archetyp_id, matches: true };
    //     self.tuple.reduce(&mut reducer);
    //     reducer.matches
    // }
}

impl<Tuple> QueryParameterImmutableImpl for Or<Tuple>
    where Tuple: QueryParameterTupleImmutable,
{
    type Value<'a> = Tuple::EitherOf<'a>;
}
