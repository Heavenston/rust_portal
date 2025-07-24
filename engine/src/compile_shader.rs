use crate::utils::default;

use naga_oil::compose::{
    ComposableModuleDescriptor, Composer, NagaModuleDescriptor, ShaderDefValue,
};
use std::collections::HashMap;

pub fn compile_shader(
    source: &str,
    shader_defs: HashMap<String, ShaderDefValue>
) -> Result<naga::Module, naga_oil::compose::ComposerError> {
    let mut composer = Composer::default();

    composer.add_composable_module(ComposableModuleDescriptor {
        source,
        file_path: "inline_shader.wgsl",
        as_name: Some("inline_shader".to_string()),
        ..default()
    })?;

    let module = composer.make_naga_module(NagaModuleDescriptor {
        source,
        file_path: "inline_shader.wgsl",
        shader_defs,
        ..default()
    })?;

    Ok(module)
}
