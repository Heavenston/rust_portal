
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

#[derive(Default, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PanicAdapter<T>(pub T);

impl<T: TryClone> Clone for PanicAdapter<T>
    where T::Error: std::fmt::Debug,
{
    fn clone(&self) -> Self {
        Self(self.0.try_clone().unwrap())
    }
}

impl<T> std::ops::Deref for PanicAdapter<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for PanicAdapter<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}
