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

pub(crate) static DEPTH_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth24PlusStencil8;
pub static RENDER_TARGET_FORMAT: TextureFormat = TextureFormat::Rgba16Float;

#[derive(Debug, Default)]
pub struct RendererResources {
    buffers: HandleMap<BufferData>,
    textures: HandleMap<TextureData>,
    bind_group_layouts: HandleMap<BindGroupLayoutData>,
    bind_groups: HandleMap<BindGroupData>,
    pipelines: HandleMap<PipelineData>,
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
    surface_format: wgpu::TextureFormat,

    depth_buffer_handle: TextureHandle,
    depth_buffer: wgpu::Texture,
    render_target_handle: TextureHandle,
    /// Hdr render target
    render_target: wgpu::Texture,

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

        let size = window.inner_size();

        let depth_buffer = Self::create_depth_texture(&device, size);
        let render_target = Self::create_render_target_texture(&device, size);

        let mut resources = RendererResources::default();
        let depth_buffer_handle = resources.textures.insert(TextureData::from_wgpu(depth_buffer.clone()));
        let render_target_handle = resources.textures.insert(TextureData::from_wgpu(render_target.clone()));

        let this = Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            size,
            window,
            surface_format,

            depth_buffer_handle,
            depth_buffer,
            render_target_handle,
            render_target,

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

    fn create_render_target_texture(device: &wgpu::Device, size: winit::dpi::PhysicalSize<u32>) -> wgpu::Texture {
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
            format: RENDER_TARGET_FORMAT.into(),
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

    pub fn depth_buffer(&self) -> TextureHandle {
        self.depth_buffer_handle
    }

    pub fn render_target(&self) -> TextureHandle {
        self.render_target_handle
    }

    pub fn present_surface_format(&self) -> TextureFormat {
        self.surface_format.try_into().expect("Unsupported texture format?")
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.size = size;
        self.configure_surface();

        let new_depth_buffer = Self::create_depth_texture(&self.device, size);
        self.depth_buffer = new_depth_buffer.clone();
        self.resources.textures.replace(self.depth_buffer_handle, TextureData::from_wgpu(new_depth_buffer));

        let new_render_target = Self::create_render_target_texture(&self.device, size);
        self.render_target = new_render_target.clone();
        self.resources.textures.replace(self.render_target_handle, TextureData::from_wgpu(new_render_target));
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
        let texture_data = &self.resources.textures.get(texture_handle)
            .expect("Invalid texture handle given");
        let texture = &texture_data.texture;
        let size = texture.size();
        let format = texture_data.format.expect("Writing to this texture is not supported");

        let pixel_byte_size = format.pixel_byte_size();

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

    pub fn create_bind_group_layout(&'_ mut self) -> CreateBindGroupLayoutBuilder<'_> {
        create_bind_group_layout(self)
    }

    pub fn delete_bind_group_layout(&mut self, handle: BindGroupLayoutHandle) {
        self.resources.bind_group_layouts.remove(handle);
    }

    pub fn create_bind_group(&'_ mut self) -> CreateBindGroupBuilder<'_> {
        create_bind_group(self)
    }

    pub fn delete_bind_group(&mut self, handle: BindGroupHandle) {
        self.resources.bind_groups.remove(handle);
    }

    pub fn create_pipeline<'a, 'f2>(&'a mut self) -> CreatePipelineBuilderBuilder<'a, 'f2> {
        create_pipeline_builder(self)
    }

    pub fn get_pipeline_bind_group_layouts(&self, handle: PipelineHandle) -> &[BindGroupLayoutHandle] {
        &self.resources.pipelines.get(handle)
            .expect("Invalind pipeline handle given")
            .bind_group_layouts
    }

    pub fn delete_pipeline(&mut self, handle: PipelineHandle) {
        self.resources.pipelines.remove(handle);
    }
}
