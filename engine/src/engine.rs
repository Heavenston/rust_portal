mod application;
pub use application::*;
mod state;
pub use state::*;
mod materials;
pub use materials::*;
mod pbr_material;
pub use pbr_material::*;

use crate::{ * };

use std::{ sync::Arc, time::Instant };
use glam::Mat4;

struct StartedEngine {
    window: Arc<winit::window::Window>,

    state: EngineState,
    application: Box<dyn Application>,

    camera_uniform_buffer: BufferHandle,
    camera_bind_group: BindGroupHandle,

    // FIXME: Probably not the best way to do this
    last_update: Option<Instant>,
}

impl StartedEngine {
    fn render(&mut self) {
        let state = &mut self.state;

        // 
        // Write into camera uniform buffer
        // 
        let Some(camera) = state.camera
        else { eprintln!("NO CAMERA"); return; };
        {
            let view = camera.transform.inverse();
            let proj = camera.projection;
            let view_proj = proj * view;
            state.renderer.write_buffer(
                self.camera_uniform_buffer,
                0,
                bytemuck::bytes_of(&view_proj),
            );
        }

        let mut render = state.renderer.render();
        let present_texture_handle = render.using_present_texture();
        let depth_buffer_handle = render.using_depth_buffer();

        // Object rendering pass
        {
            let mut render_pass = render.render_pass()
                .color_attachment()
                    .texture_view_handle(present_texture_handle)
                    .color_clear(camera.clear_color)
                    .finish()
                .depth_stencil_attachment()
                    .texture_view_handle(depth_buffer_handle)
                    .depth_ops(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Discard,
                    })
                    .finish()
                .build();

            for (_, StaticMeshData { mesh, object_bind_group }) in state.static_meshes.iter() {
                render_pass.set_pipeline(
                    state.materials.get(mesh.material_instance.material).pipeline
                );
                render_pass.set_index_buffer(mesh.index_buffer);
                render_pass.set_vertex_buffer(0, mesh.positions_buffer);
                render_pass.set_vertex_buffer(1, mesh.texcoords_buffer);
                render_pass.set_bind_group(0, self.camera_bind_group);
                render_pass.set_bind_group(1, *object_bind_group);
                render_pass.set_bind_group(2, mesh.material_instance.bind_group);
                render_pass.draw_call()
                    .draw(0..mesh.vertex_count);
            }

            render_pass.finish();
        }

        render.finish();
    }

    fn update(&mut self) {
        let dt = self.last_update
            .map(|instant| instant.elapsed().as_secs_f32())
            .unwrap_or(0.);
        self.last_update = Some(Instant::now());

        self.application.update(&mut self.state, dt);
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

        let window = event_loop.create_window(self.window_attributes.clone()).unwrap();
        let window = Arc::new(window);

        let mut state = EngineState::new(Renderer::new(Arc::clone(&window)));
        let application = self.application_factory.create_application(&mut state);

        let camera_uniform_buffer = state.renderer.create_buffer()
            .size(size_of::<Mat4>() as u64)
            .create();
        let camera_bind_group = state.renderer.create_bind_group()
            .layout(state.camera_bind_group_layout)
            .entry(0, camera_uniform_buffer)
            .create();

        self.started = Some(StartedEngine {
            window,

            state,
            application,

            camera_uniform_buffer,
            camera_bind_group,

            last_update: None,
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(started) = &mut self.started
        else { return };

        // TODO: FIXME: Separate thread?
        started.update();
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
                started.state.renderer.resize(physical_size);
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
                started.render();
            },

            _ => (),
        }
    }
}
