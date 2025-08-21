use std::ops::Not;

use crate::sign::{ Sign, Signed };

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrictSign {
    Negative,
    Positive,

    // prevents converting to number with `as`
    Never(!),
}

impl StrictSign {
    pub const VARIANT_COUNT: usize = 2;
    pub const VARIANTS: [StrictSign; Self::VARIANT_COUNT] = [
        Self::Negative,
        Self::Positive,
    ];

    pub fn sign_of(of: &impl StrictlySigned) -> Self {
        of.strict_sign()
    }

    pub fn try_sign_of(of: &impl Signed) -> Option<Self> {
        of.sign().try_into().ok()
    }

    pub fn is_negative(self) -> bool {
        matches!(self, Self::Negative)
    }

    pub fn is_positive(self) -> bool {
        matches!(self, Self::Positive)
    }

    pub fn idx(self) -> usize {
        match self {
            StrictSign::Negative => 0,
            StrictSign::Positive => 1,
        }
    }

    pub fn as_bit(self) -> u32 {
        1u32 << self.idx()
    }

    pub fn as_i8(self) -> i8 {
        match self {
            StrictSign::Negative => -1,
            StrictSign::Positive =>  1,
        }
    }

    pub fn opposit(self) -> Self {
        !self
    }
}

impl From<StrictSign> for Sign {
    fn from(value: StrictSign) -> Sign {
        match value {
            StrictSign::Negative => Sign::Negative,
            StrictSign::Positive => Sign::Positive,
        }
    }
}

impl TryFrom<Sign> for StrictSign {
    type Error = ();

    fn try_from(sign: Sign) -> Result<Self, ()> {
        match sign {
            Sign::Negative => Ok(StrictSign::Negative),
            Sign::Zero => Err(()),
            Sign::Positive => Ok(StrictSign::Positive),
        }
    }
}

impl Not for StrictSign {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            StrictSign::Negative => Self::Positive,
            StrictSign::Positive => Self::Negative,
        }
    }
}

pub trait StrictlySigned {
    fn strict_sign(&self) -> StrictSign;
}

impl StrictlySigned for f32 {
    fn strict_sign(&self) -> StrictSign {
        if self.is_sign_positive() {
            StrictSign::Positive
        }
        else {
            debug_assert!(self.is_sign_negative());
            StrictSign::Negative
        }
    }
}

impl StrictlySigned for f64 {
    fn strict_sign(&self) -> StrictSign {
        if self.is_sign_positive() {
            StrictSign::Positive
        }
        else {
            debug_assert!(self.is_sign_negative());
            StrictSign::Negative
        }
    }
}
