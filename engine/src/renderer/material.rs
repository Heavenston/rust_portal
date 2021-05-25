use super::*;
use smallvec::SmallVec;
use std::marker::PhantomData;

pub struct Material {
    pub shader: ShaderRef,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_groups: SmallVec<[wgpu::BindGroup; 2]>,

    pub(crate) marker: PhantomData<()>,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MaterialRef(pub(crate) usize);
