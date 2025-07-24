mod render_operation;
pub use render_operation::*;
mod buffers;
pub use buffers::*;
mod textures;
pub use textures::*;
mod object_bind_group;
pub use object_bind_group::*;
mod camera_bind_group;
pub use camera_bind_group::*;

use std::sync::Arc;

use crate::{ handle_map::HandleMap, utils::* };

static PBR_SHADER_SOURCE: &str = include_str!("pbr_shader.wgsl");
pub(crate) static DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;

#[derive(Debug, Default)]
pub struct RendererResources {
    buffers: HandleMap<BufferData>,
    textures: HandleMap<TextureData>,
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
    pub(crate) device: wgpu::Device,
    pub(crate) queue: wgpu::Queue,

    window: Arc<winit::window::Window>,
    size: winit::dpi::PhysicalSize<u32>,
    pub(crate) surface_format: wgpu::TextureFormat,

    camera_bind_group_layout: wgpu::BindGroupLayout,
    object_bind_group_layout: wgpu::BindGroupLayout,
    default_3d_render_pipeline: wgpu::RenderPipeline,

    depth_buffer_handle: TextureHandle,
    depth_buffer: wgpu::Texture,

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
            label: Some("pbr shader source"),
            source: wgpu::ShaderSource::Wgsl(PBR_SHADER_SOURCE.into()),
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
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &[],
                    ..default()
                },
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

        let size = window.inner_size();
        let depth_buffer = Self::create_depth_texture(&device, size);

        let mut resources = RendererResources::default();
        let depth_buffer_handle = resources.textures.insert(TextureData::from_wgpu(depth_buffer.clone()));

        let this = Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            size,
            window,
            surface_format,

            object_bind_group_layout,
            camera_bind_group_layout,
            default_3d_render_pipeline,

            depth_buffer_handle,
            depth_buffer,

            resources,
        };
        this.configure_surface();
        this
    }

    pub fn new(window: Arc<winit::window::Window>) -> Self {
        pollster::block_on(Self::new_async(window))
    }

    fn create_depth_texture(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.width.max(1),
                height: size.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_TEXTURE_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
            ,
            view_formats: &[],
        })
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

        let new_depth_buffer = Self::create_depth_texture(&self.device, size);
        self.depth_buffer = new_depth_buffer.clone();
        self.resources.textures.replace(self.depth_buffer_handle, TextureData::from_wgpu(new_depth_buffer));
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

    pub fn delete_buffer(&mut self, handle: BufferHandle) {
        self.resources.buffers.remove(handle);
    }

    pub fn write_texture(&self, texture_handle: TextureHandle, data: &[u8]) {
        let texture = &self.resources.textures.get(texture_handle)
            .expect("Invalid texture handle given").texture;

        let size = texture.size();
        let format = texture.format();

        let Some(pixel_byte_size) = format.target_pixel_byte_cost()
        else { panic!("Writing to a texture of format {format:?} isn't supported") };

        self.queue.write_texture(wgpu::TexelCopyTextureInfoBase {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        }, data, wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(pixel_byte_size * size.width),
            rows_per_image: Some(size.height),
        }, wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: size.depth_or_array_layers,
        });
    }

    pub fn create_texture<'a>(&'a mut self) -> CreateTextureBuilderBuilder<'a> {
        create_texture_builder(self)
    }

    pub fn delete_texture(&mut self, handle: TextureHandle) {
        self.resources.textures.remove(handle);
    }

    pub fn create_object_bind_group(&'_ mut self) -> CreateObjectBindGroupBuilder<'_> {
        create_object_bind_group(self)
    }

    pub fn delete_object_bind_group(&mut self, handle: ObjectBindGroupHandle) {
        self.resources.object_bind_groups.remove(handle);
    }

    pub fn create_camera_bind_group(&'_ mut self) -> CreateCameraBindGroupBuilder<'_> {
        create_camera_bind_group(self)
    }

    pub fn delete_camera_bind_group(&mut self, handle: CameraBindGroupHandle) {
        self.resources.camera_bind_groups.remove(handle);
    }
}
