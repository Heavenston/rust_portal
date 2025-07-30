mod application;
pub use application::*;
mod state;
pub use state::*;
mod materials;
pub use materials::*;
pub mod pbr_material;

use crate::{ utils::default, *, color::* };

use std::{ iter::empty, sync::Arc, time::Instant };
use crevice::std140::AsStd140;

struct StartedEngine {
    window: Arc<winit::window::Window>,

    state: EngineState,
    application: Box<dyn Application>,

    world_uniform_buffer: BufferHandle,
    lights_uniform_buffer: BufferHandle,
    world_bind_group: BindGroupHandle,

    // FIXME: Probably not the best way to do this
    last_update: Option<Instant>,
}

impl StartedEngine {
    fn render(&mut self) {
        let state = &mut self.state;

        #[cfg(debug_assertions)]
        state.materials.recreate_outdated(&mut state.renderer);

        // 
        // Write into world uniform buffer
        // 
        let Some(camera) = state.camera
        else { eprintln!("NO CAMERA"); return; };

        {
            let mut lights: Vec<pbr_material::Std140Light> = empty()
                .chain(
                    state.directional_lights.iter().map(|(_, b)| b.directional_light)
                    .map(|dl| pbr_material::Light {
                        kind: pbr_material::LightKind::Directional,
                        direction: dl.direction,
                        color: dl.color.into(),
                        intensity: dl.intensity,
                        
                        position: default(),
                        inner_cone_angle: default(),
                        outer_cone_angle: default(),
                    })
                    .map(|light| light.as_std140())
                )
                .chain(
                    state.spot_lights.iter().map(|(_, b)| b.spot_light)
                    .map(|spot_light| pbr_material::Light {
                        kind: pbr_material::LightKind::Spot,
                        position: spot_light.position,
                        direction: spot_light.direction,
                        color: spot_light.color.into(),
                        intensity: spot_light.intensity,
                        inner_cone_angle: spot_light.inner_cone_angle,
                        outer_cone_angle: spot_light.outer_cone_angle,
                    })
                    .map(|light| light.as_std140())
                )
                .collect();
            println!("Light count: {}/{}", lights.len(), pbr_material::MAX_LIGHTS);
            lights.truncate(pbr_material::MAX_LIGHTS.try_into().unwrap());
            
            let view = camera.transform.inverse();
            let proj = camera.projection;
            let view_proj = proj * view;
            let uniforms = pbr_material::WorldUniforms {
                view_projection: view_proj,
                camera_world_pos: camera.transform.translation.into(),
                light_count: lights.len().try_into().unwrap(),
            };
            state.renderer.write_buffer(
                self.world_uniform_buffer, 0,
                bytemuck::bytes_of(&uniforms.as_std140()),
            );
            state.renderer.write_buffer(
                self.lights_uniform_buffer, 0,
                bytemuck::cast_slice(&lights),
            );
        }

        let mut render = state.renderer.render();
        let present_texture_handle = render.using_present_texture();
        let depth_buffer_handle = render.using_depth_buffer();

        let mut draw_all_number = 0;

        // Object rendering pass
        {
            let cc = LinearRgba::from(camera.clear_color);
            let mut render_pass = render.render_pass()
                .color_attachment()
                    .texture_view_handle(present_texture_handle)
                    .color_clear(wgpu::Color {
                        r: cc.r.into(),
                        g: cc.g.into(),
                        b: cc.b.into(),
                        a: cc.a.into(),
                    })
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
                render_pass.set_vertex_buffer(2, mesh.normals_buffer);
                render_pass.set_bind_group(0, self.world_bind_group);
                render_pass.set_bind_group(1, *object_bind_group);
                render_pass.set_bind_group(2, mesh.material_instance.bind_group);
                render_pass.draw_call()
                    .draw(0..mesh.vertex_count);

                draw_all_number += 1;
            }

            render_pass.finish();
        }

        println!("{draw_all_number} draw calls");

        render.finish();
    }

    fn pre_frame(&mut self) {
        let dt = self.last_update
            .map(|instant| instant.elapsed().as_secs_f32())
            .unwrap_or(0.);
        self.last_update = Some(Instant::now());

        self.application.pre_frame(&mut self.state, dt);
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

        let world_uniform_buffer = state.renderer.create_buffer()
            .size(pbr_material::WorldUniforms::std140_size_static().try_into().unwrap())
            .create();
        let lights_uniform_buffer = state.renderer.create_buffer()
            .size(u64::try_from(pbr_material::Light::std140_size_static()).unwrap() * u64::from(pbr_material::MAX_LIGHTS))
            .create();
        let world_bind_group = state.renderer.create_bind_group()
            .layout(state.world_bind_group_layout)
            .entry(0, world_uniform_buffer)
            .entry(1, lights_uniform_buffer)
            .create();

        self.started = Some(StartedEngine {
            window,

            state,
            application,

            world_uniform_buffer,
            lights_uniform_buffer,
            world_bind_group,

            last_update: None,
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let Some(started) = &mut self.started
        else { return };

        started.pre_frame();
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
