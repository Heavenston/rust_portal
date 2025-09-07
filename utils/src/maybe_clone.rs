
trait MaybeCloneImpl {
    fn maybe_clone() -> Option<fn(&Self) -> Self>;
}

impl<T> MaybeCloneImpl for T {
    default fn maybe_clone() -> Option<fn(&Self) -> Self> {
        None
    }
}

impl<T: Clone> MaybeCloneImpl for T {
    fn maybe_clone() -> Option<fn(&Self) -> Self> {
        Some(T::clone)
    }
}

pub trait MaybeClone {
    fn maybe_clone() -> Option<fn(&Self) -> Self>;
}

impl<T: MaybeCloneImpl> MaybeClone for T {
    fn maybe_clone() -> Option<fn(&Self) -> Self> {
        <T as MaybeCloneImpl>::maybe_clone()
    }
}

