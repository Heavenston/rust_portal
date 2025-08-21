use super::Axis;

#[derive(Debug, Clone, Copy)]
pub struct AxisInBitfieldIterator {
    pub(super) field: u32,
    pub(super) idx: usize,
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
