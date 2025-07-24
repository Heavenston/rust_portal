use crate::{ * };

use bytemuck::NoUninit;
use glam::Vec4;

static PBR_SHADER_SOURCE: &str = include_str!("../pbr_shader.wgsl");

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct PbrMaterialParameters {
    pub enable_diffuse_texture: bool,
}
impl MaterialParameters for PbrMaterialParameters { }

#[derive(Debug, Clone, Copy, NoUninit)]
#[repr(C)]
pub struct PbrMaterialUniforms {
    pub base_color: Vec4,
}

pub(super) fn create_pbr_material_factory(
    engine_state: &mut EngineState,
) -> impl MaterialFactory<PbrMaterialParameters> {
    let camera_bind_group_layout = engine_state.camera_bind_group_layout;
    let object_bind_group_layout = engine_state.object_bind_group_layout;

    move |renderer: &mut Renderer, parameters| {
        let mut material_bind_group_layout = renderer.create_bind_group_layout()
            .entry().binding(0).uniform_buffer().add()
        ;

        if parameters.enable_diffuse_texture {
            material_bind_group_layout = material_bind_group_layout
                .entry().binding(1).texture().add();
            material_bind_group_layout = material_bind_group_layout
                .entry().binding(2).sampler().add();
        }

        let material_bind_group_layout = material_bind_group_layout.create();

        let shader_constants = [
            ("ENABLE_DIFFUSE_TEXTURE", if parameters.enable_diffuse_texture { 1. } else { 0. })
        ];
        
        let pipeline = renderer.create_pipeline()
            .shader_source(PBR_SHADER_SOURCE)
            .bind_group_layouts([
                camera_bind_group_layout,
                object_bind_group_layout,
                material_bind_group_layout,
            ])

            .vertex_shader_constants(&shader_constants)
            // Positions
            .vertex_buffer().shader_location(0).vec3().add()
            // Tex coords
            .vertex_buffer().shader_location(1).vec2().add()

            .fragment_shader_constants(&shader_constants)
        .create();

        MaterialData {
            pipeline,
        }
    }
}
