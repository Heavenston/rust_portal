use either::Either;

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
