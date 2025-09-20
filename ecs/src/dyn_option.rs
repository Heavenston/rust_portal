use std::{any::TypeId, cell::Cell};

use dynvec::DynVec;

pub trait CellDynOption {
    fn value_type_id(&self) -> TypeId;
    fn value_type_name(&self) -> &'static str;

    fn take_and_push_into(
        &self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>>;

    fn take_and_set_into(
        &self, idx: usize, into: &mut DynVec
    ) -> Option<Result<(), dynvec::InsertionError>> {
        match into.get_mut(idx) {
            Ok(mut_ref) => match self.take_and_assign(mut_ref)? {
                Ok(()) => Some(Ok(())),
                Err(error) => Some(Err(dynvec::InsertionError::IncorrectType(error))),
            },
            Err(error) => Some(Err(dynvec::InsertionError::IndexOutOfBound(error))),
        }
    }

    fn take_and_assign(
        &self, target: dynvec::DynVecValueRefMut,
    ) -> Option<Result<(), dynvec::IncorrectTypeError>>;
}

impl<T> CellDynOption for Cell<T>
    where T: DynOption + Default
{
    fn value_type_id(&self) -> TypeId {
        let val = self.take();
        let type_id = val.value_type_id();
        self.set(val);
        type_id
    }

    fn value_type_name(&self) -> &'static str {
        let val = self.take();
        let type_name = val.value_type_name();
        self.set(val);
        type_name
    }

    fn take_and_push_into(
        &self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        self.take().take_and_push_into(into)
    }

    fn take_and_assign(
        &self, target: dynvec::DynVecValueRefMut,
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        self.take().take_and_assign(target)
    }
}

pub trait DynOption {
    fn value_type_id(&self) -> TypeId;
    fn value_type_name(&self) -> &'static str;

    fn take_and_push_into(
        &mut self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>>;

    fn take_and_set_into(
        &mut self, idx: usize, into: &mut DynVec
    ) -> Option<Result<(), dynvec::InsertionError>> {
        match into.get_mut(idx) {
            Ok(mut_ref) => match self.take_and_assign(mut_ref)? {
                Ok(()) => Some(Ok(())),
                Err(error) => Some(Err(dynvec::InsertionError::IncorrectType(error))),
            },
            Err(error) => Some(Err(dynvec::InsertionError::IndexOutOfBound(error))),
        }
    }

    fn take_and_assign(
        &mut self, target: dynvec::DynVecValueRefMut,
    ) -> Option<Result<(), dynvec::IncorrectTypeError>>;
}

impl<T: 'static> DynOption for Option<T> {
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn take_and_push_into(
        &mut self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        let val = self.take()?;
        Some(try { into.typed_mut::<T>()?.push(val) })
    }

    fn take_and_assign(
        &mut self, target: dynvec::DynVecValueRefMut,
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        let val = self.take()?;
        Some(try { *target.as_typed()? = val; })
    }
}

#[derive(Debug)]
#[derive_where::derive_where(Default)]
pub struct FunDynOption<F>(Option<F>);

impl<F> FunDynOption<F> {
    pub fn new(fun: F) -> Self {
        Self(Some(fun))
    }
}

impl<F, T: 'static> DynOption for FunDynOption<F>
    where F: FnOnce() -> T,
{
    fn value_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }

    fn take_and_push_into(
        &mut self, into: &mut DynVec
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        let val = self.0.take()?();
        Some(try { into.typed_mut::<T>()?.push(val) })
    }

    fn take_and_assign(
        &mut self, target: dynvec::DynVecValueRefMut,
    ) -> Option<Result<(), dynvec::IncorrectTypeError>> {
        let val = self.0.take()?();
        Some(try { *target.as_typed()? = val; })
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
