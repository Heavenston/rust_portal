use std::ops::{BitOr, Not};

use glam::IVec3;
use crate::{ axis::Axis, strict_sign::StrictSign };

#[derive(Debug, Clone, Copy)]
pub struct AxisDirectionInBitfieldIterator {
    field: u32,
    idx: usize,
}

impl Iterator for AxisDirectionInBitfieldIterator {
    type Item = AxisDirection;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx >= AxisDirection::VARIANTS.len() {
                return None;
            }
            let dir = AxisDirection::VARIANTS[self.idx];
            self.idx += 1;
            if (self.field & dir.as_bit()) != 0 {
                return Some(dir);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let direction_bits = AxisDirection::VARIANTS.into_iter()
            .map(AxisDirection::as_bit)
            .sum::<u32>();
        // only count the bits that are valid directions for accurate length
        let size = u32::count_ones(direction_bits & self.field);
        let size = usize::try_from(size).expect("no overflow");

        (size, Some(size))
    }
}

impl ExactSizeIterator for AxisDirectionInBitfieldIterator { }

// TODO: Rename to SignedAxis or AxisSigned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    PosX,
    NegX,
    PosY,
    NegY,
    PosZ,
    NegZ,
    /// Prevents converting to integer with `as`
    Never(!),
}

impl AxisDirection {
    pub const VARIANT_COUNT: usize = 6;
    pub const VARIANTS: [AxisDirection; Self::VARIANT_COUNT] = [
        Self::PosX, Self::NegX,
        Self::PosY, Self::NegY,
        Self::PosZ, Self::NegZ,
    ];
    pub const BITS_ALL: u32 = 0x3F;

    pub fn new(axis: Axis, sign: StrictSign) -> Self {
        match (axis, sign) {
            (Axis::X, StrictSign::Negative) => Self::NegX,
            (Axis::X, StrictSign::Positive) => Self::PosX,
            (Axis::Y, StrictSign::Negative) => Self::NegY,
            (Axis::Y, StrictSign::Positive) => Self::PosY,
            (Axis::Z, StrictSign::Negative) => Self::NegZ,
            (Axis::Z, StrictSign::Positive) => Self::PosZ,
        }
    }

    pub fn idx(self) -> usize {
        match self {
            AxisDirection::PosX => 0,
            AxisDirection::NegX => 1,
            AxisDirection::PosY => 2,
            AxisDirection::NegY => 3,
            AxisDirection::PosZ => 4,
            AxisDirection::NegZ => 5,
        }
    }

    pub fn as_bit(self) -> u32 {
        1u32 << self.idx()
    }

    pub fn axis(self) -> Axis {
        match self {
            AxisDirection::PosX | AxisDirection::NegX => Axis::X,
            AxisDirection::PosY | AxisDirection::NegY => Axis::Y,
            AxisDirection::PosZ | AxisDirection::NegZ => Axis::Z,
        }
    }

    pub fn sign(self) -> StrictSign {
        match self {
            AxisDirection::PosX | AxisDirection::PosY | AxisDirection::PosZ => StrictSign::Positive,
            AxisDirection::NegX | AxisDirection::NegY | AxisDirection::NegZ => StrictSign::Negative,
        }
    }

    pub fn as_diff(self) -> IVec3 {
        IVec3::from(self.axis().as_bvec3()) * i32::from(self.sign().as_i8())
    }

    pub fn opposit(self) -> Self {
        !self
    }

    pub fn iter_in_bitfield(field: u32) -> AxisDirectionInBitfieldIterator {
        AxisDirectionInBitfieldIterator { field, idx: 0 }
    }
}

impl Not for AxisDirection {
    type Output = Self;

    fn not(self) -> Self::Output {
        Self::new(self.axis(), !self.sign())
    }
}

impl BitOr<AxisDirection> for AxisDirection {
    type Output = u32;

    fn bitor(self, rhs: Self) -> Self::Output {
        self.as_bit() | rhs.as_bit()
    }
}

impl BitOr<AxisDirection> for u32 {
    type Output = u32;

    fn bitor(self, rhs: AxisDirection) -> Self::Output {
        self | rhs.as_bit()
    }
}

impl BitOr<u32> for AxisDirection {
    type Output = u32;

    fn bitor(self, rhs: u32) -> Self::Output {
        self.as_bit() | rhs
    }
}

#[test]
fn test_bits() {
    debug_assert_eq!(AxisDirection::VARIANTS.into_iter()
        .fold(0u32, |sum, dir| sum | dir),
        AxisDirection::BITS_ALL);
}

#[test]
fn test_bits_iter() {
    let alls = AxisDirection::iter_in_bitfield(AxisDirection::BITS_ALL).collect::<Vec<_>>();
    debug_assert_eq!(
        &alls[..],
        &AxisDirection::VARIANTS[..],
    );
}
