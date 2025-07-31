use crate::{
    builtin_shaders, embedded_shader_factory_helper,
    EngineState, MaterialData, MaterialFactory, MaterialParameters
};
use pgk::GraphicsKernel;

use std::time::{ SystemTime };

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct Parameters;
impl MaterialParameters for Parameters { }

pub(super) fn create_factory(
    _engine_state: &mut EngineState,
) -> impl MaterialFactory<Parameters> {
    let factory = |kernel: &mut GraphicsKernel, shader_source: &str, Parameters: &Parameters| {
        let bind_group_layout = kernel.create_bind_group_layout()
            .entry().binding(0).texture().add()
            .entry().binding(1).sampler().add()
            .create()
        ;

        let present_surface_format = kernel.present_surface_format();
    
        let pipeline = kernel.create_pipeline()
            .shader_source(shader_source)
            .color_target().format(present_surface_format).add()
            .bind_group_layouts([ bind_group_layout ])
        .create();

        MaterialData {
            created_at: SystemTime::now(),
            pipeline,
        }
    };

    embedded_shader_factory_helper(
        factory,
        builtin_shaders::BuiltinShaders,
        "hdr_tonemapper.wgsl"
    )
}

