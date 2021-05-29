mod material;
mod shader;
mod mesh;

pub use material::*;
pub use shader::*;
pub use mesh::*;
use smallvec::SmallVec;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::camera::CameraComponent;
use legion::query::{IntoQuery, DefaultFilter};
use crate::transform::TransformComponent;
use memoffset::offset_of;
use legion::Query;
use legion::internals::query::view::IntoView;
use std::sync::RwLock;

type LegionQueryOf<A> = Query<A, <<A as IntoView>::View as DefaultFilter>::Filter>;

#[derive(Copy, Clone, Zeroable, Pod)]
#[repr(C)]
pub struct RenderUniformBuffer {
    pub view_projection: [f32; 16],
    pub model_matrix: [f32; 16],
}

pub struct Renderer<'a> {
    surface: wgpu::Surface,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    swap_chain_descriptor: wgpu::SwapChainDescriptor,
    swap_chain_format: wgpu::TextureFormat,
    swap_chain: wgpu::SwapChain,

    depth_buffer_texture: wgpu::Texture,
    depth_buffer_texture_view: wgpu::TextureView,

    render_uniform_buffer: wgpu::Buffer,
    render_uniform_bind_group_layout: wgpu::BindGroupLayout,
    render_uniform_bind_group: wgpu::BindGroup,

    shaders: Vec<Shader>,
    materials: Vec<Material>,
    meshes: Vec<Mesh>,

    mesh_query: RwLock<LegionQueryOf<(&'a TransformComponent, &'a MeshComponent)>>,
}

impl<'a> Renderer<'a> {
    fn create_depth_texture(device: &wgpu::Device, sc_desc: &wgpu::SwapChainDescriptor)
        -> (wgpu::Texture, wgpu::TextureView) {
        let size = wgpu::Extent3d {
            width: sc_desc.width,
            height: sc_desc.height,
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: None,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT
                 | wgpu::TextureUsage::COPY_DST,
        };
        let texture = device.create_texture(&desc);

        let view = texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

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

        let (depth_buffer_texture, depth_buffer_texture_view) = Self::create_depth_texture(
            &device,
            &swap_chain_descriptor
        );

        Self {
            surface,
            device,
            queue,

            swap_chain_descriptor,
            swap_chain_format,
            swap_chain,

            render_uniform_buffer,
            render_uniform_bind_group_layout,
            render_uniform_bind_group,

            depth_buffer_texture,
            depth_buffer_texture_view,

            materials: Vec::new(),
            shaders: Vec::new(),
            meshes: Vec::new(),

            mesh_query: RwLock::new(<(&TransformComponent, &MeshComponent)>::query()),
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.swap_chain_descriptor.width = width;
        self.swap_chain_descriptor.height = height;
        self.swap_chain = self
            .device
            .create_swap_chain(&self.surface, &self.swap_chain_descriptor);

        let (depth_buffer_texture, depth_buffer_texture_view) = Self::create_depth_texture(
            &self.device,
            &self.swap_chain_descriptor
        );
        self.depth_buffer_texture = depth_buffer_texture;
        self.depth_buffer_texture_view = depth_buffer_texture_view;
    }

    pub fn create_shader(
        &mut self,
        vertex_shader_module: &wgpu::ShaderModuleDescriptor,
        fragment_shader_module: &wgpu::ShaderModuleDescriptor,
        bind_group_layouts: &[&wgpu::BindGroupLayoutDescriptor],
    )-> ShaderRef
    {
        let i = self.shaders.len();
        let bind_group_layouts: SmallVec<_> = bind_group_layouts.iter()
            .map(|desc| self.device.create_bind_group_layout(desc))
            .collect();

        self.shaders.push(Shader {
            vertex_shader_module: self.device.create_shader_module(vertex_shader_module),
            fragment_shader_module: self.device.create_shader_module(fragment_shader_module),
            render_pipeline_layout: self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: std::iter::once(&self.render_uniform_bind_group_layout)
                    .chain(bind_group_layouts.iter())
                    .collect::<Vec<_>>().as_slice(),
                push_constant_ranges: &[]
            }),
            bind_group_layouts,

            marker: Default::default()
        });

