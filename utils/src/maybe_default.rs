
trait MaybeDefaultImpl {
    fn maybe_default() -> Option<fn() -> Self>;
}

impl<T> MaybeDefaultImpl for T {
    default fn maybe_default() -> Option<fn() -> Self> {
        None
    }
}

impl<T: Default> MaybeDefaultImpl for T {
    fn maybe_default() -> Option<fn() -> Self> {
        Some(T::default)
    }
}

pub trait MaybeDefault {
    fn maybe_default() -> Option<fn() -> Self>;
}

impl<T: MaybeDefaultImpl> MaybeDefault for T {
    fn maybe_default() -> Option<fn() -> Self> {
        <T as MaybeDefaultImpl>::maybe_default()
    }
}
