#![feature(generic_const_exprs)]
#![feature(never_type)]
#![feature(nonzero_internals)]
#![feature(option_zip)]
#![feature(macro_metavar_expr)]
#![feature(debug_closure_helpers)]
#![feature(specialization)]
#![feature(assert_matches)]
#![feature(type_alias_impl_trait)]

#![expect(internal_features)]
#![expect(incomplete_features)]

#![forbid(unsafe_code)]

pub mod prelude {
    pub use crate::{
        ix, dix, trait_alias,
        uid::Uid,
        default, concat_arrays, flatten_array, hash_value,
        itertools::Itertools as _,
        either::{ self, Either },
        readonly, handle_map::{ self, HandleMap },
        strict_sign::{ StrictSign, StrictlySigned },
        sign::{ Sign, Signed },
        axis::{ Axis, AxisVecHelper },
        axis_direction::AxisDirection,
        sorted_vec::{ SortedVec, SortedSet },
        dyn_clone,
        smallvec::{ smallvec, SmallVec },
        consume_on_drop::{ ConsumeOnDropExt as _ },
        chain_after::{ ChainAfterExt as _ },
        skip_after::{ SkipAfterExt as _ },
        extract_nth::{ ExtractNthExt as _ },
        assert_is_sorted::{ AssertIsSortedExt as _ },
        either_of, either_of::*,
        bitset::{ BitSetIndex, BitSet },
        maybe_default::{ MaybeDefault },
    };
}

pub use readonly;
pub use itertools;
pub use either;
pub use sorted_vec;
pub use dyn_clone;
pub use smallvec;

pub mod handle_map;
pub mod uid;
pub mod strict_sign;
pub mod sign;
pub mod axis;
pub mod axis_direction;
pub mod mapped_nonzero;
pub mod consume_on_drop;
pub mod chain_after;
pub mod skip_after;
pub mod extract_nth;
pub mod assert_is_sorted;
pub mod either_of;
pub mod bitset;
pub mod maybe_default;

use std::hash::{ DefaultHasher, Hash, Hasher };

pub fn default<T: Default>() -> T {
    T::default()
}

pub fn concat_arrays<T, const A: usize, const B: usize>(
    a: [T; A], b: [T; B]
) -> [T; A+B]
where
    T: Default,
{
    let mut ary: [T; A+B] = std::array::from_fn(|_| Default::default());
    for (idx, val) in a.into_iter().chain(b.into_iter()).enumerate() {
        ary[idx] = val;
    }
    ary
}

pub fn flatten_array<T, const A: usize, const B: usize>(
    array: [[T; A]; B]
) -> [T; A*B]
where T: Default,
{
    let mut ary: [T; A*B] = std::array::from_fn(|_| Default::default());
    for (idx, val) in array.into_iter().flatten().enumerate() {
        ary[idx] = val;
    }
    ary
}

pub fn hash_value<T: Hash>(val: &T) -> u64 {
    let mut state = DefaultHasher::new();
    val.hash(&mut state);
    state.finish()
}

#[macro_export]
macro_rules! ix {
    ($val: expr) => {
        TryInto::<usize>::try_into($val).expect("no overflow")
    };
}

#[macro_export]
macro_rules! dix {
    ($val: expr) => {
        TryInto::<usize>::try_into($val).ok().expect("no overflow")
    };
}

#[macro_export]
macro_rules! count_args {
    () => { 0 };
    ($head:tt $(, $tail:tt)* $(,)?) => {
        1 + $crate::count_args!($($tail),*)
    };
}

#[macro_export]
macro_rules! count_args_literal {
    () => { 0 };
    ($a: tt) => { 1 };
    ($a: tt, $b: tt) => { 2 };
    ($a: tt, $b: tt, $c: tt) => { 3 };
    ($a: tt, $b: tt, $c: tt, $d: tt) => { 4 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt) => { 5 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt) => { 6 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt) => { 7 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt) => { 8 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt) => { 9 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt) => { 10 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt) => { 11 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt, $l: tt) => { 12 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt, $l: tt, $m: tt) => { 13 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt, $l: tt, $m: tt, $n: tt) => { 14 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt, $l: tt, $m: tt, $n: tt, $o: tt) => { 15 };
    ($a: tt, $b: tt, $c: tt, $d: tt, $e: tt, $f: tt, $g: tt, $h: tt, $i: tt, $j: tt, $k: tt, $l: tt, $m: tt, $n: tt, $o: tt, $p: tt) => { 16 };
    ($($arg:tt),*) => {
        compile_error! {"Too many arguments provided. Maximum is 16."}
    };
}

#[macro_export]
macro_rules! trait_alias {
    ($vis: vis trait $name: ident = $($cond: tt)+) => {
        $vis trait $name: $($cond)+ { }
        impl<T: $($cond)+> $name for T { }
    };
}
