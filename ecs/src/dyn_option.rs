use dynvec::DynVec;

pub trait DynOption {
    fn take_and_push_into(
        &mut self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>>;

    fn take_and_set_into(
        &mut self, idx: usize, into: &mut DynVec
    ) -> Option<Result<(), dynvec::InsertionError>>;
}

impl<T: 'static> DynOption for Option<T> {
    fn take_and_push_into(
        &mut self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        let val = self.take()?;
        Some(try { into.typed_mut::<T>()?.push(val) })
    }

    fn take_and_set_into(
        &mut self, idx: usize, into: &mut DynVec
    ) -> Option<Result<(), dynvec::InsertionError>> {
        let val = self.take()?;
        Some(try { into.typed_mut::<T>()?.set(idx, val)? })
    }
}
