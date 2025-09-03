#![expect(unreachable_patterns)]

use crate::{ count_args, count_args_literal };

pub trait EitherOfN {
    const SIZE: usize;

    fn index(&self) -> usize;
}

pub trait EitherFor<const IDX: usize> {
    type N;

    fn either_from(value: Self::N) -> Self;

    fn either_for(&self) -> Option<&Self::N>;
    fn either_for_mut(&mut self) -> Option<&mut Self::N>;
    fn either_for_into(self) -> Option<Self::N>;
}

macro_rules! ignore {
    ($i: ident, $b: ident) => { $b };
}

macro_rules! impl_either_try_into {
    ($name: ident ! $($start: ident),* ; ) => {
        
    };

    ($name: ident ! $($start: ident),* ; $main: ident $(, $rest: ident)*) => {
        impl<$($start,)* $main $(, $rest)*> EitherFor<{ count_args_literal!($($start),*) }> for $name<$($start,)* $main $(, $rest)*> {
            type N = $main;

            fn either_from(value: $main) -> Self {
                Self::$main(value)
            }

            fn either_for(&self) -> Option<&$main> {
                match self {
                    Self::$main(val) => Some(val),
                    _ => None,
                }
            }

            fn either_for_mut(&mut self) -> Option<&mut $main> {
                match self {
                    Self::$main(val) => Some(val),
                    _ => None,
                }
            }

            fn either_for_into(self) -> Option<$main> {
                match self {
                    Self::$main(val) => Some(val),
                    _ => None,
                }
            }
        }

        impl_either_try_into!($name ! $($start,)* $main ; $($rest),*);
    }
}

macro_rules! impl_either {
    ($name: ident; $($letter: ident),*) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name<$($letter,)*> {
            $($letter($letter)),*
        }

        impl<S> $name<$(ignore!($letter, S)),*> {
            pub fn into_inner(self) -> S {
                match self {
                    $(
                        Self::$letter(val) => val,
                    )*
                }
            }
        }

        impl<$($letter,)*> EitherOfN for $name<$($letter,)*> {
            const SIZE: usize = count_args!($($letter,)*);

            fn index(&self) -> usize {
                match *self {
                    $(
                        Self::$letter(_) => ${index()},
                    )*
                }
            }
        }

        impl_either_try_into!($name ! ; $($letter),*);
    };

}

// FIXME
// impl_either!(EitherOf0 ; );
impl_either!(EitherOf1 ; A);
impl_either!(EitherOf2 ; A, B);
impl_either!(EitherOf3 ; A, B, C);
impl_either!(EitherOf4 ; A, B, C, D);
impl_either!(EitherOf5 ; A, B, C, D, E);
impl_either!(EitherOf6 ; A, B, C, D, E, F);
impl_either!(EitherOf7 ; A, B, C, D, E, F, G);
impl_either!(EitherOf8 ; A, B, C, D, E, F, G, H);
impl_either!(EitherOf9 ; A, B, C, D, E, F, G, H, I);
impl_either!(EitherOf10; A, B, C, D, E, F, G, H, I, J);
impl_either!(EitherOf11; A, B, C, D, E, F, G, H, I, J, K);
impl_either!(EitherOf12; A, B, C, D, E, F, G, H, I, J, K, L);
impl_either!(EitherOf13; A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_either!(EitherOf14; A, B, C, D, E, F, G, H, I, J, K, L, M, N);
impl_either!(EitherOf15; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O);
impl_either!(EitherOf16; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P);

#[macro_export]
macro_rules! either_of {
    () => { EitherOf0 };
    ($a:ty) => { $crate::either_of::EitherOf1<$a> };
    ($a:ty, $b:ty) => { $crate::either_of::EitherOf2<$a, $b> };
    ($a:ty, $b:ty, $c:ty) => { $crate::either_of::EitherOf3<$a, $b, $c> };
    ($a:ty, $b:ty, $c:ty, $d:ty) => { $crate::either_of::EitherOf4<$a, $b, $c, $d> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty) => { $crate::either_of::EitherOf5<$a, $b, $c, $d, $e> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty) => { $crate::either_of::EitherOf6<$a, $b, $c, $d, $e, $f> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty) => { $crate::either_of::EitherOf7<$a, $b, $c, $d, $e, $f, $g> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty) => { $crate::either_of::EitherOf8<$a, $b, $c, $d, $e, $f, $g, $h> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty) => { $crate::either_of::EitherOf9<$a, $b, $c, $d, $e, $f, $g, $h, $i> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty) => { $crate::either_of::EitherOf10<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty) => { $crate::either_of::EitherOf11<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty, $l:ty) => { $crate::either_of::EitherOf12<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k, $l> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty, $l:ty, $m:ty) => { $crate::either_of::EitherOf13<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k, $l, $m> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty, $l:ty, $m:ty, $n:ty) => { $crate::either_of::EitherOf14<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k, $l, $m, $n> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty, $l:ty, $m:ty, $n:ty, $o:ty) => { $crate::either_of::EitherOf15<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k, $l, $m, $n, $o> };
    ($a:ty, $b:ty, $c:ty, $d:ty, $e:ty, $f:ty, $g:ty, $h:ty, $i:ty, $j:ty, $k:ty, $l:ty, $m:ty, $n:ty, $o:ty, $p:ty) => {
        $crate::either_of::EitherOf16<$a, $b, $c, $d, $e, $f, $g, $h, $i, $j, $k, $l, $m, $n, $o, $p>
    };
    ($($args: ty),*) => { compile_error!("EitherOfN only goes up to 16") };
}

#[cfg(test)]
mod tests {
    use super::{ EitherFor, EitherOfN };

    type MyE = super::EitherOf3<u16, &'static str, [u32; 3]>;

    #[test]
    fn to_test() {
        let val = <MyE as EitherFor<{ crate::count_args!() }>>::either_from(5);
        assert_eq!(val.index(), 0);
        let val = <MyE as EitherFor<{ crate::count_args!(A) }>>::either_from("feur");
        assert_eq!(val.index(), 1);
        let val = <MyE as EitherFor<{ crate::count_args!(B, C) }>>::either_from([1, 2, 3]);
        assert_eq!(val.index(), 2);
    }
}
