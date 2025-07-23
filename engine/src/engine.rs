mod application;
pub use application::*;
use glam::Mat4;

use crate::{ * };

use std::{ sync::Arc, time::Instant };

struct StartedEngine {
    window: Arc<winit::window::Window>,

    world: World,
    renderer: Renderer,
    application: Box<dyn Application>,

    camera_uniform_buffer: BufferHandle,
    camera_bind_group: CameraBindGroupHandle,

    // FIXME: Probably not the best way to do this
    last_update: Option<Instant>,
}

impl StartedEngine {
    fn render(&mut self) {
        // 
        // Write into camera uniform buffer
        // 
        let Some(camera) = self.world.camera
        else { eprintln!("NO CAMERA"); return; };
        {
            let view = camera.transform.inverse();
            let proj = camera.projection;
            let view_proj = proj * view;
            self.renderer.write_buffer(
                self.camera_uniform_buffer,
                0,
                bytemuck::bytes_of(&view_proj),
            );
        }

        //
        // Set object uniforms for missing ones
        // 
        for (_, StaticMeshData { mesh, cache }) in self.world.static_meshes.iter_mut() {
            if cache.is_some() { continue }

            let transform: Mat4 = mesh.transform.into();
            let uniform_buffer = self.renderer.create_buffer()
                .size(size_of::<Mat4>() as u64)
                .data(bytemuck::bytes_of(&transform))
                .create();

            let object_bind_group = self.renderer.create_object_bind_group()
                .uniform_buffer(uniform_buffer)
                .create();

            // TODO: FIXME: Leaks buffers when static meshes are removed
            *cache = Some(StaticMeshGPUCache {
                uniform_buffer,
                object_bind_group,
            });
        }

        let mut render = self.renderer.render();
        let present_texture_handle = render.using_present_texture();

        {
            let mut render_pass = render.render_pass()
                    .color_attachment()
                    .texture_view_handle(present_texture_handle)
                    .color_clear(wgpu::Color::RED)
                    .finish()
                .build();

            for (_, StaticMeshData { mesh, cache }) in self.world.static_meshes.iter() {
                let cache = cache.as_ref().expect("Initialized before");

                render_pass.draw_call()
                    .camera_bind_group(self.camera_bind_group)
                    .object_bind_group(cache.object_bind_group)
                    .index_buffer(mesh.index_buffer)
                    .positions_buffer(mesh.positions_buffer)
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

        self.application.update(&mut self.world, &mut self.renderer, dt);
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

        let mut world = World::default();
        let mut renderer = Renderer::new(Arc::clone(&window));
        let application = self.application_factory.create_application(&mut world, &mut renderer);

        let camera_uniform_buffer = renderer.create_buffer()
            .size(size_of::<Mat4>() as u64)
            .create();
        let camera_bind_group = renderer.create_camera_bind_group()
            .uniform_buffer(camera_uniform_buffer)
            .create();

        self.started = Some(StartedEngine {
            window,

            world,
            renderer,
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
                started.render();
            },

            _ => (),
        }
    }
}
