use crate::either_of::{ EitherOfN, EitherFor };

pub trait Tuple {
    const SIZE: usize;
}

pub trait TupleAtN<const N: usize> {
    type Type;
}

pub trait AcceptTupleValue<T> { }

pub trait AcceptedTuple<Accepter>: Tuple { }

pub trait TupleCreator<const N: usize, T> {
    fn create(&mut self) -> T;
}

impl<const N: usize, T> TupleCreator<N, T> for ()
    where T: Default
{
    fn create(&mut self) -> T {
        T::default()
    }
}

pub trait CreatableTuple<Creator>: Tuple {
    fn create_tuple(create: &mut Creator) -> Self;
}

pub trait TupleVisiter<const N: usize, T> {
    type Output<'a>
        where T: 'a;

    fn visit<'a>(&mut self, current: T) -> Self::Output<'a>;
}

pub trait VisitableTuple<Visiter>: Tuple {
    type Output<'a>
        where Self: 'a;

    fn visit_tuple<'a>(self, mapper: &mut Visiter) -> Self::Output<'a>;
}

pub trait TupleMapper<const N: usize, T> {
    type Output<'a>
        where Self: 'a,
              T: 'a;

    fn map<'a>(&'a self, current: T) -> Self::Output<'a>;
}

pub trait MappableTuple<Mapper>: Tuple {
    type Output<'a>
        where Self: 'a,
              Mapper: 'a;

    fn map_tuple<'a>(self, mapper: &'a Mapper) -> Self::Output<'a>;
}

pub trait TupleSelecter<const N: usize, T> {
    type Output<'a>
        where Self: 'a,
              T: 'a;

    fn select<'a>(&'a self, current: T) -> Option<Self::Output<'a>>
        where Self: 'a,
              T: 'a;
}

pub trait SelectableTuple<Selecter>: Tuple {
    type Output<'a>: EitherOfN</* SIZE = Self::SIZE */>
        where Self: 'a,
              Selecter: 'a;

    fn select_tuple<'a>(self, selecter: &'a Selecter) -> Option<Self::Output<'a>>;
}

macro_rules! tuple_impl_for {
    ($($start: ident),* ; ) => { };

    ($($start: ident),* ; $main: ident $(, $rest: ident)*) => {
        impl<$($start,)* $main $(, $rest)*> TupleAtN<{ $crate::count_args_literal!($($start),*) }> for ($($start,)* $main, $($rest,)*) {
            type Type = $main;
        }

        tuple_impl_for!($($start,)* $main ; $($rest),*);
    }
}

