use crate::{
    builtin_shaders,
    color::{ LinearRgb, LinearRgba },
    BindGroupLayoutHandle, EngineState, MaterialData, MaterialFactory,
    MaterialParameters, Renderer,
};

use std::time::{ Duration, SystemTime };

use crevice::std140::AsStd140;
use glam::{ Mat4, Vec3 };

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
pub struct MaterialUniforms {
    pub base_color: LinearRgba,
    pub metallic: f32,
    pub roughness: f32,
}

pub(super) fn create_factory(
    engine_state: &mut EngineState,
) -> impl MaterialFactory<Parameters> {
    struct Factory {
        camera_bind_group_layout: BindGroupLayoutHandle,
        object_bind_group_layout: BindGroupLayoutHandle,
    }

    impl MaterialFactory<Parameters> for Factory {
        fn create(&mut self, renderer: &mut Renderer, parameters: &Parameters) -> MaterialData {
            let shader_source_file = builtin_shaders::BuiltinShaders::get("pbr_shader.wgsl")
                .expect("Could not find 'pbr_shader.wgsl'");
            let shader_source = str::from_utf8(&shader_source_file.data)
                .expect("Invalid utf8 in 'pbr_shader.wgsl'");

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
                .shader_source(shader_source)
                .bind_group_layouts([
                    self.camera_bind_group_layout,
                    self.object_bind_group_layout,
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
                created_at: SystemTime::now(),
                pipeline,
            }
        }

        fn is_outdated(&mut self, data: &MaterialData) -> bool {
            let shader_source_file = builtin_shaders::BuiltinShaders::get("pbr_shader.wgsl")
                .expect("Could not find 'pbr_shader.wgsl'");
            let Some(last_modified) = shader_source_file.metadata.last_modified()
            else {
                // FIXME: Change to warn! or something
                println!("Could not get last modified on pbr_shader.wgsl shader");
                return false;
            };
            let last_modified_time =
                SystemTime::UNIX_EPOCH + Duration::from_secs(last_modified);

            data.created_at < last_modified_time
        }
    }

    Factory {
        camera_bind_group_layout: engine_state.world_bind_group_layout,
        object_bind_group_layout: engine_state.object_bind_group_layout,
    }
}
