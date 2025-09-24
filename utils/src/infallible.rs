
/// Marker trait for any type that is uninhabited like the std::convert::Infallible
/// type.
pub trait Infallible {
    fn into_never(self) -> !;
    fn as_never(&self) -> !;
}

impl Infallible for std::convert::Infallible {
    fn into_never(self) -> ! { match self { } }
    fn as_never(&self) -> ! { match *self { } }
}

impl Infallible for ! {
    fn into_never(self) -> ! { self }
    fn as_never(&self) -> ! { *self }
}

pub trait InfallibleUnwrap {
    type Output;

    fn unwrap_infallible(self) -> Self::Output;
}

impl<A, B: Infallible> InfallibleUnwrap for Result<A, B> {
    type Output = A;

    fn unwrap_infallible(self) -> Self::Output {
        let Ok(val) = self.map_err(Infallible::into_never);
        val
    }
}
