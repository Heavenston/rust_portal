use std::sync::Arc;

use crate::renderer::Renderer;

struct StartedEngine {
    renderer: Renderer,
    window: Arc<winit::window::Window>,
}

pub struct Engine {
    window_attributes: winit::window::WindowAttributes,
    started: Option<StartedEngine>,
}

impl Engine {
    pub fn new(window_attributes: winit::window::WindowAttributes) -> Self {
        Self {
            window_attributes,
            started: None,
        }
    }

    fn render(&mut self) {
        let started = self.started.as_mut().unwrap();

        let mut render = started.renderer.render();
        let present_texture_handle = render.using_present_texture();

        {
            let render_pass = render.render_pass()
                    .color_attachment()
                    .texture_view_handle(present_texture_handle)
                    .color_clear(wgpu::Color::RED)
                    .finish()
                .build();
            render_pass.finish();
        }

        render.finish();
    }
}

impl winit::application::ApplicationHandler for Engine {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.started.is_some() {
            return;
        }

        let window = event_loop.create_window(self.window_attributes.clone()).unwrap();
        let window = Arc::new(window);

        self.started = Some(StartedEngine {
            renderer: Renderer::new(Arc::clone(&window)),
            window,
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(started) = &mut self.started
        else { return };

        started.window.request_redraw();
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

        use winit::event::WindowEvent as We;
        match event {
            We::Resized(physical_size) => {
                started.renderer.resize(physical_size);
            },
            We::CloseRequested => {
                self.started = None;
                event_loop.exit();
            },
            We::Destroyed => {
                self.started = None;
                event_loop.exit();
            },
            We::Focused(_) => {
                
            },
            We::KeyboardInput { device_id, event, is_synthetic } => {
                
            },
            We::ModifiersChanged(modifiers) => {
                
            },
            We::Ime(ime) => {
                
            },
            We::CursorMoved { device_id, position } => {
                
            },
            We::CursorEntered { device_id } => {
                
            },
            We::CursorLeft { device_id } => {
                
            },
            We::MouseWheel { device_id, delta, phase } => {
                
            },
            We::MouseInput { device_id, state, button } => {
                
            },
            We::AxisMotion { device_id, axis, value } => {
                
            },
            We::Touch(touch) => {
                
            },
            We::RedrawRequested => {
                self.render();
            },

            _ => (),
        }
    }
}
