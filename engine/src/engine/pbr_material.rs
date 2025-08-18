use crate::{
    builtin_shaders, embedded_shader_factory_helper,
    EngineState, MaterialData, MaterialFactory, MaterialParameters,
};
use pgk::{
    color::{ LinearRgb, LinearRgba },
    GraphicsKernel, RENDER_TARGET_FORMAT,
};

use std::time::SystemTime;

use crevice::std140::AsStd140;
use glam::{ Mat4, Vec3 };

pub static MAX_LIGHTS: u32 = 8;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Parameters {
    pub enable_base_color_texture: bool,
}
impl MaterialParameters for Parameters { }

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LightKind {
    Directional = 0,
    Spot = 1,
}

impl AsStd140 for LightKind {
    type Output = u32;

    fn as_std140(&self) -> Self::Output {
        *self as u8 as u32
    }

    fn from_std140(val: Self::Output) -> Self {
        match val {
            0 => Self::Directional,
            1 => Self::Spot,
            _ => panic!("Invalid light kind value"),
        }
    }
}

#[derive(AsStd140, Debug, Clone, Copy)]
pub struct Light {
    pub kind: LightKind,
    pub position: Vec3,
    pub direction: Vec3,
    pub color: LinearRgb,
    pub intensity: f32,
    pub inner_cone_angle: f32,
    pub outer_cone_angle: f32,
}

#[derive(AsStd140, Debug, Clone, Copy)]
pub struct WorldUniforms {
    pub view_projection: Mat4,
    pub camera_world_pos: Vec3,
    pub light_count: u32,
}

#[derive(AsStd140, Debug, Clone, Copy)]
pub struct ObjectUniforms {
    pub model: Mat4,
}

#[derive(AsStd140, Debug, Clone, Copy)]
pub struct MaterialUniforms {
    pub base_color: LinearRgba,
    pub metallic: f32,
    pub roughness: f32,
}

pub(super) fn create_factory(
    engine_state: &mut EngineState,
) -> impl MaterialFactory<Parameters> {
    let camera_bind_group_layout = engine_state.resources.world_bind_group_layout;
    let object_bind_group_layout = engine_state.resources.object_bind_group_layout;

    let factory = move |kernel: &mut GraphicsKernel, shader_source: &str, parameters: &Parameters| -> MaterialData {
        let mut material_bind_group_layout = kernel.create_bind_group_layout()
            .label(format!("PBR Material bind group layout ({parameters:?})"))
            .entry().binding(0).uniform_buffer().add()
        ;

        if parameters.enable_base_color_texture {
            material_bind_group_layout = material_bind_group_layout
                .entry().binding(1).texture().add()
                .entry().binding(2).sampler().add()
            ;
        }

        let material_bind_group_layout = material_bind_group_layout.create();
    
        let pipeline = kernel.create_pipeline()
            .label(format!("PBR Material pipeline ({parameters:?})"))
            .shader_source(shader_source)
            .bind_group_layouts([
                camera_bind_group_layout,
                object_bind_group_layout,
                material_bind_group_layout,
            ])
            .shader_defs([
                ("ENABLE_BASE_COLOR_TEXTURE".to_string(), parameters.enable_base_color_texture.into()),
                ("MAX_LIGHTS".to_string(), MAX_LIGHTS.into()),
            ])
            
            .color_target().format(RENDER_TARGET_FORMAT).add()
            .depth_buffer(true)

            // Positions
            .vertex_buffer().shader_location(0).vec3().add()
            // Tex coords
            .vertex_buffer().shader_location(1).vec2().add()
            // Normals
            .vertex_buffer().shader_location(2).vec3().add()
        .create();

        MaterialData {
            created_at: SystemTime::now(),
            pipeline,
        }
    };

    embedded_shader_factory_helper(
        factory,
        builtin_shaders::BuiltinShaders,
        "pbr_shader.wgsl"
    )
}
