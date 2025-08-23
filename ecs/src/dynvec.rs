use std::any::Any;

pub trait DynVec {
    fn dyn_len(&self) -> usize;

    fn dyn_get(&self, idx: usize) -> Option<&dyn Any>;
    fn dyn_get_mut(&mut self, idx: usize) -> Option<&mut dyn Any>;

    fn dyn_swap_remove(&mut self, idx: usize);
}

impl<T: 'static> DynVec for Vec<T> {
    fn dyn_len(&self) -> usize {
        self.len()
    }

    fn dyn_get(&self, idx: usize) -> Option<&dyn Any> {
        self.as_slice().get(idx).map(|p| -> &dyn Any { p })
    }

    fn dyn_get_mut(&mut self, idx: usize) -> Option<&mut dyn Any> {
        self.as_mut_slice().get_mut(idx).map(|p| -> &mut dyn Any { p })
    }

    fn dyn_swap_remove(&mut self, idx: usize) {
        self.swap_remove(idx);
    }
}
