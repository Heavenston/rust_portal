use either::Either;

pub trait OptionExt: Sized {
    type Value;
    
    fn left_or<R>(self, right: R) -> Either<Self::Value, R> {
        self.left_or_else(|| right)
    }
    fn left_or_else<R>(self, right: impl FnOnce() -> R) -> Either<Self::Value, R>;

    fn right_or<L>(self, left: L) -> Either<L, Self::Value> {
        self.right_or_else(|| left)
    }
    fn right_or_else<L>(self, left: impl FnOnce() -> L) -> Either<L, Self::Value>;
}

impl<T> OptionExt for Option<T> {
    type Value = T;

    fn left_or_else<R>(self, right: impl FnOnce() -> R) -> Either<Self::Value, R> {
        match self {
            Some(left) => Either::Left(left),
            None => Either::Right(right()),
        }
    }

    fn right_or_else<L>(self, left: impl FnOnce() -> L) -> Either<L, Self::Value> {
        match self {
            Some(right) => Either::Right(right),
            None => Either::Left(left()),
        }
    }
}

pub fn zip_left<A, B>(option: Option<A>, val_b: B) -> Either<(A, B), B> {
    match option {
        Some(val_a) => Either::Left((val_a, val_b)),
        None => Either::Right(val_b),
    }
}

pub fn zip_right<A, B>(val_a: A, option: Option<B>) -> Either<A, (A, B)> {
    match option {
        None => Either::Left(val_a),
        Some(val_b) => Either::Right((val_a, val_b)),
    }
}
