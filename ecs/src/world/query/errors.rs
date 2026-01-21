use std::marker::PhantomData;

use crate::world::{
    ComponentDoesNotHaveStorageError,
    ComponentIsNotAliveError, ComponentNotPresentError,
    EntityIsNotAliveError, ForbiddenError,
    Entity,
};
use utils::prelude::*;
use concat_idents::concat_idents;

#[derive(Debug, thiserror::Error)]
#[error("{entity} does not match this query")]
pub struct EntityDoesNotMatchQueryError {
    pub entity: Entity,
}

mod sealed {
    use super::*;

    pub trait AddTo<T: AnyQueryError>: Sized {
        type Added: AnyQueryError + From<Self> + From<T>;
    }

    pub trait ValidQueryError: Into<AllQueryError>
        + AddTo<QueryError000000> + AddTo<QueryError000001> + AddTo<QueryError000010> + AddTo<QueryError000011> + AddTo<QueryError000100> + AddTo<QueryError000101> + AddTo<QueryError000110> + AddTo<QueryError000111> + AddTo<QueryError001000> + AddTo<QueryError001001> + AddTo<QueryError001010> + AddTo<QueryError001011> + AddTo<QueryError001100> + AddTo<QueryError001101> + AddTo<QueryError001110> + AddTo<QueryError001111> + AddTo<QueryError010000> + AddTo<QueryError010001> + AddTo<QueryError010010> + AddTo<QueryError010011> + AddTo<QueryError010100> + AddTo<QueryError010101> + AddTo<QueryError010110> + AddTo<QueryError010111> + AddTo<QueryError011000> + AddTo<QueryError011001> + AddTo<QueryError011010> + AddTo<QueryError011011> + AddTo<QueryError011100> + AddTo<QueryError011101> + AddTo<QueryError011110> + AddTo<QueryError011111> + AddTo<QueryError100000> + AddTo<QueryError100001> + AddTo<QueryError100010> + AddTo<QueryError100011> + AddTo<QueryError100100> + AddTo<QueryError100101> + AddTo<QueryError100110> + AddTo<QueryError100111> + AddTo<QueryError101000> + AddTo<QueryError101001> + AddTo<QueryError101010> + AddTo<QueryError101011> + AddTo<QueryError101100> + AddTo<QueryError101101> + AddTo<QueryError101110> + AddTo<QueryError101111> + AddTo<QueryError110000> + AddTo<QueryError110001> + AddTo<QueryError110010> + AddTo<QueryError110011> + AddTo<QueryError110100> + AddTo<QueryError110101> + AddTo<QueryError110110> + AddTo<QueryError110111> + AddTo<QueryError111000> + AddTo<QueryError111001> + AddTo<QueryError111010> + AddTo<QueryError111011> + AddTo<QueryError111100> + AddTo<QueryError111101> + AddTo<QueryError111110> + AddTo<QueryError111111>
    { }

    impl ValidQueryError for ComponentDoesNotHaveStorageError { }
    impl ValidQueryError for ComponentIsNotAliveError { }
    impl ValidQueryError for ComponentNotPresentError { }
    impl ValidQueryError for EntityIsNotAliveError { }
    impl ValidQueryError for ForbiddenError { }
    impl ValidQueryError for EntityDoesNotMatchQueryError { }

    pub trait AndWith<T: AnyQueryError>: Sized {
        type Anded: AnyQueryError + From<Self> + From<T>;
    }

