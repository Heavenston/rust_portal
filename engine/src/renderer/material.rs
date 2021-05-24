use super::*;
use smallvec::SmallVec;
use std::marker::PhantomData;

pub struct Material<'a> {
    pub shader: &'a Shader,
    pub render_pipeline: wgpu::RenderPipeline,
    pub bind_groups: SmallVec<[wgpu::BindGroup; 2]>,

    pub(crate) marker: PhantomData<()>,
}
