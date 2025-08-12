mod application;
pub use application::*;
mod state;
use glam::{ DVec2, UVec2 };
pub use state::*;
mod materials;
pub use materials::*;
pub mod pbr_material;
pub mod hdr_tonemapper_material;
pub mod input;

use crate::{input::InputButton, renderers::{forward_renderer::ForwardRenderer, Renderer}};

use pgk::GraphicsKernel;

use std::{ sync::Arc, time::Instant };

struct StartedEngine {
    window: Arc<winit::window::Window>,

    state: EngineState,
    application: Box<dyn Application>,
    renderer: Box<dyn Renderer>,

    // FIXME: Probably not the best way to do this
    last_update: Option<Instant>,
}

impl StartedEngine {
    fn render(&mut self) {
        let state = &mut self.state;
        let kernel = &mut state.kernel;

        if cfg!(debug_assertions) {
            state.materials.recreate_outdated(kernel);
        }

        self.renderer.render(state);
    }

    fn pre_frame(&mut self) {
        let dt = self.last_update
            .map(|instant| instant.elapsed().as_secs_f32())
            .unwrap_or(0.);
        self.last_update = Some(Instant::now());

        self.application.pre_frame(&mut self.state, dt);
        self.state.input.pre_frame_apply_changes(&self.window);
    }
}

#[derive(bon::Builder)]
pub struct Engine {
    #[builder(default)]
    window_attributes: winit::window::WindowAttributes,
    #[builder(with = |factory: impl ApplicationFactory + 'static| Box::new(factory))]
    application_factory: Box<dyn ApplicationFactory>,
    #[builder(skip)]
    started: Option<StartedEngine>,
}

impl winit::application::ApplicationHandler for Engine {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.started.is_some() {
            return;
        }

        let window = event_loop.create_window(self.window_attributes.clone()
            .with_visible(false)
        ).unwrap();
        let window = Arc::new(window);

        let mut state = EngineState::new(GraphicsKernel::new(Arc::clone(&window)));
        let application = self.application_factory.create_application(&mut state);
        let renderer = Box::new(ForwardRenderer::new(&mut state));

        window.set_visible(true);

        self.started = Some(StartedEngine {
            window,

            state,
            application,
            renderer,

            last_update: None,
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(started) = &mut self.started
        else { return };

        started.pre_frame();
        started.window.request_redraw();
    }

    fn device_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        let Some(started) = &mut self.started
        else {
            event_loop.exit();
            return;
        };

        use winit::event::{
            DeviceEvent as De,
            RawKeyEvent, MouseScrollDelta,
        };
        use winit::keyboard::{
            PhysicalKey,
        };
        match event {
            De::MouseMotion { delta: (dx, dy) } => {
                started.state.input.register_mouse_motion(DVec2::new(dx, dy).as_vec2());
            },
            De::MouseWheel { delta: MouseScrollDelta::LineDelta(_, dy) } => {
                if dy > 0. {
                    started.state.input.register_tapped(InputButton::MouseWheelDown);
                }
                else if dy < 0. {
                    started.state.input.register_tapped(InputButton::MouseWheelUp);
                }
            },
            De::Key(RawKeyEvent { physical_key: PhysicalKey::Code(keycode), state }) => {
                started.state.input.register_button_state(keycode, state.is_pressed());
            },

            _ => (),
        }
    }

    #[allow(unused_variables)]
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let Some(started) = &mut self.started
        else {
            event_loop.exit();
            return;
        };
        debug_assert!(window_id == started.window.id());

        use winit::event::{
            WindowEvent as We,
            ElementState,
        };
        match event {
            We::Resized(physical_size) => {
                started.state.kernel.resize(physical_size);
                started.renderer.resize(&mut started.state, UVec2::new(
                    physical_size.width,
                    physical_size.height,
                ));
            },
            We::Destroyed | We::CloseRequested => {
                self.started = None;
                event_loop.exit();
            },
            We::Focused(focused) => {
                if focused {
                    started.state.input.register_pressed(InputButton::WindowFocused);
                }
                else {
                    started.state.input.register_released(InputButton::WindowFocused);
                }
            },
            We::CursorMoved { device_id, position } => {
                started.state.input.register_mouse_pos(DVec2::new(
                    position.x, position.y
                ).as_vec2());
            },
            We::CursorEntered { device_id } => {
                started.state.input.register_pressed(InputButton::MouseEntered);
            },
            We::CursorLeft { device_id } => {
                started.state.input.register_released(InputButton::MouseEntered);
            },
            We::MouseInput { device_id, state, button } => {
                match state {
                    ElementState::Pressed => {
                        started.state.input.register_pressed(button);
                    },
                    ElementState::Released => {
                        started.state.input.register_released(button);
                    },
                }
            },
            We::RedrawRequested => {
                started.render();
            },

            _ => (),
        }
    }
}
