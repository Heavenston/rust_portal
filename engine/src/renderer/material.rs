use std::marker::PhantomData;

use smallvec::SmallVec;

use super::*;

pub struct Material {
    pub shader: ShaderRef,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_groups: SmallVec<[wgpu::BindGroup; 2]>,

    pub(crate) marker: PhantomData<()>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MaterialRef(pub(crate) usize);