        ShaderRef(i)
    }
    pub fn create_material(
        &mut self, shader_ref: ShaderRef,
        bind_groups: &[&[wgpu::BindGroupEntry]],
        vertex_buffer_layouts: &[wgpu::VertexBufferLayout],
        cull_mode: Option<wgpu::Face>,
    ) -> MaterialRef
    {
        let i = self.materials.len();
        let shader = &self.shaders[shader_ref.0];
        self.materials.push(Material {
            render_pipeline: self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&shader.render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader.vertex_shader_module,
                    entry_point: "vertex",
                    buffers: &vertex_buffer_layouts
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader.fragment_shader_module,
                    entry_point: "fragment",
                    targets: &[self.swap_chain_format.into()]
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode,
                    clamp_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default()
                }),
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
    pub fn create_mesh<T: Pod>(&mut self, material: MaterialRef, indices: &[u32], vertex_buffers: &[&[T]]) -> MeshRef {
        let i = self.meshes.len();
        self.meshes.push(Mesh {
            material,
            vertex_buffers: vertex_buffers.iter().map(|vertices| {
                self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(vertices),
                    usage: wgpu::BufferUsage::VERTEX,
                })
            }).collect(),
            indices: self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsage::INDEX,
            }),
            indices_size: indices.len()
        });

        MeshRef(i)
    }

    pub fn get_shader(&self, reference: ShaderRef) -> Option<&Shader> {
        self.shaders.get(reference.0)
    }
    pub fn get_shader_mut(&mut self, reference: ShaderRef) -> Option<&mut Shader> {
        self.shaders.get_mut(reference.0)
    }
    pub fn get_material(&self, reference: MaterialRef) -> Option<&Material> {
        self.materials.get(reference.0)
    }
    pub fn get_material_mut(&mut self, reference: MaterialRef) -> Option<&mut Material> {
        self.materials.get_mut(reference.0)
    }
    pub fn get_mesh(&self, reference: MeshRef) -> Option<&Mesh> {
        self.meshes.get(reference.0)
    }
    pub fn get_mesh_mut(&mut self, reference: MeshRef) -> Option<&mut Mesh> {
        self.meshes.get_mut(reference.0)
    }

    pub fn render(&self, world: &legion::World) {
        let current_camera = <(&CameraComponent, &TransformComponent)>::query()
            .iter(world)
            .filter(|(c, _)| c.is_enabled)
            .next();

        if let Some((current_camera, transform)) = current_camera {
            self.render_camera(current_camera, &transform, world);
        }
    }
    pub fn render_camera(&self, camera: &CameraComponent, camera_transform: &TransformComponent, world: &legion::World) {
        let camera_matrix = &*camera.matrix;

        self.queue.write_buffer(
            &self.render_uniform_buffer,
            offset_of!(RenderUniformBuffer, view_projection) as u64,
            bytemuck::cast_slice(camera_matrix.get_vp_matrix(camera_transform).as_slice())
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
                        load: match camera.clear_color {
                            None => wgpu::LoadOp::Load,
                            Some(c) => wgpu::LoadOp::Clear(c),
                        },
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_buffer_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: true
                    }),
                    stencil_ops: None
                }),
            });
            r_pass.set_bind_group(0, &self.render_uniform_bind_group, &[]);

            let mut last_material = None;
            self.mesh_query.write().unwrap().for_each(world, |(transform, MeshComponent(mesh_ref))| {
                let mesh = &self.meshes[mesh_ref.0];
                if last_material != Some(mesh.material) {
                    last_material = Some(mesh.material);
                    let material = &self.materials[mesh.material.0];
                    r_pass.set_pipeline(&material.render_pipeline);
                    for (bg, i) in material.bind_groups.iter().zip(1..) {
                        r_pass.set_bind_group(i, bg, &[]);
                    }
                }

                self.queue.write_buffer(
                    &self.render_uniform_buffer,
                    offset_of!(RenderUniformBuffer, model_matrix) as u64,
                    bytemuck::cast_slice(transform.to_homogeneous().as_slice())
                );

                r_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                for (vertex, i) in mesh.vertex_buffers.iter().zip(0..) {
                    r_pass.set_vertex_buffer(i, vertex.slice(..));
                }

                r_pass.draw_indexed(
                    0..mesh.indices_size as u32,
                    0,
                    0..1
                );
            });
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
