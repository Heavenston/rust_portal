mod material;
mod shader;

pub use material::*;
pub use shader::*;
use smallvec::SmallVec;

pub struct Renderer<'a> {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub(crate) swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub(crate) swap_chain_format: wgpu::TextureFormat,
    pub(crate) swap_chain: wgpu::SwapChain,

    pub shaders: Vec<Shader>,
    pub materials: Vec<Material<'a>>,
}

impl<'a> Renderer<'a> {
    pub async fn new(window: &winit::window::Window, width: u32, height: u32) -> Renderer<'a> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                // Request an adapter which can render to our surface
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create the logical device and command queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: wgpu::Features::empty(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let swap_chain_format = adapter.get_swap_chain_preferred_format(&surface).unwrap();
        let swap_chain_descriptor = wgpu::SwapChainDescriptor {
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
            format: swap_chain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let swap_chain = device.create_swap_chain(&surface, &swap_chain_descriptor);

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            swap_chain_descriptor,
            swap_chain_format,
            swap_chain,

            materials: Vec::new(),
            shaders: Vec::new(),
        }
    }
    pub fn resize(&'a mut self, width: u32, height: u32) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn create_shader(
        &'a mut self, shader_module: &wgpu::ShaderModuleDescriptor,
        bind_group_layouts: &[&wgpu::BindGroupLayoutDescriptor]
    )-> &'a Shader
    {
        let i = self.shaders.len();
        let bind_group_layouts: SmallVec<_> = bind_group_layouts.iter()
            .map(|desc| self.device.create_bind_group_layout(desc))
            .collect();

        self.shaders.push(Shader {
            shader_module: self.device.create_shader_module(shader_module),
            render_pipeline_layout: self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.iter().collect::<Vec<_>>().as_slice(),
                push_constant_ranges: &[]
            }),
            bind_group_layouts,

            marker: Default::default()
        });
        &self.shaders[i]
    }
    pub fn create_material(
        &'a mut self, shader: &'a Shader,
        bind_groups: &[&[wgpu::BindGroupEntry]]
    ) -> &'a Material<'a>
    {
        let i = self.materials.len();
        self.materials.push(Material {
            render_pipeline: self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&shader.render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.shader_module,
                    entry_point: "vertex",
                    buffers: &[]
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.shader_module,
                    entry_point: "fragment",
                    targets: &[self.swap_chain_format.into()]
                }),
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
            }),
            bind_groups: bind_groups.iter()
                .enumerate()
                .map(|(i, entries)| self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &shader.bind_group_layouts[i],
                    entries
                })).collect(),
            shader,

            marker: Default::default()
        });
        &self.materials[i]
    }


}
