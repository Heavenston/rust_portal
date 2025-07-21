
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
