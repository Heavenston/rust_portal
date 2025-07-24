use std::{borrow::Cow, collections::HashMap};

use super::*;
use crate::handle_map;

#[derive(Debug)]
pub struct PipelineData {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) bind_group_layouts: Vec<BindGroupLayoutHandle>,
}
pub type PipelineHandle = handle_map::Handle<PipelineData>;

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

#[bon::builder(finish_fn = create)]
pub fn create_pipeline_builder(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    #[builder(field)]
    vertex_buffers: Vec<VertexBufferBindingData>,
    #[builder(into)]
    bind_group_layouts: Vec<BindGroupLayoutHandle>,
    shader_source: &str,
    #[builder(into, default)]
    shader_defs: HashMap<String, naga_oil::compose::ShaderDefValue>,
) -> PipelineHandle {
    let device = &renderer.device;

    let compiled_shader = crate::compile_shader::compile_shader(
        shader_source,
        shader_defs,
    ).expect("Could not compile shader");

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Naga(Cow::Owned(compiled_shader.into())),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: bind_group_layouts.iter()
            .map(|&handle| {
                &renderer.resources.bind_group_layouts.get(handle)
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

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: None,
            compilation_options: default(),
            buffers: &vertex_buffers,
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: None,
            compilation_options: default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: renderer.surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            ..default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
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

    renderer.resources.pipelines.insert(PipelineData { pipeline, bind_group_layouts })
}

impl<'f1, 'f2, S> CreatePipelineBuilderBuilder<'f1, 'f2, S>
    where S: create_pipeline_builder_builder::State,
{
    pub fn vertex_buffer(self) -> AddVertexBufferBuilder<'f1, 'f2, S> {
        add_vertex_buffer(self)
    }
}
