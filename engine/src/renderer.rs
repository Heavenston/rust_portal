mod render_operation;
pub use render_operation::*;
mod buffers;
pub use buffers::*;
mod object_bind_group;
pub use object_bind_group::*;
mod camera_bind_group;
pub use camera_bind_group::*;

use std::sync::Arc;

use crate::{ handle_map::HandleMap, utils::* };

static DEFAULT_3D_MATERIAL_SHADER: &str = include_str!("default_3d_material.wgsl");

#[derive(Debug, Default)]
pub struct RendererResources {
    buffers: HandleMap<BufferData>,
    object_bind_groups: HandleMap<ObjectBindGroupData>,
    camera_bind_groups: HandleMap<CameraBindGroupData>,
}

#[derive(Debug)]
pub struct Renderer {
    #[expect(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    #[expect(dead_code)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    window: Arc<winit::window::Window>,
    size: winit::dpi::PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,

    camera_bind_group_layout: wgpu::BindGroupLayout,
    object_bind_group_layout: wgpu::BindGroupLayout,
    default_3d_render_pipeline: wgpu::RenderPipeline,

    resources: RendererResources,
}

impl Renderer {
    async fn new_async(window: Arc<winit::window::Window>) -> Self {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                    .using_resolution(adapter.limits()),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .expect("Failed to create device");

        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(DEFAULT_3D_MATERIAL_SHADER),
            source: wgpu::ShaderSource::Wgsl(DEFAULT_3D_MATERIAL_SHADER.into()),
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
            });

        let object_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("object_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                }],
            });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &object_bind_group_layout],
            push_constant_ranges: &[],
        });

        let default_3d_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: None,
                compilation_options: default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: size_of::<glam::Vec3>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0, shader_location: 0,
                    }],
                }, wgpu::VertexBufferLayout {
                    array_stride: size_of::<glam::Vec2>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0, shader_location: 1,
                    }],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: None,
                compilation_options: default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                ..default()
            },
            depth_stencil: None,
            multisample: default(),
            multiview: None,
            cache: None,
        });

        let this = Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            size: window.inner_size(),
            window,
            surface_format,

            object_bind_group_layout,
            camera_bind_group_layout,
            default_3d_render_pipeline,

            resources: default(),
        };
        this.configure_surface();
        this
    }

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        pollster::block_on(Self::new_async(window))
    }

    fn configure_surface(&self) {
        self.surface.configure(&self.device, &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
        });
    }

    pub fn viewport_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }

    pub fn aspect_ration(&self) -> f32 {
        let w = self.size.width as f32;
        let h = self.size.height as f32;
        w / h
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        self.configure_surface();
    }

    pub fn render(&'_ mut self) -> RenderOperation<'_> {
        RenderOperation::new(self)
    }

    pub fn write_buffer(&self, buffer_handle: BufferHandle, offset: u64, data: &[u8]) {
        let buffer = &self.resources.buffers.get(buffer_handle)
            .expect("Invalid buffer handle given").buffer;
        self.queue.write_buffer(buffer, offset, data);
    }

    pub fn create_buffer<'a, 'b>(&'a mut self) -> CreateBufferBuilderBuilder<'a, 'b> {
        create_buffer_builder(self)
    }

    pub fn create_object_bind_group(&'_ mut self) -> CreateObjectBindGroupBuilder<'_> {
        create_object_bind_group(self)
    }

    pub fn create_camera_bind_group(&'_ mut self) -> CreateCameraBindGroupBuilder<'_> {
        create_camera_bind_group(self)
    }
}
