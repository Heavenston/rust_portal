use std::any::Any;

use utils::dyn_clone;

use crate::world::Component;

pub trait ComponentVec {
    fn dyn_len(&self) -> usize;

    fn dyn_get(&self, idx: usize) -> Option<&dyn Component>;
    fn dyn_get_mut(&mut self, idx: usize) -> Option<&mut dyn Component>;

    fn dyn_swap_remove(&mut self, idx: usize) -> Box<dyn Component>;
    fn dyn_push(&mut self, val: Box<dyn Component>);
    fn dyn_set(&mut self, idx: usize, val: Box<dyn Component>);
}

impl<T: Component + 'static> ComponentVec for Vec<T> {
    fn dyn_len(&self) -> usize {
        self.len()
    }

    fn dyn_get(&self, idx: usize) -> Option<&dyn Component> {
        self.as_slice().get(idx).map(|p| -> &dyn Component { p })
    }

    fn dyn_get_mut(&mut self, idx: usize) -> Option<&mut dyn Component> {
        self.as_mut_slice().get_mut(idx).map(|p| -> &mut dyn Component { p })
    }

    fn dyn_swap_remove(&mut self, idx: usize) -> Box<dyn Component> {
        Box::new(self.swap_remove(idx))
    }

    fn dyn_push(&mut self, val: Box<dyn Component>) {
        self.push(*(val as Box<dyn Any>).downcast().expect("Correct type"))
    }

    fn dyn_set(&mut self, idx: usize, val: Box<dyn Component>) {
        self[idx] = *(val as Box<dyn Any>).downcast().expect("Correct type");
    }
}

pub trait ComponentVecFactory: dyn_clone::DynClone {
    fn create(&self) -> Box<dyn ComponentVec>;
}
dyn_clone::clone_trait_object!(ComponentVecFactory);

impl<T> ComponentVecFactory for T
    where T: dyn_clone::DynClone + (Fn() -> Box<dyn ComponentVec>),
{
    fn create(&self) -> Box<dyn ComponentVec> {
        (self)()
    }
}