macro_rules! tuples_impl {
    ($($name: ident),*) => {
        impl<$($name,)*> Tuple for ($($name,)*) {
            const SIZE: usize = $crate::count_args_literal!($($name),*);
        }

        impl<Accepter, $($name,)*> AcceptedTuple<Accepter> for ($($name,)*)
            where $(Accepter: AcceptTupleValue<$name>,)*
        { }

        impl<Creator, $($name,)*> CreatableTuple<Creator> for ($($name,)*)
            where $(Creator: TupleCreator<${index()}, $name>,)*
        {
            fn create_tuple(creator: &mut Creator) -> Self {
                ($(<Creator as TupleCreator<${index()}, $name>>::create(creator),)*)
            }
        }

        impl<Visiter, $($name,)*> VisitableTuple<Visiter> for ($($name,)*)
            where $(Visiter: TupleVisiter<${index()}, $name>,)*
        {
            type Output<'a> = ($(<Visiter as TupleVisiter<${index()}, $name>>::Output<'a>,)*)
                where $($name: 'a,)*;

            fn visit_tuple<'a>(self, mapper: &mut Visiter) -> Self::Output<'a> {
                ($(<Visiter as TupleVisiter<${index()}, $name>>::visit(mapper, self.${index()}),)*)
            }
        }

        impl<'l, Visiter, $($name,)*> VisitableTuple<Visiter> for &'l ($($name,)*)
            where $(Visiter: TupleVisiter<${index()}, &'l $name>,)*
        {
            type Output<'a> = ($(<Visiter as TupleVisiter<${index()}, &'l $name>>::Output<'a>,)*)
                where 'l: 'a;

            fn visit_tuple<'a>(self, mapper: &mut Visiter) -> Self::Output<'a> {
                ($(<Visiter as TupleVisiter<${index()}, &'l $name>>::visit(mapper, &self.${index()}),)*)
            }
        }

        impl<'l, Visiter, $($name,)*> VisitableTuple<Visiter> for &'l mut ($($name,)*)
            where $(Visiter: TupleVisiter<${index()}, &'l mut $name>,)*
        {
            type Output<'a> = ($(<Visiter as TupleVisiter<${index()}, &'l mut $name>>::Output<'a>,)*)
                where 'l: 'a;

            fn visit_tuple<'a>(self, mapper: &mut Visiter) -> Self::Output<'a> {
                ($(<Visiter as TupleVisiter<${index()}, &'l mut $name>>::visit(mapper, &mut self.${index()}),)*)
            }
        }

        impl<Mapper, $($name,)*> MappableTuple<Mapper> for ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, $name>,)*
        {
            type Output<'a> = ($(<Mapper as TupleMapper<${index()}, $name>>::Output<'a>,)*)
                where $($name: 'a,)*
                      Mapper: 'a;

            fn map_tuple<'a>(self, mapper: &'a Mapper) -> Self::Output<'a> {
                ($(<Mapper as TupleMapper<${index()}, $name>>::map(mapper, self.${index()}),)*)
            }
        }

        impl<'l, Mapper, $($name,)*> MappableTuple<Mapper> for &'l ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, &'l $name>,)*
        {
            type Output<'a> = ($(<Mapper as TupleMapper<${index()}, &'l $name>>::Output<'a>,)*)
                where 'l: 'a,
                      Mapper: 'a;

            fn map_tuple<'a>(self, mapper: &'a Mapper) -> Self::Output<'a> {
                ($(<Mapper as TupleMapper<${index()}, &'l $name>>::map(mapper, &self.${index()}),)*)
            }
        }

        impl<'l, Mapper, $($name,)*> MappableTuple<Mapper> for &'l mut ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, &'l mut $name>,)*
        {
            type Output<'a> = ($(<Mapper as TupleMapper<${index()}, &'l mut $name>>::Output<'a>,)*)
                where 'l: 'a,
                      Mapper: 'a;

            fn map_tuple<'a>(self, mapper: &'a Mapper) -> Self::Output<'a> {
                ($(<Mapper as TupleMapper<${index()}, &'l mut $name>>::map(mapper, &mut self.${index()}),)*)
            }
        }

        impl<Selecter, $($name,)*> SelectableTuple<Selecter> for ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, $name>,)*
        {
            type Output<'a> = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, $name>>::Output<'a>),*)
                where $($name: 'a,)*
                      Selecter: 'a;

            fn select_tuple<'a>(self, selecter: &'a Selecter) -> Option<Self::Output<'a>> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, $name>>::select(selecter, self.${index()}) {
                        return Some(<Self::Output<'a> as EitherFor::<${index()}>>::either_from(val));
                    }
                )*

                None
            }
        }

        impl<'l, Selecter, $($name,)*> SelectableTuple<Selecter> for &'l ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, &'l $name>,)*
        {
            type Output<'a> = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, &'l $name>>::Output<'a>),*)
                where 'l: 'a,
                      Selecter: 'a;

            fn select_tuple<'a>(self, selecter: &'a Selecter) -> Option<Self::Output<'a>> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, &'l $name>>::select(selecter, &self.${index()}) {
                        return Some(<Self::Output<'a> as EitherFor::<${index()}>>::either_from(val));
                    }
                )*

                None
            }
        }

        impl<'l, Selecter, $($name,)*> SelectableTuple<Selecter> for &'l mut ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, &'l mut $name>,)*
        {
            type Output<'a> = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, &'l mut $name>>::Output<'a>),*)
                where 'l: 'a,
                      Selecter: 'a;

            fn select_tuple<'a>(self, selecter: &'a Selecter) -> Option<Self::Output<'a>> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, &'l mut $name>>::select(selecter, &mut self.${index()}) {
                        return Some(<Self::Output<'a> as EitherFor::<${index()}>>::either_from(val));
                    }
                )*

                None
            }
        }

        tuple_impl_for!( ; $($name),*);
    };
}

tuples_impl!(A);
tuples_impl!(A, B);
tuples_impl!(A, B, C);
tuples_impl!(A, B, C, D);
tuples_impl!(A, B, C, D, E);
tuples_impl!(A, B, C, D, E, F);
tuples_impl!(A, B, C, D, E, F, G);
tuples_impl!(A, B, C, D, E, F, G, H);
tuples_impl!(A, B, C, D, E, F, G, H, I);
tuples_impl!(A, B, C, D, E, F, G, H, I, J);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K, L);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
tuples_impl!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

impl<'a, T: Tuple> Tuple for &'a T {
    const SIZE: usize = T::SIZE;
}

impl<'a, T: Tuple> Tuple for &'a mut T {
    const SIZE: usize = T::SIZE;
}

/// Helper trait for requiring &T: VisitableTuple
pub trait RefVisitableTuple<Mapper>: Tuple {
    type Output<'a>
        where Self: 'a;

    fn visit_tuple<'a>(&'a self, mapper: &mut Mapper) -> Self::Output<'a>;
}

