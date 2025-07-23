use crate::{ Renderer, World };

pub trait ApplicationFactory {
    fn create_application(&mut self, world: &mut World, renderer: &mut Renderer) -> Box<dyn Application>;
}

impl<F, A> ApplicationFactory for F
    where F: for<'a> FnMut(&mut World, &'a mut Renderer) -> A,
          A: Application + 'static
{
    fn create_application(&mut self, world: &mut World, renderer: &mut Renderer) -> Box<dyn Application> {
        Box::new(self(world, renderer))
    }
}


pub trait Application {
    fn update(&mut self, world: &mut World, renderer: &mut Renderer, dt: f32) {
        let _ = world;
        let _ = renderer;
        let _ = dt;
    }
}
