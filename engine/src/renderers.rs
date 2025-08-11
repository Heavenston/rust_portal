pub mod forward_renderer;

use glam::UVec2;

use crate::EngineState;

pub trait Renderer {
    fn resize(&mut self, state: &mut EngineState, new_size: UVec2) {
        let _ = state;
        let _ = new_size;
    }

    fn render(&mut self, state: &mut EngineState) {
        let _ = state;
    }
}