impl<T, Mapper> RefVisitableTuple<Mapper> for T
    where for<'a> &'a T: VisitableTuple<Mapper>,
          T: Tuple,
{
    type Output<'a> = <&'a T as VisitableTuple<Mapper>>::Output<'a>
        where Self: 'a;

    fn visit_tuple<'a>(&'a self, mapper: &mut Mapper) -> Self::Output<'a> {
        <&'a T as VisitableTuple<Mapper>>::visit_tuple(self, mapper)
    }
}

/// Helper trait for requiring &mut T: VisitableTuple
pub trait RefMutVisitableTuple<Mapper>: Tuple {
    type Output<'a>
        where Self: 'a;

    fn visit_tuple<'a>(&'a mut self, mapper: &mut Mapper) -> Self::Output<'a>;
}

impl<T, Mapper> RefMutVisitableTuple<Mapper> for T
    where for<'a> &'a mut T: VisitableTuple<Mapper>,
          T: Tuple,
{
    type Output<'a> = <&'a mut T as VisitableTuple<Mapper>>::Output<'a>
        where Self: 'a;

    fn visit_tuple<'a>(&'a mut self, mapper: &mut Mapper) -> Self::Output<'a> {
        <&'a mut T as VisitableTuple<Mapper>>::visit_tuple(self, mapper)
    }
}

/// Helper trait for requiring &T: MappableTuple
pub trait RefMappableTuple<Mapper>: Tuple {
    type Output<'a>
        where Self: 'a,
              Mapper: 'a;

    fn map_tuple<'a>(&'a self, mapper: &'a Mapper) -> Self::Output<'a>;
}

impl<T, Mapper> RefMappableTuple<Mapper> for T
    where for<'a> &'a T: MappableTuple<Mapper>,
          T: Tuple,
{
    type Output<'a> = <&'a T as MappableTuple<Mapper>>::Output<'a>
        where Self: 'a,
              Mapper: 'a;

    fn map_tuple<'a>(&'a self, mapper: &'a Mapper) -> Self::Output<'a> {
        <&'a T as MappableTuple<Mapper>>::map_tuple(self, mapper)
    }
}

/// Helper trait for requiring &mut T: MappableTuple
pub trait RefMutMappableTuple<Mapper>: Tuple {
    type Output<'a>
        where Self: 'a,
              Mapper: 'a;

    fn map_tuple<'a>(&'a mut self, mapper: &'a Mapper) -> Self::Output<'a>;
}

impl<T, Mapper> RefMutMappableTuple<Mapper> for T
    where for<'a> &'a mut T: MappableTuple<Mapper>,
          T: Tuple,
{
    type Output<'a> = <&'a mut T as MappableTuple<Mapper>>::Output<'a>
        where Self: 'a,
              Mapper: 'a;

    fn map_tuple<'a>(&'a mut self, mapper: &'a Mapper) -> Self::Output<'a> {
        <&'a mut T as MappableTuple<Mapper>>::map_tuple(self, mapper)
    }
}

/// Helper trait for requiring &T: SelectableTuple
pub trait RefSelectableTuple<Selecter>: Tuple {
    type Output<'a>: EitherOfN
        where Self: 'a,
              Selecter: 'a;

    fn select_tuple<'a>(&'a self, selecter: &'a Selecter) -> Option<Self::Output<'a>>;
}

impl<T, Selecter> RefSelectableTuple<Selecter> for T
    where T: Tuple,
          for<'a> &'a T: SelectableTuple<Selecter>
{
    type Output<'a> = <&'a T as SelectableTuple<Selecter>>::Output<'a>
        where Self: 'a,
              Selecter: 'a;

    fn select_tuple<'a>(&'a self, selecter: &'a Selecter) -> Option<Self::Output<'a>> {
        <&'a T as SelectableTuple<Selecter>>::select_tuple(self, selecter)
    }
}

/// Helper trait for requiring &mut T: SelectableTuple
pub trait RefMutSelectableTuple<Selecter>: Tuple {
    type Output<'a>: EitherOfN
        where Self: 'a,
              Selecter: 'a;

    fn select_tuple<'a>(&'a mut self, selecter: &'a Selecter) -> Option<Self::Output<'a>>;
}

impl<T, Selecter> RefMutSelectableTuple<Selecter> for T
    where T: Tuple,
          for<'a> &'a mut T: SelectableTuple<Selecter>
{
    type Output<'a> = <&'a mut T as SelectableTuple<Selecter>>::Output<'a>
        where Self: 'a,
              Selecter: 'a;

    fn select_tuple<'a>(&'a mut self, selecter: &'a Selecter) -> Option<Self::Output<'a>> {
        <&'a mut T as SelectableTuple<Selecter>>::select_tuple(self, selecter)
    }
}