    pub trait AnyQueryError: Sized + Into<AllQueryError>
        + AndWith<QueryError000000> + AndWith<QueryError000001> + AndWith<QueryError000010> + AndWith<QueryError000011> + AndWith<QueryError000100> + AndWith<QueryError000101> + AndWith<QueryError000110> + AndWith<QueryError000111> + AndWith<QueryError001000> + AndWith<QueryError001001> + AndWith<QueryError001010> + AndWith<QueryError001011> + AndWith<QueryError001100> + AndWith<QueryError001101> + AndWith<QueryError001110> + AndWith<QueryError001111> + AndWith<QueryError010000> + AndWith<QueryError010001> + AndWith<QueryError010010> + AndWith<QueryError010011> + AndWith<QueryError010100> + AndWith<QueryError010101> + AndWith<QueryError010110> + AndWith<QueryError010111> + AndWith<QueryError011000> + AndWith<QueryError011001> + AndWith<QueryError011010> + AndWith<QueryError011011> + AndWith<QueryError011100> + AndWith<QueryError011101> + AndWith<QueryError011110> + AndWith<QueryError011111> + AndWith<QueryError100000> + AndWith<QueryError100001> + AndWith<QueryError100010> + AndWith<QueryError100011> + AndWith<QueryError100100> + AndWith<QueryError100101> + AndWith<QueryError100110> + AndWith<QueryError100111> + AndWith<QueryError101000> + AndWith<QueryError101001> + AndWith<QueryError101010> + AndWith<QueryError101011> + AndWith<QueryError101100> + AndWith<QueryError101101> + AndWith<QueryError101110> + AndWith<QueryError101111> + AndWith<QueryError110000> + AndWith<QueryError110001> + AndWith<QueryError110010> + AndWith<QueryError110011> + AndWith<QueryError110100> + AndWith<QueryError110101> + AndWith<QueryError110110> + AndWith<QueryError110111> + AndWith<QueryError111000> + AndWith<QueryError111001> + AndWith<QueryError111010> + AndWith<QueryError111011> + AndWith<QueryError111100> + AndWith<QueryError111101> + AndWith<QueryError111110> + AndWith<QueryError111111>
    {
        type With<P: ValidQueryError>: AnyQueryError + From<P> + From<Self>;
        type And<O: AnyQueryError>: AnyQueryError + From<O> + From<Self>;
    }

    pub struct QueryErrorFromTuple<T>(!, PhantomData<fn(T) -> T>);
    pub trait QueryErrorFromTupleError {
        type Error: AnyQueryError;
    }

    macro_rules! impl_query_error_struct {
        (@with $first: ident $(, $T: ident)*) => {
            <impl_query_error_struct!(@with $($T),*) as AnyQueryError>::With<$first>
        };

        (@with) => { EmptyQueryError };

        ($($T: ident),*) => {
            impl<$($T,)*> QueryErrorFromTupleError for QueryErrorFromTuple<($($T,)*)>
                where $($T: ValidQueryError,)*
            {
                type Error = impl_query_error_struct!(@with $($T),*);
            }
        };
    }
    variadics_please::all_tuples!(impl_query_error_struct, 0, 8, T);
}
use sealed::{ ValidQueryError, AddTo, AndWith, QueryErrorFromTuple, QueryErrorFromTupleError };
pub(super) use sealed::AnyQueryError;

