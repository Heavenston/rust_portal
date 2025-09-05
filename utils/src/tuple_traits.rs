// macros are easier this way
#![allow(non_snake_case)]

use crate::count_args_literal;
use std::{ iter::empty, mem::MaybeUninit };

mod iter_tuple_nth;
pub use iter_tuple_nth::*;

mod iter_tuple_map_nth;
pub use iter_tuple_map_nth::*;

/// Used to circumvant rust thinking A may not be equal to B
/// Hopefully gets turned into a noop
#[allow(unsafe_code)]
fn transmute_array_i_assure_you_its_the_same_size<const A: usize, const B: usize, T>(
    a: [T; A],
) -> [T; B] {
    assert_eq!(A, B);
    let mut maybe_a = MaybeUninit::new(a);
    let ptr_a: &mut MaybeUninit<[T; A]> = &mut maybe_a;
    // they **ARE** the same size
    let ptr_b: &mut MaybeUninit<[T; B]> = unsafe { std::mem::transmute(ptr_a) };
    // Created already init
    unsafe { ptr_b.assume_init_read() }
}

/// Simple mapper for mapping any type T to another one
/// Must be able to map any type, seems ***impossible*** otherwise
pub trait TypeMapper {
    type Map<T>;

    fn map<T>(&mut self, val: T) -> Self::Map<T>;
}

pub trait Tuple {
    const SIZE: usize;
    type Map<Mapper: TypeMapper>: Tuple;
    type Append<V>: Tuple;
    type Prepend<V>: Tuple;

    fn map<M: TypeMapper>(self, mapper: M) -> Self::Map<M>;
}

pub trait HomogeneousTuple: Tuple {
    type Item;
    
    fn from_array(array: [Self::Item; Self::SIZE]) -> Self;

    fn to_array(self) -> [Self::Item; Self::SIZE];
    fn to_ref_array(&self) -> [&Self::Item; Self::SIZE];
    fn to_ref_mut_array(&mut self) -> [&mut Self::Item; Self::SIZE];

    fn iter(&self) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator + DoubleEndedIterator;
    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item> + ExactSizeIterator + DoubleEndedIterator;
    fn into_iter(self) -> impl Iterator<Item = Self::Item> + ExactSizeIterator + DoubleEndedIterator;
}

pub trait TupleAt<const N: usize>: Tuple {
    type Item;
    type Mapped<O>: TupleAt<N, Item = O>;
    type Removed: Tuple;

    fn tuple_into_at(self) -> Self::Item;
    fn tuple_get_at(&self) -> &Self::Item;
    fn tuple_get_mut_at(&mut self) -> &mut Self::Item;

    fn tuple_map_at<F, NI>(self, fun: F) -> Self::Mapped<NI>
        where F: FnOnce(Self::Item) -> NI;
}

pub trait TupleConcat<Rhs> {
    type Out;

    fn tuple_concat(self, rhs: Rhs) -> Self::Out;
}

pub trait TupleIteratorExt
    where Self: Iterator + Sized,
          Self::Item: Tuple,
{
    fn tuple_nth<const N: usize>(self) -> TupleNth<N, Self>
        where Self::Item: TupleAt<N> {
        self.into()
    }

    fn tuple_map_nth<const N: usize, F, O>(self, mapper: F) -> TupleMapNth<N, Self, F, O>
        where Self::Item: TupleAt<N>,
              F: FnMut(<Self::Item as TupleAt<N>>::Item) -> O,
    {
        TupleMapNth {
            inner: self,
            mapper,
        }
    }

    fn tuple_nth_into<const N: usize, O>(self) -> TupleMapNth<N, Self, fn(<Self::Item as TupleAt<N>>::Item) -> O, O>
        where Self::Item: TupleAt<N>,
              <Self::Item as TupleAt<N>>::Item: Into<O>,
    {
        TupleMapNth {
            inner: self,
            mapper: Into::into,
        }
    }
}

impl<I> TupleIteratorExt for I
    where I: Iterator + Sized,
          I::Item: Tuple,
{ }