#[cfg(test)]
mod tests {
    use crate::{ either_of::EitherOf4, tuple_trait::{SelectableTuple, VisitableTuple} };

    // This should compile
    #[test]
    fn simple_should_compile() {
        struct AcceptsInteger;

        impl super::AcceptTupleValue<i32> for AcceptsInteger {}
        impl super::AcceptTupleValue<u32> for AcceptsInteger {}

        fn integer_tuple(_tuple: impl super::AcceptedTuple<AcceptsInteger>) { }

        let tuple: (i32, u32) = (8, 32);
        integer_tuple(tuple);
    }

    #[test]
    fn simple_add_one() {
        struct AddOneMapper;

        impl<const N: usize> super::TupleMapper<N, u32> for AddOneMapper {
            type Output<'a> = i64;

            fn map(&self, current: u32) -> Self::Output<'_> {
                i64::from(current) + 1
            }
        }

        impl<const N: usize> super::TupleMapper<N, i32> for AddOneMapper {
            type Output<'a> = i64;

            fn map(&self, current: i32) -> Self::Output<'_> {
                i64::from(current) + 1
            }
        }

        let tuple: (i32, u32) = (8, 32);
        assert_eq!(super::MappableTuple::map_tuple(tuple, &mut AddOneMapper), (9i64, 33i64));
    }

    #[test]
    fn simple_increment_one() {
        struct IncrementOneMapper;

        impl<'l, const N: usize> super::TupleMapper<N, &'l mut u32> for IncrementOneMapper {
            type Output<'a> = ()
                where 'l: 'a;

            fn map(&self, current: &'l mut u32) {
                *current += 1;
            }
        }

        impl<'l, const N: usize> super::TupleMapper<N, &'l mut i32> for IncrementOneMapper {
            type Output<'a> = ()
                where 'l: 'a;

            fn map(&self, current: &'l mut i32) {
                *current += 1;
            }
        }

        let mut tuple: (i32, u32) = (8, 32);
        assert_eq!(super::MappableTuple::map_tuple(&mut tuple, &mut IncrementOneMapper), ((), ()));
        assert_eq!(tuple, (9, 33));
    }

    #[test]
    fn select_the_correct() {
        struct StringSelecter;

        impl<const N: usize> super::TupleSelecter<N, String> for StringSelecter {
            type Output<'a> = String;

            fn select<'a>(&'a self, current: String) -> Option<Self::Output<'a>>
                where Self: 'a,
                      String: 'a
            {
                (!current.is_empty()).then_some(current)
            }
        }

        impl<const N: usize> super::TupleSelecter<N, i32> for StringSelecter {
            type Output<'a> = !;

            fn select<'a>(&'a self, current: i32) -> Option<!>
                where Self: 'a,
                      i32: 'a
            {
                let _ = current;
                None
            }
        }

        let mut tuple: (i32, i32, String, String) = (8, 32, "".to_string(), "Feur".to_string());
        assert_eq!(tuple.select_tuple(&mut StringSelecter), Some(EitherOf4::D("Feur".to_string())));
        tuple = (8, 32, "".to_string(), "".to_string());
        assert_eq!(tuple.select_tuple(&mut StringSelecter), None);
    }

    #[test]
    fn create_tuple() {
        struct Creator;

        impl<const N: usize> super::TupleCreator<N, u32> for Creator {
            fn create(&mut self) -> u32 {
                42
            }
        }

        impl<const N: usize> super::TupleCreator<N, String> for Creator {
            fn create(&mut self) -> String {
                "My cool value".to_string()
            }
        }

        impl<const N: usize> super::TupleCreator<N, i32> for Creator {
            fn create(&mut self) -> i32 {
                -8
            }
        }

        let created_tuple: (String, String, String, i32, u32) = super::CreatableTuple::create_tuple(&mut Creator);

        assert_eq!(
            created_tuple,
            ("My cool value".to_string(), "My cool value".to_string(), "My cool value".to_string(), -8, 42)
        );
    }

    #[test]
    fn sum_of_tuple() {
        #[derive(Debug, PartialEq, Eq)]
        struct SumVisitor(i64);

        impl<const N: usize, T> super::TupleVisiter<N, T> for SumVisitor
            where i64: From<T>,
        {
            type Output<'a> = ()
                where T: 'a;

            fn visit<'a>(&mut self, current: T) where Self: 'a {
                self.0 += i64::from(current);
            }
        }

        let tuple: (i32, u8, i32, u32, i64) = (
            -5, 5, 32, 37, -64,
        );

        let mut visiter = SumVisitor(0);
        assert_eq!(
            tuple.visit_tuple(&mut visiter),
            ((), (), (), (), ())
        );
        assert_eq!(
            visiter,
            SumVisitor(5)
        );
    }
}
