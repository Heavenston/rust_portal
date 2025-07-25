use crate::{ MaterialParameters, MaterialFactory, Renderer, EngineState, MaterialData };

use crevice::std140::AsStd140;
use glam::{Mat4, Vec3, Vec4};

static PBR_SHADER_SOURCE: &str = include_str!("../pbr_shader.wgsl");
pub static MAX_LIGHTS: u32 = 8;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Parameters {
    pub enable_diffuse_texture: bool,
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
    pub color: Vec3,
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
pub struct MaterialUniforms {
    pub base_color: Vec4,
    pub metallic: f32,
    pub roughness: f32,
}

pub(super) fn create_factory(
    engine_state: &mut EngineState,
) -> impl MaterialFactory<Parameters> {
    let camera_bind_group_layout = engine_state.world_bind_group_layout;
    let object_bind_group_layout = engine_state.object_bind_group_layout;

    move |renderer: &mut Renderer, parameters: &Parameters| {
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
        
        let pipeline = renderer.create_pipeline()
            .shader_source(PBR_SHADER_SOURCE)
            .bind_group_layouts([
                camera_bind_group_layout,
                object_bind_group_layout,
                material_bind_group_layout,
            ])
            .shader_defs([
                ("ENABLE_DIFFUSE_TEXTURE".to_string(), parameters.enable_diffuse_texture.into()),
                ("MAX_LIGHTS".to_string(), MAX_LIGHTS.into()),
            ])

            // Positions
            .vertex_buffer().shader_location(0).vec3().add()
            // Tex coords
            .vertex_buffer().shader_location(1).vec2().add()
            // Normals
            .vertex_buffer().shader_location(2).vec3().add()
        .create();

        MaterialData {
            pipeline,
        }
    }
}