macro_rules! for_every {
    ($macro: ident $delem:tt $(, $($args: tt)+)?) => {
        $macro!(0,0,0,0,0,0$(, $($args)+)?)$delem $macro!(0,0,0,0,0,1$(, $($args)+)?)$delem $macro!(0,0,0,0,1,0$(, $($args)+)?)$delem $macro!(0,0,0,0,1,1$(, $($args)+)?)$delem $macro!(0,0,0,1,0,0$(, $($args)+)?)$delem $macro!(0,0,0,1,0,1$(, $($args)+)?)$delem $macro!(0,0,0,1,1,0$(, $($args)+)?)$delem $macro!(0,0,0,1,1,1$(, $($args)+)?)$delem $macro!(0,0,1,0,0,0$(, $($args)+)?)$delem $macro!(0,0,1,0,0,1$(, $($args)+)?)$delem $macro!(0,0,1,0,1,0$(, $($args)+)?)$delem $macro!(0,0,1,0,1,1$(, $($args)+)?)$delem $macro!(0,0,1,1,0,0$(, $($args)+)?)$delem $macro!(0,0,1,1,0,1$(, $($args)+)?)$delem $macro!(0,0,1,1,1,0$(, $($args)+)?)$delem $macro!(0,0,1,1,1,1$(, $($args)+)?)$delem $macro!(0,1,0,0,0,0$(, $($args)+)?)$delem $macro!(0,1,0,0,0,1$(, $($args)+)?)$delem $macro!(0,1,0,0,1,0$(, $($args)+)?)$delem $macro!(0,1,0,0,1,1$(, $($args)+)?)$delem $macro!(0,1,0,1,0,0$(, $($args)+)?)$delem $macro!(0,1,0,1,0,1$(, $($args)+)?)$delem $macro!(0,1,0,1,1,0$(, $($args)+)?)$delem $macro!(0,1,0,1,1,1$(, $($args)+)?)$delem $macro!(0,1,1,0,0,0$(, $($args)+)?)$delem $macro!(0,1,1,0,0,1$(, $($args)+)?)$delem $macro!(0,1,1,0,1,0$(, $($args)+)?)$delem $macro!(0,1,1,0,1,1$(, $($args)+)?)$delem $macro!(0,1,1,1,0,0$(, $($args)+)?)$delem $macro!(0,1,1,1,0,1$(, $($args)+)?)$delem $macro!(0,1,1,1,1,0$(, $($args)+)?)$delem $macro!(0,1,1,1,1,1$(, $($args)+)?)$delem $macro!(1,0,0,0,0,0$(, $($args)+)?)$delem $macro!(1,0,0,0,0,1$(, $($args)+)?)$delem $macro!(1,0,0,0,1,0$(, $($args)+)?)$delem $macro!(1,0,0,0,1,1$(, $($args)+)?)$delem $macro!(1,0,0,1,0,0$(, $($args)+)?)$delem $macro!(1,0,0,1,0,1$(, $($args)+)?)$delem $macro!(1,0,0,1,1,0$(, $($args)+)?)$delem $macro!(1,0,0,1,1,1$(, $($args)+)?)$delem $macro!(1,0,1,0,0,0$(, $($args)+)?)$delem $macro!(1,0,1,0,0,1$(, $($args)+)?)$delem $macro!(1,0,1,0,1,0$(, $($args)+)?)$delem $macro!(1,0,1,0,1,1$(, $($args)+)?)$delem $macro!(1,0,1,1,0,0$(, $($args)+)?)$delem $macro!(1,0,1,1,0,1$(, $($args)+)?)$delem $macro!(1,0,1,1,1,0$(, $($args)+)?)$delem $macro!(1,0,1,1,1,1$(, $($args)+)?)$delem $macro!(1,1,0,0,0,0$(, $($args)+)?)$delem $macro!(1,1,0,0,0,1$(, $($args)+)?)$delem $macro!(1,1,0,0,1,0$(, $($args)+)?)$delem $macro!(1,1,0,0,1,1$(, $($args)+)?)$delem $macro!(1,1,0,1,0,0$(, $($args)+)?)$delem $macro!(1,1,0,1,0,1$(, $($args)+)?)$delem $macro!(1,1,0,1,1,0$(, $($args)+)?)$delem $macro!(1,1,0,1,1,1$(, $($args)+)?)$delem $macro!(1,1,1,0,0,0$(, $($args)+)?)$delem $macro!(1,1,1,0,0,1$(, $($args)+)?)$delem $macro!(1,1,1,0,1,0$(, $($args)+)?)$delem $macro!(1,1,1,0,1,1$(, $($args)+)?)$delem $macro!(1,1,1,1,0,0$(, $($args)+)?)$delem $macro!(1,1,1,1,0,1$(, $($args)+)?)$delem $macro!(1,1,1,1,1,0$(, $($args)+)?)$delem $macro!(1,1,1,1,1,1$(, $($args)+)?)$delem
    };
}

