use crate::either_of::{ EitherFor, EitherOfN };

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

pub trait TupleMapper<const N: usize, T> {
    type Output;

    fn map(&mut self, current: T) -> Self::Output;
}

pub trait MappableTuple<Mapper>: Tuple {
    type Output;

    fn map_tuple(self, mapper: &mut Mapper) -> Self::Output;
}

pub trait TupleSelecter<const N: usize, T> {
    type Output;

    fn select(&mut self, current: T) -> Option<Self::Output>;
}

pub trait SelectableTuple<Selecter>: Tuple {
    type Output: EitherOfN</* SIZE = Self::SIZE */>;

    fn select_tuple(self, selecter: &mut Selecter) -> Option<Self::Output>;
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

        impl<Mapper, $($name,)*> MappableTuple<Mapper> for ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, $name>,)*
        {
            type Output = ($(<Mapper as TupleMapper<${index()}, $name>>::Output,)*);

            fn map_tuple(self, mapper: &mut Mapper) -> Self::Output {
                ($(<Mapper as TupleMapper<${index()}, $name>>::map(mapper, self.${index()}),)*)
            }
        }

        impl<'a, Mapper, $($name,)*> MappableTuple<Mapper> for &'a ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, &'a $name>,)*
        {
            type Output = ($(<Mapper as TupleMapper<${index()}, &'a $name>>::Output,)*);

            fn map_tuple(self, mapper: &mut Mapper) -> Self::Output {
                ($(<Mapper as TupleMapper<${index()}, &'a $name>>::map(mapper, &self.${index()}),)*)
            }
        }

        impl<'a, Mapper, $($name,)*> MappableTuple<Mapper> for &'a mut ($($name,)*)
            where $(Mapper: TupleMapper<${index()}, &'a mut $name>,)*
        {
            type Output = ($(<Mapper as TupleMapper<${index()}, &'a mut $name>>::Output,)*);

            fn map_tuple(self, mapper: &mut Mapper) -> Self::Output {
                ($(<Mapper as TupleMapper<${index()}, &'a mut $name>>::map(mapper, &mut self.${index()}),)*)
            }
        }

        impl<Selecter, $($name,)*> SelectableTuple<Selecter> for ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, $name>,)*
        {
            type Output = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, $name>>::Output),*);

            fn select_tuple(self, selecter: &mut Selecter) -> Option<Self::Output> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, $name>>::select(selecter, self.${index()}) {
                        return Some(<Self::Output as EitherFor::<${index()}>>::either_from(val));
                    }
                )*

                None
            }
        }

        impl<'a, Selecter, $($name,)*> SelectableTuple<Selecter> for &'a ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, &'a $name>,)*
        {
            type Output = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, &'a $name>>::Output),*);

            fn select_tuple(self, selecter: &mut Selecter) -> Option<Self::Output> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, &'a $name>>::select(selecter, &self.${index()}) {
                        return Some(<Self::Output as EitherFor::<${index()}>>::either_from(val));
                    }
                )*

                None
            }
        }

        impl<'a, Selecter, $($name,)*> SelectableTuple<Selecter> for &'a mut ($($name,)*)
            where $(Selecter: TupleSelecter<${index()}, &'a mut $name>,)*
        {
            type Output = $crate::either_of!($(<Selecter as TupleSelecter<${index()}, &'a mut $name>>::Output),*);

            fn select_tuple(self, selecter: &mut Selecter) -> Option<Self::Output> {
                $(
                    if let Some(val) = <Selecter as TupleSelecter<${index()}, &'a mut $name>>::select(selecter, &mut self.${index()}) {
                        return Some(<Self::Output as EitherFor::<${index()}>>::either_from(val));
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

#[cfg(test)]
mod tests {
    use crate::{ either_of::EitherOf4, tuple_trait::SelectableTuple };

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
            type Output = i64;

            fn map(&mut self, current: u32) -> Self::Output {
                i64::from(current) + 1
            }
        }

        impl<const N: usize> super::TupleMapper<N, i32> for AddOneMapper {
            type Output = i64;

            fn map(&mut self, current: i32) -> Self::Output {
                i64::from(current) + 1
            }
        }

        let tuple: (i32, u32) = (8, 32);
        assert_eq!(super::MappableTuple::map_tuple(tuple, &mut AddOneMapper), (9i64, 33i64));
    }

    #[test]
    fn simple_increment_one() {
        struct IncrementOneMapper;

        impl<'a, const N: usize> super::TupleMapper<N, &'a mut u32> for IncrementOneMapper {
            type Output = ();

            fn map(&mut self, current: &'a mut u32) -> Self::Output {
                *current += 1;
            }
        }

        impl<'a, const N: usize> super::TupleMapper<N, &'a mut i32> for IncrementOneMapper {
            type Output = ();

            fn map(&mut self, current: &'a mut i32) -> Self::Output {
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
            type Output = String;

            fn select(&mut self, current: String) -> Option<Self::Output> {
                (!current.is_empty()).then_some(current)
            }
        }

        impl<const N: usize> super::TupleSelecter<N, i32> for StringSelecter {
            type Output = !;

            fn select(&mut self, _current: i32) -> Option<Self::Output> {
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
}
