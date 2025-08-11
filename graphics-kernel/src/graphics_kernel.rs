mod render_operation;
pub use render_operation::*;
mod buffers;
pub use buffers::*;
mod textures;
pub use textures::*;
mod bind_group_layout;
pub use bind_group_layout::*;
mod bind_group;
pub use bind_group::*;
mod pipeline;
pub use pipeline::*;

use utils::{ handle_map::HandleMap, * };

use std::sync::Arc;

use derive_more::{ From, TryInto, IsVariant };
use glam::UVec2;

pub static DEPTH_TEXTURE_FORMAT: TextureFormat = TextureFormat::Depth24PlusStencil8;
pub static RENDER_TARGET_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

#[derive(Debug, Clone, Copy, From, TryInto, IsVariant)]
pub enum GraphicsResourceHandle {
    Buffer(BufferHandle),
    Texture(TextureHandle),
    BindGroupLayout(BindGroupLayoutHandle),
    BindGroup(BindGroupHandle),
    Pipeline(PipelineHandle),
}

#[derive(Debug, Default)]
pub(crate) struct GraphicsKernelResources {
    buffers: HandleMap<BufferData>,
    textures: HandleMap<TextureData>,
    bind_group_layouts: HandleMap<BindGroupLayoutData>,
    bind_groups: HandleMap<BindGroupData>,
    pipelines: HandleMap<PipelineData>,
}

#[derive(Debug)]
pub struct GraphicsKernel {
    #[expect(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    #[expect(dead_code)]
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    window: Arc<winit::window::Window>,
    size: UVec2,
    surface_format: wgpu::TextureFormat,

    resources: GraphicsKernelResources,
}

impl GraphicsKernel {
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

        let size = window.inner_size();

        let resources = GraphicsKernelResources::default();

        let this = Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            size: UVec2::new(size.width, size.height),
            window,
            surface_format,

            resources,
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
            width: self.size.x,
            height: self.size.y,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::Fifo,
        });
    }

    pub fn viewport_size(&self) -> UVec2 {
        self.size
    }

    pub fn aspect_ration(&self) -> f32 {
        let size = self.size.as_vec2();
        size.x / size.y
    }

    pub fn present_surface_format(&self) -> TextureFormat {
        self.surface_format.try_into().expect("Unsupported texture format?")
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = UVec2::new(size.width, size.height);
        self.configure_surface();
    }

    pub fn render(&'_ mut self) -> RenderOperation<'_> {
        RenderOperation::new(self)
    }

    pub fn create_buffer<'a, 'b>(&'a mut self) -> CreateBufferBuilderBuilder<'a, 'b> {
        create_buffer_builder(self)
    }

    pub fn delete_buffer(&mut self, handle: BufferHandle) {
        self.resources.buffers.remove(handle);
    }

    pub fn get_buffer_data(&self, handle: BufferHandle) -> Option<&BufferData> {
        self.resources.buffers.get(handle)
    }

    pub fn write_buffer(&self, buffer_handle: BufferHandle, offset: u64, data: &[u8]) {
        let buffer = &self.resources.buffers.get(buffer_handle)
            .expect("Invalid buffer handle given").buffer;
        self.queue.write_buffer(buffer, offset, data);
    }

    pub fn create_texture<'a>(&'a mut self) -> CreateTextureBuilderBuilder<'a> {
        create_texture_builder(self)
    }

    pub fn delete_texture(&mut self, handle: TextureHandle) {
        self.resources.textures.remove(handle);
    }

    pub fn get_texture_data(&self, handle: TextureHandle) -> Option<&TextureData> {
        self.resources.textures.get(handle)
    }

    pub fn write_texture(&self, texture_handle: TextureHandle, data: &[u8]) {
        let texture_data = &self.resources.textures.get(texture_handle)
            .expect("Invalid texture handle given");
        let texture = &texture_data.texture;
        let size = texture.size();
        let pixel_byte_size = texture_data.format.pixel_byte_size();

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

    pub fn create_bind_group_layout<'f2>(&'_ mut self) -> CreateBindGroupLayoutBuilder<'_, 'f2> {
        create_bind_group_layout(self)
    }

    pub fn delete_bind_group_layout(&mut self, handle: BindGroupLayoutHandle) {
        self.resources.bind_group_layouts.remove(handle);
    }

    pub fn get_bind_group_layout_data(&self, handle: BindGroupLayoutHandle) -> Option<&BindGroupLayoutData> {
        self.resources.bind_group_layouts.get(handle)
    }

    pub fn create_bind_group<'f2>(&'_ mut self) -> CreateBindGroupBuilder<'_, 'f2> {
        create_bind_group(self)
    }

    pub fn delete_bind_group(&mut self, handle: BindGroupHandle) {
        self.resources.bind_groups.remove(handle);
    }

    pub fn get_bind_group_data(&self, handle: BindGroupHandle) -> Option<&BindGroupData> {
        self.resources.bind_groups.get(handle)
    }

    pub fn create_pipeline<'f2, 'f3>(&'_ mut self) -> CreatePipelineBuilderBuilder<'_, 'f2, 'f3> {
        create_pipeline_builder(self)
    }

    pub fn delete_pipeline(&mut self, handle: PipelineHandle) {
        self.resources.pipelines.remove(handle);
    }

    pub fn get_pipeline_data(&self, handle: PipelineHandle) -> Option<&PipelineData> {
        self.resources.pipelines.get(handle)
    }
}
