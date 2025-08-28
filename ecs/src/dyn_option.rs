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

#[cfg(test)]
mod tests {
    use super::DynOption;
    use dynvec::DynVec;

    #[test]
    fn take_and_push_into_moves_value_and_clears_option() {
        let mut storage = DynVec::new::<i32>();
        let mut v = Some(7);

        // First push consumes value and writes into dynvec
        let r = v.take_and_push_into(&mut storage);
        assert!(matches!(r, Some(Ok(()))));
        assert!(v.is_none());
        let view = storage.typed::<i32>().unwrap();
        assert_eq!(view.as_slice(), &[7]);

        // Second push with None is a no-op and returns None
        let r2 = v.take_and_push_into(&mut storage);
        assert!(r2.is_none());
        let view = storage.typed::<i32>().unwrap();
        assert_eq!(view.as_slice(), &[7]);
    }

    #[test]
    fn take_and_set_into_writes_at_index() {
        let mut storage = DynVec::new::<&'static str>();
        // Seed with one element to set over
        storage.typed_mut::<&'static str>().unwrap().push("a");

        let mut v = Some("b");
        let r = v.take_and_set_into(0, &mut storage);
        assert!(matches!(r, Some(Ok(()))));
        assert!(v.is_none());
        assert_eq!(storage.typed::<&'static str>().unwrap().as_slice(), &["b"]);

        // None does nothing
        let r2 = v.take_and_set_into(0, &mut storage);
        assert!(r2.is_none());
    }
}
