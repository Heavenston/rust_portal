mod material;
mod shader;
mod mesh;

pub use material::*;
pub use shader::*;
pub use mesh::*;
use smallvec::SmallVec;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::camera::Camera;
use std::convert::TryInto;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct RenderUniformBuffer {
    pub view_projection: [f32; 16],
}

pub struct Renderer {
    pub instance: wgpu::Instance,
    pub surface: wgpu::Surface,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub(crate) swap_chain_descriptor: wgpu::SwapChainDescriptor,
    pub(crate) swap_chain_format: wgpu::TextureFormat,
    pub(crate) swap_chain: wgpu::SwapChain,

    pub render_uniform_buffer: wgpu::Buffer,
    pub render_uniform_bind_group_layout: wgpu::BindGroupLayout,
    pub render_uniform_bind_group: wgpu::BindGroup,

    pub shaders: Vec<Shader>,
    pub materials: Vec<Material>,
    pub meshes: Vec<Mesh>,

    _unpin_marker: std::marker::PhantomPinned,
}

impl Renderer {
    pub async fn new(window: &winit::window::Window, width: u32, height: u32) -> Renderer {
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

        let render_uniform_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: std::mem::size_of::<RenderUniformBuffer>() as u64,
                usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
                mapped_at_creation: false
            }
        );
        let render_uniform_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::all(),
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None
            }]
        });
        let render_uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &render_uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(render_uniform_buffer.as_entire_buffer_binding()),
            }]
        });

        Self {
            instance,
            surface,
            adapter,
            device,
            queue,

            swap_chain_descriptor,
            swap_chain_format,
            swap_chain,

            render_uniform_buffer,
            render_uniform_bind_group_layout,
            render_uniform_bind_group,

            materials: Vec::new(),
            shaders: Vec::new(),
            meshes: Vec::new(),

            _unpin_marker: Default::default(),
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);
    }

    pub fn create_shader(
        &mut self, shader_module: &wgpu::ShaderModuleDescriptor,
        bind_group_layouts: &[&wgpu::BindGroupLayoutDescriptor],
        vertex_buffer_layout: &[wgpu::VertexBufferLayout<'static>],
    )-> ShaderRef
    {
        let i = self.shaders.len();
        let bind_group_layouts: SmallVec<_> = bind_group_layouts.iter()
            .map(|desc| self.device.create_bind_group_layout(desc))
            .collect();

        self.shaders.push(Shader {
            shader_module: self.device.create_shader_module(shader_module),
            render_pipeline_layout: self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: std::iter::once(&self.render_uniform_bind_group_layout)
                    .chain(bind_group_layouts.iter())
                    .collect::<Vec<_>>().as_slice(),
                push_constant_ranges: &[]
            }),
            bind_group_layouts,
            vertex_buffer_layouts: SmallVec::from(vertex_buffer_layout),

            marker: Default::default()
        });

        ShaderRef(i)
    }
    pub fn create_material(
        &mut self, shader_ref: ShaderRef,
        bind_groups: &[&[wgpu::BindGroupEntry]]
    ) -> MaterialRef
    {
        let i = self.materials.len();
        let shader = &self.shaders[shader_ref.0];
        self.materials.push(Material {
            render_pipeline: self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&shader.render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.shader_module,
                    entry_point: "vertex",
                    buffers: &shader.vertex_buffer_layouts
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
                }))
                .collect(),
            shader: shader_ref,

            marker: Default::default()
        });

        MaterialRef(i)
    }
    pub fn create_mesh<T: Pod>(&mut self, material: MaterialRef, indices: &[u32], vertices: &[T]) -> MeshRef {
        let i = self.meshes.len();
        self.meshes.push(Mesh {
            material,
            vertices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsage::VERTEX,
            }),
            vertices_size: vertices.len(),
            indices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsage::INDEX,
            }),
            indices_size: indices.len()
        });

        MeshRef(i)
    }

    pub fn render(&self, camera: impl Camera, meshes: &[MeshRef]) {
        self.queue.write_buffer(
            &self.render_uniform_buffer,
            0,
            bytemuck::bytes_of(&RenderUniformBuffer {
                view_projection: camera.get_vp_matrix().as_slice().try_into().unwrap()
            })
        );

        let frame = self.swap_chain
            .get_current_frame()
            .expect("Failed to acquire next swap chain texture")
            .output;
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut r_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });
            r_pass.set_bind_group(0, &self.render_uniform_bind_group, &[]);

            let mut last_material = None;
            for mesh_ref in meshes {
                let mesh = &self.meshes[mesh_ref.0];
                if last_material != Some(mesh.material) {
                    last_material = Some(mesh.material);
                    let material = &self.materials[mesh.material.0];
                    r_pass.set_pipeline(&material.render_pipeline);
                    for (bg, i) in material.bind_groups.iter().zip(1..) {
                        r_pass.set_bind_group(i, bg, &[]);
                    }
                }

                r_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                r_pass.set_vertex_buffer(0, mesh.vertices.slice(..));

                r_pass.draw_indexed(
                    0..mesh.indices_size as u32,
                    0,
                    0..1
                );
            }
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
