use super::Renderer;

use crate::{
    hdr_tonemapper_material, pbr_material, Camera, EngineState, StaticMeshData
};

use pgk::{
    BindGroupHandle, BufferHandle, LoadOperations, LoadStoreOperations, StoreOperations, TextureFormat, TextureHandle
};
use utils::default;

use std::{ iter::empty };
use glam::UVec2;
use crevice::std140::AsStd140 as _;

pub static DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
pub static RENDER_TARGET_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

pub struct ForwardRenderer {
    depth_buffer_handle: TextureHandle,
    render_target_handle: TextureHandle,

    tonemap_bind_group: BindGroupHandle,

    world_uniform_buffer: BufferHandle,
    lights_uniform_buffer: BufferHandle,
    world_bind_group: BindGroupHandle,
}

impl ForwardRenderer {
    pub fn new(state: &mut EngineState) -> Self {
        let world_uniform_buffer = state.kernel.create_buffer()
            .size(pbr_material::WorldUniforms::std140_size_static().try_into().unwrap())
            .create();
        let lights_uniform_buffer = state.kernel.create_buffer()
            .size(u64::try_from(pbr_material::Light::std140_size_static()).unwrap() * u64::from(pbr_material::MAX_LIGHTS))
            .create();
        let world_bind_group = state.kernel.create_bind_group()
            .layout(state.resources.world_bind_group_layout)
            .entry(0, world_uniform_buffer)
            .entry(1, lights_uniform_buffer)
            .create();

        let mut this = Self {
            depth_buffer_handle: default(),
            render_target_handle: default(),

            tonemap_bind_group: default(),

            world_uniform_buffer,
            lights_uniform_buffer,
            world_bind_group,
        };
        this.resize(state, state.kernel.viewport_size());
        this
    }

    fn write_world_buffer(&mut self, state: &mut EngineState, camera: Camera) {
        let mut lights: Vec<pbr_material::Std140Light> = empty()
            .chain(
                state.directional_lights_map.iter().map(|(_, b)| b.directional_light)
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
                state.spot_lights_map.iter().map(|(_, b)| b.spot_light)
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
        state.kernel.write_buffer(
            self.world_uniform_buffer, 0,
            bytemuck::bytes_of(&uniforms.as_std140()),
        );
        state.kernel.write_buffer(
            self.lights_uniform_buffer, 0,
            bytemuck::cast_slice(&lights),
        );

        let tonemap_material_handle = state.materials.get_handle(&mut state.kernel, &hdr_tonemapper_material::Parameters);
        let tonemap_pipeline = state.materials.get(tonemap_material_handle).pipeline;
        let tonemap_bind_group_layout = state.kernel.get_pipeline_data(tonemap_pipeline).unwrap().bind_group_layouts[0];

        self.tonemap_bind_group = state.kernel.create_bind_group()
            .layout(tonemap_bind_group_layout)
            .entry(0, self.render_target_handle)
            .sampler(1)
            .create();
    }
}

impl Renderer for ForwardRenderer {
    fn resize(&mut self, state: &mut EngineState, size: UVec2) {
        let kernel = &mut state.kernel;

        kernel.delete_texture(self.depth_buffer_handle);
        self.depth_buffer_handle = kernel.create_texture()
            .usages(pgk::TextureUsages {
                render_attachment: true,
                texture_binding: true,
                ..default()
            })
            .size(size)
            .format(DEPTH_TEXTURE_FORMAT)
            .create();

        kernel.delete_texture(self.render_target_handle);
        self.render_target_handle = kernel.create_texture()
            .usages(pgk::TextureUsages {
                render_attachment: true,
                texture_binding: true,
                ..default()
            })
            .size(size)
            .format(RENDER_TARGET_FORMAT)
            .create();
    }

    fn render(&mut self, state: &mut EngineState) {
        // 
        // Write into world uniform buffer
        // 
        let Some(camera) = state.camera
        else { eprintln!("NO CAMERA"); return; };

        self.write_world_buffer(state, camera);

        let tonemap_material_handle = state.materials.get_handle(&mut state.kernel, &hdr_tonemapper_material::Parameters);
        let tonemap_pipeline = state.materials.get(tonemap_material_handle).pipeline;
        let tonemap_bind_group_layout = state.kernel.get_pipeline_data(tonemap_pipeline).unwrap().bind_group_layouts[0];

        let tonemap_bind_group = state.kernel.create_bind_group()
            .layout(tonemap_bind_group_layout)
            .entry(0, self.render_target_handle)
            .sampler(1)
            .create();

        let mut render = state.kernel.render();
        let present_texture_handle = render.using_present_texture();
        let render_target_handle = render.using_texture_view(self.render_target_handle)
            .expect("valid handle");
        let depth_buffer_handle = render.using_texture_view(self.depth_buffer_handle)
            .expect("valid handle");

        let mut draw_call_number = 0;

        // Object rendering pass
        {
            let mut render_pass = render.render_pass()
                .color_attachment()
                    .texture_view_handle(render_target_handle)
                    .load_op(LoadOperations::Clear(camera.clear_color))
                    .store_op(StoreOperations::Store)
                    .finish()
                .depth_stencil_attachment()
                    .texture_view_handle(depth_buffer_handle)
                    .depth_ops(LoadStoreOperations {
                        load: LoadOperations::Clear(1.),
                        store: StoreOperations::Discard,
                    })
                    .finish()
                .build();

            for (_, StaticMeshData { mesh, object_bind_group }) in state.resources.static_meshes_map.iter() {
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
                render_pass.draw_indexed(0..mesh.vertex_count, 0, 0..1);

                draw_call_number += 1;
            }

            render_pass.finish();
        }

        // tonemapping pass
        {
            let mut render_pass = render.render_pass()
                .color_attachment()
                    .texture_view_handle(present_texture_handle)
                    .finish()
                .build();
            render_pass.set_pipeline(tonemap_pipeline);
            render_pass.set_bind_group(0, tonemap_bind_group);
            render_pass.draw(0..3, 0..1);
            render_pass.finish();
        }

        println!("{draw_call_number} draw calls");

        render.finish();

        state.kernel.delete_bind_group(tonemap_bind_group);
    }
}


