
mod sealed {
    pub trait PrivateBoolValue { }
    pub trait PrivatePartialBool { }
}

pub trait BoolValue: std::fmt::Debug + Copy + sealed::PrivateBoolValue {
    const INHABITED: bool;
    /// Type is INHABITED if both Self and O are INHABITED
    type And<O: BoolValue>: BoolValue;
    /// Type is INHABITED if one of Self or O are INHABITED
    type Or<O: BoolValue>: BoolValue;
    type Not: BoolValue;

    fn and<O: BoolValue>(self, other: O) -> Self::And<O>;
    fn or_left<O: BoolValue>(self) -> Self::Or<O>;
    fn or_right<O: BoolValue>(_: O) -> Self::Or<O>;
}

impl sealed::PrivateBoolValue for () { }
impl BoolValue for () {
    const INHABITED: bool = true;
    type And<O: BoolValue> = O;
    type Or<O: BoolValue> = ();
    type Not = !;

    fn and<O: BoolValue>(self: (), other: O) -> Self::And<O> { other }
    fn or_left<O: BoolValue>(self: ()) -> Self::Or<O> { () }
    fn or_right<O: BoolValue>(_: O) -> Self::Or<O> { () }
}

impl sealed::PrivateBoolValue for ! { }
impl BoolValue for ! {
    const INHABITED: bool = false;
    type And<O: BoolValue> = !;
    type Or<O: BoolValue> = O;
    type Not = ();

    /// Takes never as argument so we know we could never call this method
    fn and<O: BoolValue>(self: !, _: O) -> Self::And<O> { unreachable!() }

    /// Takes never as argument so we know we could never call this method
    fn or_left<O: BoolValue>(self: !) -> Self::Or<O> { unreachable!() }
    fn or_right<O: BoolValue>(other: O) -> Self::Or<O> { other }
}

pub type BoolValueAnd<A, B> = <A as BoolValue>::And<B>;
pub type BoolValueOr<A, B> = <A as BoolValue>::Or<B>;
pub type BoolValueNot<A> = <A as BoolValue>::Not;

pub type BoolAnd<A, B> = Bool<<<A as PartialBool>::T as BoolValue>::And<<B as PartialBool>::T>, <<A as PartialBool>::F as BoolValue>::Or<<B as PartialBool>::F>>;
pub type BoolOr<A, B> = Bool<<<A as PartialBool>::T as BoolValue>::Or<<B as PartialBool>::T>, <<A as PartialBool>::F as BoolValue>::And<<B as PartialBool>::F>>;
pub type BoolNot<A> = Bool<<A as PartialBool>::F, <A as PartialBool>::T>;

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
            Bool::False(f1) => Bool::False(Self::F::or_left::<O::F>(f1)),
            Bool::True(t1) => match other().into() {
                Bool::False(f2) => Bool::False(Self::F::or_right::<O::F>(f2)),
                Bool::True(t2) => Bool::True(Self::T::and::<O::T>(t1, t2)),
            },
        }
    }

    fn or<O: PartialBool>(self, other: O) -> BoolOr<Self, O> {
        self.or_else(|| other)
    }

    fn or_else<O: PartialBool>(self, other: impl FnOnce() -> O) -> BoolOr<Self, O> {
        match self.into() {
            Bool::True(t1) => Bool::True(Self::T::or_left::<O::T>(t1)),
            Bool::False(f1) => match other().into() {
                Bool::True(t2) => Bool::True(Self::T::or_right::<O::T>(t2)),
                Bool::False(f2) => Bool::False(Self::F::and::<O::F>(f1, f2)),
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
            (Bool::True(t1), Bool::True(t2)) => Bool::True(Ta::and(t1, t2)),
            (Bool::False(f1), _) => Bool::False(Fa::or_left ::<Fb>(f1)),
            (_, Bool::False(f2)) => Bool::False(Fa::or_right::<Fb>(f2)),
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
            (Bool::False(f1), Bool::False(f2)) => Bool::False(Fa::and(f1, f2)),
            (Bool::True(t1), _) => Bool::True(Ta::or_left ::<Tb>(t1)),
            (_, Bool::True(t2)) => Bool::True(Ta::or_right::<Tb>(t2)),
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
