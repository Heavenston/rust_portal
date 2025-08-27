#![feature(generic_const_exprs)]
#![feature(never_type)]
#![feature(nonzero_internals)]
#![feature(option_zip)]

#![expect(internal_features)]
#![expect(incomplete_features)]

pub mod prelude {
    pub use crate::{
        ix,
        uid::Uid,
        default, concat_arrays, flatten_array, hash_value,
        itertools::Itertools as _,
        either::{ self, Either },
        readonly, handle_map::{ self, HandleMap },
        strict_sign::{ StrictSign, StrictlySigned },
        sign::{ Sign, Signed },
        axis::{ Axis, AxisVecHelper },
        axis_direction::AxisDirection,
        mapped_nonzero::{ MappedNonZero, PlusOneNonZeroUsize },
        sorted_vec::{ SortedVec, SortedSet },
        dyn_clone,
        consume_on_drop::{ ConsumeOnDropExt as _ },
        chain_after::{ ChainAfterExt as _ },
        skip_after::{ SkipAfterExt as _ },
    };
}

pub use readonly;
pub use itertools;
pub use either;
pub use sorted_vec;
pub use dyn_clone;

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
        usize::try_from($val).expect("no overflow")
    };
}
