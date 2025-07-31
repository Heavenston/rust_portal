use super::*;
use utils::handle_map;

use std::{borrow::Cow, collections::HashMap};
use derive_more::From;

#[derive(Debug)]
pub struct PipelineData {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layouts: Vec<BindGroupLayoutHandle>,
}
pub type PipelineHandle = handle_map::Handle<PipelineData>;

#[derive(Debug, Clone, Copy, From)]
pub enum ShaderValueDef {
    Bool(bool),
    Int(i64),
}

impl ShaderValueDef {
    pub fn to_preprocessor_value(self) -> shader_preprocessor::Value {
        use shader_preprocessor::Value as PrepValue;
        match self {
            ShaderValueDef::Bool(b) => PrepValue::Bool(b),
            ShaderValueDef::Int(val) => PrepValue::Int(val),
        }
    }
}

impl From<i32> for ShaderValueDef {
    fn from(value: i32) -> Self {
        Self::from(i64::from(value))
    }
}

impl From<u32> for ShaderValueDef {
    fn from(value: u32) -> Self {
        Self::from(i64::from(value))
    }
}

struct VertexBufferBindingData {
    stride: u64,
    attribute: [wgpu::VertexAttribute; 1],
}

#[bon::builder(finish_fn = add)]
pub fn add_vertex_buffer<'f1, 'f2, PS>(
    #[builder(start_fn)]
    mut parent: CreatePipelineBuilderBuilder<'f1, 'f2, PS>,
    shader_location: u32,
    #[builder(setters(vis = "", name = "priv_format"))]
    format: (u64, wgpu::VertexFormat),
) -> CreatePipelineBuilderBuilder<'f1, 'f2, PS>
where PS: create_pipeline_builder_builder::State,
{
    let (stride, format) = format;

    parent.vertex_buffers.push(VertexBufferBindingData {
        stride,
        attribute: [wgpu::VertexAttribute {
            format, offset: 0, shader_location,
        }],
    });
    parent
}

impl<'f1, 'f2, PS, S> AddVertexBufferBuilder<'f1, 'f2, PS, S>
    where PS: create_pipeline_builder_builder::State,
          S: add_vertex_buffer_builder::State,
          S::Format: add_vertex_buffer_builder::IsUnset,
{
    pub fn vec2(self) -> AddVertexBufferBuilder<'f1, 'f2, PS, add_vertex_buffer_builder::SetFormat<S>> {
        self.priv_format((
            size_of::<glam::Vec2>() as u64,
            wgpu::VertexFormat::Float32x2,
        ))
    }

    pub fn vec3(self) -> AddVertexBufferBuilder<'f1, 'f2, PS, add_vertex_buffer_builder::SetFormat<S>> {
        self.priv_format((
            size_of::<glam::Vec3>() as u64,
            wgpu::VertexFormat::Float32x3,
        ))
    }
}

#[bon::builder(finish_fn = add)]
pub fn add_color_target<'f1, 'f2, PS>(
    #[builder(start_fn)]
    mut parent: CreatePipelineBuilderBuilder<'f1, 'f2, PS>,
    format: TextureFormat,
) -> CreatePipelineBuilderBuilder<'f1, 'f2, PS>
where PS: create_pipeline_builder_builder::State,
{
    parent.color_targets.push(Some(wgpu::ColorTargetState {
        format: format.into(),
        // blend: Some(wgpu::BlendState::REPLACE),
        blend: None,
        write_mask: wgpu::ColorWrites::ALL,
    }));
    parent
}

#[bon::builder(finish_fn = create)]
pub fn create_pipeline_builder(
    #[builder(start_fn)]
    kernel: &mut GraphicsKernel,
    #[builder(field)]
    vertex_buffers: Vec<VertexBufferBindingData>,
    #[builder(field)]
    color_targets: Vec<Option<wgpu::ColorTargetState>>,
    #[builder(default = false)]
    depth_buffer: bool,
    #[builder(into)]
    bind_group_layouts: Vec<BindGroupLayoutHandle>,
    shader_source: &str,
    #[builder(into, default)]
    shader_defs: HashMap<String, ShaderValueDef>,
    #[builder(into, default)]
    vertex_pipleline_overrides: HashMap<String, f64>,
    #[builder(into, default)]
    fragment_pipleline_overrides: HashMap<String, f64>,
) -> PipelineHandle {
    let device = &kernel.device;

    let compiled_shader = crate::compile_shader::compile_shader(
        shader_source,
        shader_defs.into_iter()
            .map(|(k, v)| (k, v.to_preprocessor_value()))
            .collect(),
    );

    let compiled_shader = match compiled_shader {
        Ok(c) => c,
        Err(e) => panic!("Could not preprocess shader: {e}"),
    };

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Owned(compiled_shader)),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: bind_group_layouts.iter()
            .map(|&handle| {
                &kernel.resources.bind_group_layouts.get(handle)
                    .expect("Invalid bind group layout handle given")
                    .bind_group_layout
            })
            .collect::<Vec<_>>()
            .as_slice(),
        push_constant_ranges: &[],
    });

    let vertex_buffers = vertex_buffers.iter()
        .map(|data| wgpu::VertexBufferLayout {
            array_stride: data.stride,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &data.attribute,
        })
        .collect::<Vec<_>>();

    let vertex_overrides = vertex_pipleline_overrides.iter()
        .map(|(a, b)| (a.as_str(), *b))
        .collect::<Vec::<_>>();
    let fragment_overrides = fragment_pipleline_overrides.iter()
        .map(|(a, b)| (a.as_str(), *b))
        .collect::<Vec::<_>>();

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &vertex_overrides,
                ..default()
            },
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: None,
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &fragment_overrides,
                ..default()
            },
            targets: &color_targets,
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            ..default()
        },
        depth_stencil: depth_buffer.then_some(wgpu::DepthStencilState {
            format: DEPTH_TEXTURE_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: default(),
            bias: default(),
        }),
        multisample: default(),
        multiview: None,
        cache: None,
    });

    kernel.resources.pipelines.insert(PipelineData { pipeline, bind_group_layouts })
}

impl<'f1, 'f2, S> CreatePipelineBuilderBuilder<'f1, 'f2, S>
    where S: create_pipeline_builder_builder::State,
{
    pub fn vertex_buffer(self) -> AddVertexBufferBuilder<'f1, 'f2, S> {
        add_vertex_buffer(self)
    }

    pub fn color_target(self) -> AddColorTargetBuilder<'f1, 'f2, S> {
        add_color_target(self)
    }
}

impl<'f1, 'f2, S> CreatePipelineBuilderBuilder<'f1, 'f2, S>
    where S: create_pipeline_builder_builder::State,
          S::FragmentPiplelineOverrides: create_pipeline_builder_builder::IsUnset,
          S::VertexPiplelineOverrides: create_pipeline_builder_builder::IsUnset,
{
    pub fn pipeline_overrides(self, vals: impl Into<HashMap<String, f64>>) -> CreatePipelineBuilderBuilder<'f1, 'f2, create_pipeline_builder_builder::SetVertexPiplelineOverrides<create_pipeline_builder_builder::SetFragmentPiplelineOverrides<S>>> {
        let vals = vals.into();
        self.fragment_pipleline_overrides(vals.clone())
            .vertex_pipleline_overrides(vals.clone())
    }
}
