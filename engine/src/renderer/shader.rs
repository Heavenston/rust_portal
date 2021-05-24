use std::marker::PhantomData;
use smallvec::SmallVec;

pub struct Shader {
    pub shader_module: wgpu::ShaderModule,
    pub render_pipeline_layout: wgpu::PipelineLayout,
    pub bind_group_layouts: SmallVec<[wgpu::BindGroupLayout; 2]>,

    pub(crate) marker: PhantomData<()>,
}
