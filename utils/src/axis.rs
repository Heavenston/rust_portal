use std::ops::BitOr;

use glam::BVec3;

#[derive(Debug, Clone, Copy)]
pub struct AxisInBitfieldIterator {
    field: u32,
    idx: usize,
}

impl Iterator for AxisInBitfieldIterator {
    type Item = Axis;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.idx >= Axis::VARIANTS.len() {
                return None;
            }
            let dir = Axis::VARIANTS[self.idx];
            self.idx += 1;
            if (self.field & dir.as_bit()) != 0 {
                return Some(dir);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let direction_bits = Axis::VARIANTS.into_iter()
            .map(Axis::as_bit)
            .sum::<u32>();
        // only count the bits that are valid directions for accurate length
        let size = u32::count_ones(direction_bits & self.field);
        let size = usize::try_from(size).expect("no overflow");

        (size, Some(size))
    }
}

impl ExactSizeIterator for AxisInBitfieldIterator { }

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