macro_rules! create_err {
    ($a1: tt, $a2: tt, $a3: tt, $a4: tt, $a5: tt, $a6: tt) => {
        optional_enum_fields!(concat_idents!(err_name = QueryError, $a1, $a2, $a3, $a4, $a5, $a6 {
            #[derive(Debug, thiserror::Error)]
            #[error(transparent)]
            #[doc(hidden)]
            pub enum err_name {
                __optional($a1 ComponentDoesNotHaveStorage(#[from] ComponentDoesNotHaveStorageError),)
                __optional($a2 ComponentIsNotAlive(#[from] ComponentIsNotAliveError),)
                __optional($a3 ComponentNotPresent(#[from] ComponentNotPresentError),)
                __optional($a4 EntityIsNotAlive(#[from] EntityIsNotAliveError),)
                __optional($a5 Forbidden(#[from] ForbiddenError),)
                __optional($a6 EntityDoesNotMatchQuery(#[from] EntityDoesNotMatchQueryError),)
            }

            impl AnyQueryError for err_name {
                type With<T: ValidQueryError> = <T as AddTo<err_name>>::Added;
                type And<T: AnyQueryError> = <T as AndWith<err_name>>::Anded;
            }

            impl AddTo<err_name> for ComponentDoesNotHaveStorageError {
                concat_idents!(err2_name = QueryError, 1, $a2, $a3, $a4, $a5, $a6 {
                    type Added = err2_name;
                });
            }

            impl AddTo<err_name> for ComponentIsNotAliveError {
                concat_idents!(err2_name = QueryError, $a1, 1, $a3, $a4, $a5, $a6 {
                    type Added = err2_name;
                });
            }

            impl AddTo<err_name> for ComponentNotPresentError {
                concat_idents!(err2_name = QueryError, $a1, $a2, 1, $a4, $a5, $a6 {
                    type Added = err2_name;
                });
            }

            impl AddTo<err_name> for EntityIsNotAliveError {
                concat_idents!(err2_name = QueryError, $a1, $a2, $a3, 1, $a5, $a6 {
                    type Added = err2_name;
                });
            }

            impl AddTo<err_name> for ForbiddenError {
                concat_idents!(err2_name = QueryError, $a1, $a2, $a3, $a4, 1, $a6 {
                    type Added = err2_name;
                });
            }

            impl AddTo<err_name> for EntityDoesNotMatchQueryError {
                concat_idents!(err2_name = QueryError, $a1, $a2, $a3, $a4, $a5, 1 {
                    type Added = err2_name;
                });
            }

            // Dont know how to do it without specilization here
            // impl<T: AnyQueryError> QueryErrorAnd<T> for err_name {
            //     type And = AllQueryError;
            // }
        }););

        macro_rules! create_err_again {
            ($b1: tt, $b2: tt, $b3: tt, $b4: tt, $b5: tt, $b6: tt) => {optional_enum_fields!(
                __optional(
                   // Not all equal (cant impl From<T> for T)
                   !($a1 = $b1  & $a2 = $b2  & $a3 = $b3  & $a4 = $b4  & $a5 = $b5  & $a6 = $b6) &
                   // And the other do not have a variant this one does not
                   (!$b1 | $a1) & (!$b2 | $a2) & (!$b3 | $a3) & (!$b4 | $a4) & (!$b5 | $a5) & (!$b6 | $a6)

                impl From<concat_idents!(err_name2 = QueryError, $b1, $b2, $b3, $b4, $b5, $b6 { err_name2 })> for concat_idents!(err_name = QueryError, $a1, $a2, $a3, $a4, $a5, $a6 { err_name }) {
                    fn from(other: concat_idents!(err_name2 = QueryError, $b1, $b2, $b3, $b4, $b5, $b6 { err_name2 })) -> Self {
                        concat_idents!(err_name2 = QueryError, $b1, $b2, $b3, $b4, $b5, $b6 {
                            type Other = err_name2;
                        });

                        match other {
                            __optional($b1 Other::ComponentDoesNotHaveStorage(e) => e.into(),)
                            __optional($b2 Other::ComponentIsNotAlive(e) => e.into(),)
                            __optional($b3 Other::ComponentNotPresent(e) => e.into(),)
                            __optional($b4 Other::EntityIsNotAlive(e) => e.into(),)
                            __optional($b5 Other::Forbidden(e) => e.into(),)
                            __optional($b6 Other::EntityDoesNotMatchQuery(e) => e.into(),)
                        }
                    }
                })
            );}
        }
        for_every!(create_err_again;);

        macro_rules! impl_and {
            ($b1: tt, $b2: tt, $b3: tt, $b4: tt, $b5: tt, $b6: tt) => {optional_enum_fields!(
                impl AndWith<concat_idents!(err_name2 = QueryError, $b1, $b2, $b3, $b4, $b5, $b6 { err_name2 })> for concat_idents!(err_name = QueryError, $a1, $a2, $a3, $a4, $a5, $a6 { err_name }) { 
                    type Anded = concat_idents!(err_name = QueryError, __eval($a1 | $b1), __eval($a2 | $b2), __eval($a3 | $b3), __eval($a4 | $b4), __eval($a5 | $b5), __eval($a6 | $b6) { err_name });
                }
            );}
        }
        for_every!(impl_and;);
    };
}
for_every!(create_err;);

#[doc(inline)]
pub use QueryError000000 as EmptyQueryError;
#[doc(inline)]
pub use QueryError111111 as AllQueryError;
pub type QueryError<T> = <QueryErrorFromTuple<T> as QueryErrorFromTupleError>::Error;
pub(super) type QueryGetError = QueryError<(EntityIsNotAliveError, EntityDoesNotMatchQueryError)>;

impl Infallible for EmptyQueryError {
    fn into_never(self) -> ! { match self { } }
    fn as_never(&self) -> ! { match *self { } }
}
