//! Implementing here [`QueryParameter`]s that modify other parameters

use super::{
    QueryParameterImpl, QueryParameterImmutableImpl,
    QueryParameter, QueryParameterImmutable,
};

use std::marker::PhantomData;
use derive_where::derive_where;
use utils::prelude::*;

impl<T> QueryParameterImpl for Option<T>
    where T: QueryParameter,
{
    type ValueMut<'a> = Option<T::ValueMut<'a>>;
}

impl<T> QueryParameterImmutableImpl for Option<T>
    where T: QueryParameterImmutable,
{
    type Value<'a> = Option<T::Value<'a>>;
}

#[derive_where(Default, Debug, Clone, Copy)]
pub struct Not<C>
    where C: QueryParameter,
{
    _child: PhantomData<fn(C) -> C>,
}

impl<C> QueryParameterImpl for Not<C>
    where C: QueryParameter,
{
    type ValueMut<'a> = ();
}

// Always immutable event with mutable parameter (the value is dropped)
impl<C> QueryParameterImmutableImpl for Not<C>
    where C: QueryParameter,
{
    type Value<'a> = ();
}

mod private {
    use super::*;

    pub trait QueryParameterTupleImpl {
        type ValueMut<'a>;
        type EitherOfMut<'a>: EitherOfN;
    }

    pub trait QueryParameterTupleImmutableImpl: QueryParameterTupleImpl {
        type Value<'a>;
        type EitherOf<'a>: EitherOfN;
    }
}
use private::{ QueryParameterTupleImpl, QueryParameterTupleImmutableImpl };

pub trait QueryParameterTuple = QueryParameterTupleImpl;
pub trait QueryParameterTupleImmutable = QueryParameterTupleImmutableImpl;

macro_rules! parameters_impl {
    ($($name: ident),*) => {
        impl<$($name),*> QueryParameterTupleImpl for ($($name,)*)
            where $($name: QueryParameterImpl,)*
        {
            type ValueMut<'a> = ($($name::ValueMut<'a>,)*);
            type EitherOfMut<'a> = either_of!($($name::ValueMut<'a>),*);
        }

        impl<$($name),*> QueryParameterTupleImmutableImpl for ($($name,)*)
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

#[derive_where(Default, Debug, Clone, Copy)]
pub struct And<P>
    where P: QueryParameterTuple,
{
    _child: PhantomData<fn(P) -> P>,
}

impl<P> QueryParameterImpl for And<P>
    where P: QueryParameterTuple,
{
    type ValueMut<'a> = P::ValueMut<'a>;
}

impl<P> QueryParameterImmutableImpl for And<P>
    where P: QueryParameterTupleImmutable,
{
    type Value<'a> = P::Value<'a>;
}

#[derive_where(Default, Debug, Clone, Copy)]
pub struct Or<P>
    where P: QueryParameterTuple,
{
    _child: PhantomData<fn(P) -> P>,
}

impl<P> QueryParameterImpl for Or<P>
    where P: QueryParameterTuple,
{
    type ValueMut<'a> = P::EitherOfMut<'a>;
}

impl<P> QueryParameterImmutableImpl for Or<P>
    where P: QueryParameterTupleImmutable,
{
    type Value<'a> = P::EitherOf<'a>;
}
