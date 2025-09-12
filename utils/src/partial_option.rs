use crate::partial_bool::{
    sealed as partial_bool_sealed, BoolValue, PartialBool, BoolNot, False, True, Bool
};

mod private {
    use super::*;

    pub trait PartialValueImpl {
        type Constructible: BoolValue;
    }

    impl<T> PartialValueImpl for T {
        default type Constructible = ();
    }

    impl PartialValueImpl for ! {
        type Constructible = !;
    }
}
use private::*;

pub trait PartialOption<T> {
    type IsSome: PartialBool;

    fn partial_is_some(&self) -> Self::IsSome;

    fn partial_is_none(&self) -> BoolNot<Self::IsSome> {
        self.partial_is_some().not()
    }

    fn into_result(self) -> Result<T, <Self::IsSome as PartialBool>::F>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlwaysOption<T> {
    Some(T),
}

impl<T> PartialOption<T> for AlwaysOption<T> {
    type IsSome = True;

    fn partial_is_some(&self) -> Self::IsSome {
        True
    }

    fn into_result(self) -> Result<T, !> {
        let Self::Some(value) = self;
        Ok(value)
    }
}

pub enum NeverOption {
    None,
}

impl<T> PartialOption<T> for NeverOption {
    type IsSome = False;

    fn partial_is_some(&self) -> Self::IsSome {
        False
    }

    fn into_result(self) -> Result<T, ()> {
        Err(())
    }
}

impl<T> PartialOption<T> for Option<T> {
    type IsSome = Bool<<T as PartialValueImpl>::Constructible, ()>;

    fn partial_is_some(&self) -> Self::IsSome {
        match self {
            Some(_) => Bool::True(partial_bool_sealed::PrivateBoolValue::value()),
            None => Bool::False(()),
        }
    }

    fn into_result(self) -> Result<T, ()> {
        match self {
            Some(value) => Ok(value),
            None => Err(()),
        }
    }
}

impl<T, O: BoolValue> PartialOption<T> for Result<T, O> {
    type IsSome = Bool<<T as PartialValueImpl>::Constructible, O>;

    fn partial_is_some(&self) -> Self::IsSome {
        match self {
            Ok(_) => Bool::True(partial_bool_sealed::PrivateBoolValue::value()),
            &Err(o) => Bool::False(o),
        }
    }

    fn into_result(self) -> Result<T, O> { self }
}
