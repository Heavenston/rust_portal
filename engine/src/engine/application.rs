use super::*;

pub trait ApplicationFactory {
    fn create_application(&mut self, engine_state: &mut EngineState) -> Box<dyn Application>;
}

impl<F, A> ApplicationFactory for F
    where F: for<'a> FnMut(&'a mut EngineState) -> A,
          A: Application + 'static
{
    fn create_application(&mut self, engine_state: &mut EngineState) -> Box<dyn Application> {
        Box::new(self(engine_state))
    }
}

pub trait Application {
    fn update(&mut self, engine_state: &mut EngineState, dt: f32) {
        let _ = engine_state;
        let _ = dt;
    }
}
