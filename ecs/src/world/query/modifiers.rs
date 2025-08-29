//! Implementing here [`QueryParameter`]s that modify other parameters

use super::{ QueryParameter, QueryParameterImmutable };

use std::marker::PhantomData;
use derive_where::derive_where;
use utils::prelude::*;

impl<T> QueryParameter for Option<T>
    where T: QueryParameter,
{
    type ValueMut<'a> = Option<T::ValueMut<'a>>;
}

impl<T> QueryParameterImmutable for Option<T>
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

impl<C> QueryParameter for Not<C>
    where C: QueryParameter,
{
    type ValueMut<'a> = ();
}

// Always immutable event with mutable parameter (the value is dropped)
impl<C> QueryParameterImmutable for Not<C>
    where C: QueryParameter,
{
    type Value<'a> = ();
}

pub trait QueryParameterTuple {
    type ValueMut<'a>;
    type EitherOfMut<'a>: EitherOfN;
}

pub trait QueryParameterTupleImmutable: QueryParameterTuple {
    type Value<'a>;
    type EitherOf<'a>: EitherOfN;
}

macro_rules! parameters_impl {
    ($($name: ident),*) => {
        impl<$($name),*> QueryParameterTuple for ($($name,)*)
            where $($name: QueryParameter,)*
        {
            type ValueMut<'a> = ($($name::ValueMut<'a>,)*);
            type EitherOfMut<'a> = either_of!($($name::ValueMut<'a>),*);
        }

        impl<$($name),*> QueryParameterTupleImmutable for ($($name,)*)
            where $($name: QueryParameterImmutable,)*
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

impl<P> QueryParameter for And<P>
    where P: QueryParameterTuple,
{
    type ValueMut<'a> = P::ValueMut<'a>;
}

impl<P> QueryParameterImmutable for And<P>
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

impl<P> QueryParameter for Or<P>
    where P: QueryParameterTuple,
{
    type ValueMut<'a> = P::EitherOfMut<'a>;
}

impl<P> QueryParameterImmutable for Or<P>
    where P: QueryParameterTupleImmutable,
{
    type Value<'a> = P::EitherOf<'a>;
}