macro_rules! ignore_first {
    ($f: tt $($rest: tt)*) => { $($rest)* };
}

macro_rules! prepend_unless_at_max {
    ($V: ident ! $A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident, $N:ident, $O:ident, $P:ident) =>  {
        ($V, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N, $O, $P)
    };
    ($V: ident ! $($T: ident),*) => {
        ($V, $($T,)*)
    };
}

macro_rules! append_unless_at_max {
    ($V: ident ! $A:ident, $B:ident, $C:ident, $D:ident, $E:ident, $F:ident, $G:ident, $H:ident, $I:ident, $J:ident, $K:ident, $L:ident, $M:ident, $N:ident, $O:ident, $P:ident) =>  {
        ($A, $B, $C, $D, $E, $F, $G, $H, $I, $J, $K, $L, $M, $N, $O, $V)
    };
    ($V: ident ! $($T: ident),*) => {
        ($($T,)* $V,)
    };
}

macro_rules! impl_concat {
    ($(($lhs_i: tt, $lhs_t: ident)),*) => {
        macro_rules! impl_concat_nested {
            ($$(($rhs_i: tt, $rhs_t: ident)),*) => {
                impl<$($lhs_t,)* $$($rhs_t,)*> TupleConcat<($$($rhs_t,)*)> for ($($lhs_t,)*) {
                    type Out = ($($lhs_t,)* $$($rhs_t,)*);

                    fn tuple_concat(self, rhs: ($$($rhs_t,)*)) -> Self::Out {
                        let _ = rhs;
                        ($(self.$lhs_i,)* $$(rhs.$rhs_i,)*)
                    }
                }
            };
        }

        variadics_please::all_tuples_enumerated!(impl_concat_nested, 0, 1, U);
    };
}

macro_rules! impl_at_trait {
    ($(($SI: tt, $start: ident)),* ! ) => { };

    ($(($SI:tt, $start: ident)),* ! ($I: tt, $main: ident) $(, ($RI: tt, $rest: ident))*) => {
        impl<$($start,)* $main $(,$rest)*> TupleAt<$I> for ($($start,)* $main, $($rest,)*) {
            type Item = $main;
            type Mapped<O> = ($($start,)* O, $($rest,)*);
            type Removed = ($($start,)* $($rest,)*);

            fn tuple_into_at(self) -> Self::Item {
                self.$I
            }

            fn tuple_get_at(&self) -> &Self::Item {
                &self.$I
            }

            fn tuple_get_mut_at(&mut self) -> &mut Self::Item {
                &mut self.$I
            }

            fn tuple_map_at<F, NI>(self, fun: F) -> Self::Mapped<NI>
                where F: FnOnce(Self::Item) -> NI {
                ($(self.$SI,)* fun(self.$I), $(self.$RI,)*)
            }
        }
        
        impl_at_trait!($(($SI, $start),)* ($I, $main) ! $(($RI, $rest)),*);
    };
}

