
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sign {
    Negative,
    Zero,
    Positive,

    // prevents converting to number with `as`
    Never(!),
}

impl Sign {
    pub const VARIANT_COUNT: usize = 3;
    pub const VARIANTS: [Sign; Self::VARIANT_COUNT] = [
        Self::Negative,
        Self::Zero,
        Self::Positive,
    ];

    pub fn sign_of(of: &impl Signed) -> Self {
        of.sign()
    }

    pub fn is_negative(self) -> bool {
        matches!(self, Self::Negative)
    }

    pub fn is_zero(self) -> bool {
        matches!(self, Self::Zero)
    }

    pub fn is_positive(self) -> bool {
        matches!(self, Self::Positive)
    }

    pub fn idx(self) -> usize {
        match self {
            Sign::Negative => 0,
            Sign::Zero => 1,
            Sign::Positive => 2,
        }
    }

    pub fn as_bit(self) -> u32 {
        1u32 << self.idx()
    }

    pub fn as_i8(self) -> i8 {
        match self {
            Sign::Negative => -1,
            Sign::Zero     =>  0,
            Sign::Positive =>  1,
        }
    }
}

pub trait Signed {
    fn sign(&self) -> Sign;
}

impl Signed for i8 {
    fn sign(&self) -> Sign {
        match *self {
            ..=-1 => Sign::Negative,
            0     => Sign::Zero,
            1..   => Sign::Positive,
        }
    }
}

impl Signed for i16 {
    fn sign(&self) -> Sign {
        match *self {
            ..=-1 => Sign::Negative,
            0     => Sign::Zero,
            1..   => Sign::Positive,
        }
    }
}

impl Signed for i32 {
    fn sign(&self) -> Sign {
        match *self {
            ..=-1 => Sign::Negative,
            0     => Sign::Zero,
            1..   => Sign::Positive,
        }
    }
}

impl Signed for i64 {
    fn sign(&self) -> Sign {
        match *self {
            ..=-1 => Sign::Negative,
            0     => Sign::Zero,
            1..   => Sign::Positive,
        }
    }
}

impl Signed for i128 {
    fn sign(&self) -> Sign {
        match *self {
            ..=-1 => Sign::Negative,
            0     => Sign::Zero,
            1..   => Sign::Positive,
        }
    }
}

impl Signed for f32 {
    fn sign(&self) -> Sign {
        // both -0 and +0 are reported to be 0 here
        if *self == 0f32 {
            Sign::Zero
        }
        else if self.is_sign_positive() {
            Sign::Positive
        }
        else {
            debug_assert!(self.is_sign_negative());
            Sign::Negative
        }
    }
}

impl Signed for f64 {
    fn sign(&self) -> Sign {
        // both -0 and +0 are reported to be 0 here
        if *self == 0f64 {
            Sign::Zero
        }
        else if self.is_sign_positive() {
            Sign::Positive
        }
        else {
            debug_assert!(self.is_sign_negative());
            Sign::Negative
        }
    }
}
