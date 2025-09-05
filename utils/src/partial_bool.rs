
mod sealed {
    use super::BoolValue;

    pub trait PrivateBoolValue {
        fn value() -> Self;
    }
    pub trait PrivatePartialBool { }

    pub trait BoolValueAnd<O: BoolValue> {
        type Out: BoolValue;
    }
    impl<A: BoolValue, B: BoolValue> BoolValueAnd<A> for B { default type Out = (); }

    pub trait BoolValueOr<O: BoolValue> {
        type Out: BoolValue;
    }
    impl<A: BoolValue, B: BoolValue> BoolValueOr<A> for B { default type Out = (); }
}

pub trait BoolValue: std::fmt::Debug + Copy + sealed::PrivateBoolValue {
    const INHABITED: bool;
    /// Type is INHABITED if both Self and O are INHABITED
    type And<O: BoolValue>: BoolValue;
    /// Type is INHABITED if one of Self or O are INHABITED
    type Or<O: BoolValue>: BoolValue;
    type Not: BoolValue;
}

impl sealed::PrivateBoolValue for () {
    fn value() -> Self { () }
}
impl BoolValue for () {
    const INHABITED: bool = true;
    type And<O: BoolValue> = <Self as sealed::BoolValueAnd<O>>::Out;
    type Or<O: BoolValue> = <Self as sealed::BoolValueOr<O>>::Out;
    type Not = !;
}

impl sealed::BoolValueAnd<()> for () { type Out = (); }
impl sealed::BoolValueAnd<!> for () { type Out = !; }
impl sealed::BoolValueOr<()> for () { type Out = (); }
impl sealed::BoolValueOr<!> for () { type Out = (); }

impl sealed::PrivateBoolValue for ! {
    fn value() -> Self { unreachable!() }
}
impl BoolValue for ! {
    const INHABITED: bool = false;
    type And<O: BoolValue> = <Self as sealed::BoolValueAnd<O>>::Out;
    type Or<O: BoolValue> = <Self as sealed::BoolValueOr<O>>::Out;
    type Not = ();
}

impl sealed::BoolValueAnd<()> for ! { type Out = !; }
impl sealed::BoolValueAnd<!> for ! { type Out = !; }
impl sealed::BoolValueOr<()> for ! { type Out = (); }
impl sealed::BoolValueOr<!> for ! { type Out = !; }

#[allow(type_alias_bounds)]
pub type BoolAnd<A: PartialBool, B: PartialBool> = Bool<<A::T as BoolValue>::And<B::T>, <A::F as BoolValue>::Or<B::F>>;
#[allow(type_alias_bounds)]
pub type BoolOr<A: PartialBool, B: PartialBool> = Bool<<A::T as BoolValue>::Or<B::T>, <A::F as BoolValue>::And<B::F>>;
#[allow(type_alias_bounds)]
pub type BoolNot<A: PartialBool> = Bool<A::F, A::T>;

pub trait PartialBool: Clone + Copy + sealed::PrivatePartialBool + Into<Bool<Self::T, Self::F>> + std::ops::Not {
    type T: BoolValue;
    type F: BoolValue;

    const CAN_BE_TRUE: bool = Self::T::INHABITED;
    const CAN_BE_FALSE: bool = Self::F::INHABITED;

    fn is_true(self) -> bool;
    fn is_false(self) -> bool { !self.is_true() }

    fn into_bool(self) -> Bool<Self::T, Self::F> {
        self.into()
    }

    fn and<O: PartialBool>(self, other: O) -> BoolAnd<Self, O> {
        self.and_then(|| other)
    }

    fn and_then<O: PartialBool>(self, other: impl FnOnce() -> O) -> BoolAnd<Self, O> {
        match self.into() {
            Bool::False(_) => Bool::value_false(),
            Bool::True(_) => match other().into() {
                Bool::False(_) => Bool::value_false(),
                Bool::True(_) => Bool::value_true(),
            },
        }
    }

    fn or<O: PartialBool>(self, other: O) -> BoolOr<Self, O> {
        self.or_else(|| other)
    }

    fn or_else<O: PartialBool>(self, other: impl FnOnce() -> O) -> BoolOr<Self, O> {
        match self.into() {
            Bool::True(_) => Bool::value_true(),
            Bool::False(_) => match other().into() {
                Bool::True(_) => Bool::value_true(),
                Bool::False(_) => Bool::value_false(),
            },
        }
    }

    fn not(self) -> BoolNot<Self> {
        !self.into()
    }
}

impl sealed::PrivatePartialBool for bool { }
impl PartialBool for bool {
    type T = ();
    type F = ();

