use crate::partial_bool::{ PartialBool, BoolNot, False, True };

pub trait SomePartialOption {
    type IsSome: PartialBool;
    type And<O: SomePartialOption>: SomePartialOption;
    type Using<V>: PartialOption<V>;
}

pub trait PartialOption<T>: Sized + SomePartialOption {
    type Zip<V, O: PartialOption<V>>: PartialOption<(T, V)> = <Self::And<O> as SomePartialOption>::Using::<(T, V)>;

    fn partial_is_some(&self) -> Self::IsSome;

    fn partial_is_none(&self) -> BoolNot<Self::IsSome> {
        self.partial_is_some().not()
    }

    fn into_result(self) -> Result<T, <Self::IsSome as PartialBool>::F>;
    fn into_option(self) -> Option<T>;

    fn partial_map<V>(self, mapper: impl FnOnce(T) -> V) -> Self::Using<V>;

    fn partial_zip<V, O: PartialOption<V>>(self, other: O) -> Self::Zip<V, O>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AlwaysOption<T> {
    Some(T),
}

impl<T> SomePartialOption for AlwaysOption<T> {
    type IsSome = True;
    type And<O: SomePartialOption> = O;
    type Using<V> = AlwaysOption<V>;
}

impl<T> PartialOption<T> for AlwaysOption<T> {
    fn partial_is_some(&self) -> Self::IsSome {
        True
    }

    fn into_result(self) -> Result<T, !> {
        let Self::Some(value) = self;
        Ok(value)
    }

    fn into_option(self) -> Option<T> {
        let Self::Some(value) = self;
        Some(value)
    }

    fn partial_map<V>(self, mapper: impl FnOnce(T) -> V) -> Self::Using<V> {
        let AlwaysOption::Some(value) = self;
        AlwaysOption::Some(mapper(value))
    }

    fn partial_zip<V, O: PartialOption<V>>(self, other: O) -> Self::Zip<V, O> {
        let Self::Some(value) = self;
        other.partial_map(|o| (value, o))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NeverOption {
    None,
}

impl SomePartialOption for NeverOption {
    type IsSome = False;
    type And<O: SomePartialOption> = NeverOption;
    type Using<V> = NeverOption;
}

impl<T> PartialOption<T> for NeverOption {
    fn partial_is_some(&self) -> Self::IsSome {
        False
    }

    fn into_result(self) -> Result<T, ()> {
        Err(())
    }

    fn into_option(self) -> Option<T> {
        None
    }

    fn partial_map<V>(self, mapper: impl FnOnce(T) -> V) -> Self::Using<V> {
        let _ = mapper;
        NeverOption::None
    }

    fn partial_zip<V, O: PartialOption<V>>(self, other: O) -> Self::Zip<V, O> {
        let _ = other;
        NeverOption::None
    }
}

type FullOption<T> = Option<T>;

impl<T> SomePartialOption for FullOption<T> {
    type IsSome = bool;
    type And<O: SomePartialOption> = Option<()>;
    type Using<V> = Option<V>;
}

impl<T> PartialOption<T> for FullOption<T> {
    fn partial_is_some(&self) -> Self::IsSome {
        Option::is_some(self)
    }

    fn into_result(self) -> Result<T, ()> {
        Option::ok_or(self, ())
    }

    fn into_option(self) -> Option<T> {
        self
    }

    fn partial_map<V>(self, mapper: impl FnOnce(T) -> V) -> Self::Using<V> {
        Option::map(self, mapper)
    }

    fn partial_zip<V, O: PartialOption<V>>(self, other: O) -> Self::Zip<V, O> {
        Option::zip(self, other.into_option())
    }
}
