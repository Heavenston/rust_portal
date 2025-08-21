use std::ops::BitOr;

mod vec_helper;
pub use vec_helper::*;
mod iterator;
pub use iterator::*;

use glam::BVec3;

use crate::prelude::{AxisDirection, StrictSign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Axis {
    X, Y, Z,
    /// Prevents converting to integer with `as`
    Never(!),
}

impl Axis {
    pub const VARIANT_COUNT: usize = 3;
    pub const VARIANTS: [Axis; Self::VARIANT_COUNT] = [
        Self::X, Self::Y, Self::Z,
    ];
    pub const BITS_ALL: u32 = 0x7;

    pub fn idx(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }

    pub fn from_idx(idx: usize) -> Self {
        assert!(idx < 3);
        match idx {
            0 => Self::X,
            1 => Self::Y,
            2 => Self::Z,
            _ => unreachable!(),
        }
    }

    pub fn as_bit(self) -> u32 {
        1u32 << self.idx()
    }

    pub fn as_bvec3(self) -> BVec3 {
        match self {
            Axis::X => BVec3::new(true,  false, false),
            Axis::Y => BVec3::new(false, true,  false),
            Axis::Z => BVec3::new(false, false, true),
        }
    }

    pub fn iter_in_bitfield(field: u32) -> AxisInBitfieldIterator {
        AxisInBitfieldIterator { field, idx: 0 }
    }

    pub fn with_sign(self, sign: StrictSign) -> AxisDirection {
        AxisDirection::new(self, sign)
    }
}

impl BitOr<Axis> for Axis {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.as_bit() | rhs.as_bit()
    }
}

impl BitOr<Axis> for u32 {
    type Output = u32;

    fn bitor(self, rhs: Axis) -> Self::Output {
        self | rhs.as_bit()
    }
}

impl BitOr<u32> for Axis {
    type Output = u32;

    fn bitor(self, rhs: u32) -> Self::Output {
        self.as_bit() | rhs
    }
}

#[test]
fn test_bits() {
    debug_assert_eq!(Axis::VARIANTS.into_iter()
        .fold(0u32, |sum, dir| sum | dir),
        Axis::BITS_ALL);
}

#[test]
fn test_bits_iter() {
    let alls = Axis::iter_in_bitfield(Axis::BITS_ALL).collect::<Vec<_>>();
    debug_assert_eq!(
        &alls[..],
        &Axis::VARIANTS[..],
    );
}