macro_rules! impl_traits {
    ($(($N: tt, $T: ident)),*) => {
        impl<$($T),*> Tuple for ($($T,)*) {
            const SIZE: usize = count_args_literal!($($T),*);
            type Map<Mapper: TypeMapper> = ($(Mapper::Map<$T>,)*);
            type Append<V> = append_unless_at_max!(V ! $($T),*);
            type Prepend<V> = prepend_unless_at_max!(V ! $($T),*);

            fn map<M: TypeMapper>(self, mut mapper: M) -> Self::Map<M> {
                let _ = &mut mapper;
                ($(mapper.map(self.$N),)*)
            }
        }

        impl_traits!(if_at_least_one $(($N, $T)),*);
        impl_at_trait!(! $(($N, $T)),*);
        impl_concat!($(($N, $T)),*);
    };

    (if_at_least_one ($fN: tt, $fT: ident) $(,($N: tt, $T: ident))*) => {
        impl<T> HomogeneousTuple for (T, $(ignore_first!($T T),)*) {
            type Item = T;

            fn from_array(array: [T; Self::SIZE]) -> Self {
                let [$fT $(, $T)*]: [T; count_args_literal!($fT $(, $T)*)] = transmute_array_i_assure_you_its_the_same_size(array);
                ($fT, $($T,)*)
            }

            fn to_array(self) -> [T; Self::SIZE] {
                transmute_array_i_assure_you_its_the_same_size([$(self.$N),*])
            }

            fn to_ref_array(&self) -> [&T; Self::SIZE] {
                transmute_array_i_assure_you_its_the_same_size([$(&self.$N),*])
            }

            fn to_ref_mut_array(&mut self) -> [&mut T; Self::SIZE] {
                transmute_array_i_assure_you_its_the_same_size([$(&mut self.$N),*])
            }

            fn iter(&self) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator + DoubleEndedIterator {
                [&self.0, $(&self.$N),*].into_iter()
            }

            fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item> + ExactSizeIterator + DoubleEndedIterator {
                [&mut self.0, $(&mut self.$N),*].into_iter()
            }

            fn into_iter(self) -> impl Iterator<Item = Self::Item> + ExactSizeIterator + DoubleEndedIterator {
                [self.0, $(self.$N),*].into_iter()
            }
        }
    };

    // This means we are doing the unit type
    (if_at_least_one) => {
        impl HomogeneousTuple for () {
            type Item = !;

            fn from_array(_: [!; Self::SIZE]) -> Self { () }

            fn to_array(self) -> [!; Self::SIZE] { [] }
            fn to_ref_array(&self) -> [&!; Self::SIZE] { [] }
            fn to_ref_mut_array(&mut self) -> [&mut !; Self::SIZE] { [] }

            fn iter(&self) -> impl Iterator<Item = &Self::Item> + ExactSizeIterator + DoubleEndedIterator { empty() }
            fn iter_mut(&mut self) -> impl Iterator<Item = &mut Self::Item> + ExactSizeIterator + DoubleEndedIterator { empty() }
            fn into_iter(self) -> impl Iterator<Item = Self::Item> + ExactSizeIterator + DoubleEndedIterator { empty() }
        }
    };
}
variadics_please::all_tuples_enumerated!(impl_traits, 0, 16, T);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accurate_size() {
        assert_eq!(<() as Tuple>::SIZE, 0);
        assert_eq!(<(u32,) as Tuple>::SIZE, 1);
        assert_eq!(<((), u64,) as Tuple>::SIZE, 2);
        assert_eq!(<((), (), ()) as Tuple>::SIZE, 3);
        assert_eq!(<(u128, (), (), ()) as Tuple>::SIZE, 4);
        assert_eq!(<(u128, (), (), (), ()) as Tuple>::SIZE, 5);
        assert_eq!(<(u128, (), (), (), (), ()) as Tuple>::SIZE, 6);
    }

    #[test]
    fn tuple_at() {
        let tuple = ("feur".to_string(), 8u32, -5i32, 5.45f32);
        assert_eq!(TupleAt::<0>::tuple_into_at(tuple.clone()), "feur");
        assert_eq!(TupleAt::<1>::tuple_into_at(tuple.clone()), 8u32);
        assert_eq!(TupleAt::<2>::tuple_into_at(tuple.clone()), -5i32);
        assert_eq!(TupleAt::<3>::tuple_into_at(tuple.clone()), 5.45f32);
    }

    #[test]
    fn tuple_map_at() {
        let tuple = ("feur".to_string(), 8u32, -5i32, 5.45f32);
        assert_eq!(
            TupleAt::<2>::tuple_map_at(tuple.clone(), |val| val * 2),
            ("feur".to_string(), 8u32, -10i32, 5.45f32),
        );
        assert_eq!(
            TupleAt::<0>::tuple_map_at(tuple.clone(), |val| val + " * 2"),
            ("feur * 2".to_string(), 8u32, -5i32, 5.45f32),
        );
    }
}
