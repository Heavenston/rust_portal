
pub trait TryClone: Sized {
    type Error: From<!>;

    fn try_clone(&self) -> Result<Self, Self::Error>;
}

impl<T: Clone> TryClone for T {
    type Error = !;

    fn try_clone(&self) -> Result<Self, Self::Error> {
        Ok(self.clone())
    }
}

pub fn try_clone_boxed_slice<T: TryClone>(slice: &[T]) -> Result<Box<[T]>, T::Error> {
    slice.iter().map(T::try_clone).try_collect()
}