    fn is_true(self) -> bool { self }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bool<T = (), F = ()>
    where T: BoolValue, F: BoolValue,
{
    True(T),
    False(F),
}

impl<T, F> Bool<T, F>
    where T: BoolValue,
          F: BoolValue,
{
    // Make sure you know this is unreachable if T is not inhabited
    fn value_true() -> Self {
        Self::True(T::value())
    }
    
    // Make sure you know this is unreachable if F is not inhabited
    fn value_false() -> Self {
        Self::False(F::value())
    }
}

impl Bool {
    pub const TRUE: Bool = Bool::True(());
    pub const FALSE: Bool = Bool::False(());
}

impl<T> Default for Bool<T, ()>
    where T: BoolValue,
{
    fn default() -> Self {
        Bool::False(())
    }
}

impl From<Bool<(), !>> for Bool {
    fn from(Bool::True(()): Bool<(), !>) -> Bool {
        Bool::True(())
    }
}

impl From<Bool<!, ()>> for Bool {
    fn from(Bool::False(()): Bool<!, ()>) -> Bool {
        Bool::False(())
   }
}

impl From<Bool<!, !>> for Bool {
    fn from(_: Bool<!, !>) -> Bool {
        unreachable!()
   }
}

impl From<bool> for Bool {
    fn from(value: bool) -> Self {
        match value {
            true => Bool::True(()),
            false => Bool::False(()),
        }
    }
}

impl<A, B> Into<bool> for Bool<A, B>
    where A: BoolValue, B: BoolValue,
{
    fn into(self) -> bool {
        match self {
            Bool::False(_) => false,
            Bool::True(_) => true,
        }
    }
}

impl<T, F> sealed::PrivatePartialBool for Bool<T, F>
    where T: BoolValue, F: BoolValue,
{ }
impl<T, F> PartialBool for Bool<T, F>
    where T: BoolValue, F: BoolValue,
{
    type T = T;
    type F = F;

    fn is_true(self) -> bool {
        matches!(self, Self::True(_))
    }
}

impl<Ta, Fa, Tb, Fb> std::ops::BitAnd<Bool<Tb, Fb>> for Bool<Ta, Fa>
    where Ta: BoolValue, Fa: BoolValue,
          Tb: BoolValue, Fb: BoolValue,
{
    type Output = BoolAnd<Bool<Ta, Fa>, Bool<Tb, Fb>>;

    fn bitand(self, rhs: Bool<Tb, Fb>) -> Self::Output {
        match (self, rhs) {
            (Bool::True(_), Bool::True(_)) => Bool::value_true(),
            _ => Bool::value_false(),
        }
    }
}

impl<Ta, Fa, Tb, Fb> std::ops::BitOr<Bool<Tb, Fb>> for Bool<Ta, Fa>
    where Ta: BoolValue, Fa: BoolValue,
          Tb: BoolValue, Fb: BoolValue,
{
    type Output = BoolOr<Bool<Ta, Fa>, Bool<Tb, Fb>>;

    fn bitor(self, rhs: Bool<Tb, Fb>) -> Self::Output {
        match (self, rhs) {
            (Bool::False(_), Bool::False(_)) => Bool::value_false(),
            _ => Bool::value_true(),
        }
    }
}

impl<T, F> std::ops::Not for Bool<T, F>
    where T: BoolValue, F: BoolValue,
{
    type Output = BoolNot<Self>;

    fn not(self) -> Self::Output {
        match self {
            Bool::True(t) => Bool::False(t),
            Bool::False(f) => Bool::True(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct True;

impl From<True> for bool {
    fn from(_: True) -> Self { true }
}

impl<F> From<True> for Bool<(), F>
    where F: BoolValue,
{
    fn from(_: True) -> Self {
        Bool::True(())
    }
}

impl From<Bool<(), !>> for True {
    fn from(Bool::True(()): Bool<(), !>) -> Self {
        True
    }
}

impl sealed::PrivatePartialBool for True { }
impl PartialBool for True {
    type T = ();
    type F = !;

    fn is_true(self) -> bool { true }
}

impl std::ops::Not for True {
    type Output = False;

    fn not(self) -> Self::Output {
        False
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct False;

impl From<False> for bool {
    fn from(_: False) -> Self { false }
}

impl<T> From<False> for Bool<T, ()>
    where T: BoolValue,
{
    fn from(_: False) -> Self {
        Bool::False(())
    }
}

impl From<Bool<!, ()>> for False {
    fn from(Bool::False(()): Bool<!, ()>) -> Self {
        False
    }
}

impl sealed::PrivatePartialBool for False { }
impl PartialBool for False {
    type T = !;
    type F = ();

    fn is_true(self) -> bool { false }
}

impl std::ops::Not for False {
    type Output = True;

    fn not(self) -> Self::Output {
        True
    }
}

#[cfg(test)]
mod tests {
    use super::{ PartialBool, Bool, True, False };

    #[test]
    fn correct_can_be_values() {
        assert!( Bool::<(), ()>::CAN_BE_TRUE);
        assert!( Bool::<(), ()>::CAN_BE_FALSE);

        assert!(!Bool::<! , ()>::CAN_BE_TRUE);
        assert!( Bool::<! , ()>::CAN_BE_FALSE);

        assert!( Bool::<(),  !>::CAN_BE_TRUE);
        assert!(!Bool::<(),  !>::CAN_BE_FALSE);

        assert!(!Bool::<! ,  !>::CAN_BE_TRUE);
        assert!(!Bool::<! ,  !>::CAN_BE_FALSE);
    }

    #[test]
    fn correct_and() {
        assert_eq!(True .and(True),  True.into());
        assert_eq!(False.and(True),  False.into());
        assert_eq!(True .and(False), False.into());
        assert_eq!(False.and(False), False.into());
    }

    #[test]
    fn correct_or() {
        assert_eq!(True .or(True),  True.into());
        assert_eq!(False.or(True),  True.into());
        assert_eq!(True .or(False), True.into());
        assert_eq!(False.or(False), False.into());
    }

    #[test]
    fn correct_not() {
        assert_eq!(True .not(), False.into());
        assert_eq!(False.not(), True.into());
    }
}
