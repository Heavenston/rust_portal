#![feature(generic_const_exprs)]

#![expect(incomplete_features)]

pub mod handle_map;
mod uid;
pub use uid::*;

use std::hash::{ DefaultHasher, Hasher, Hash };

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
