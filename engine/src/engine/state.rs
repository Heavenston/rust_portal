use crate::{ utils::*, * };

use glam::{ Affine3A, Mat4 };

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub transform: Affine3A,
    pub projection: Mat4,

    pub clear_color: wgpu::Color,
}

#[derive(Debug, Clone, Copy)]
pub struct StaticMesh {
    pub transform: Affine3A,
    /// list of vec3
    pub positions_buffer: BufferHandle,
    /// list of vec2
    pub texcoords_buffer: BufferHandle,
    /// list of u32
    pub index_buffer: BufferHandle,
    /// At most the size of the indices buffer
    pub vertex_count: u32,

    pub material_instance: MaterialInstance,
}

impl StaticMesh {
    pub fn delete_buffers(&self, renderer: &mut Renderer) {
        renderer.delete_buffer(self.positions_buffer);
        renderer.delete_buffer(self.texcoords_buffer);
        renderer.delete_buffer(self.index_buffer);
    }
}

#[derive(Debug)]
pub struct StaticMeshData {
    pub mesh: StaticMesh,
    pub(super) object_bind_group: BindGroupHandle,
}

pub type StaticMeshHandle = handle_map::Handle<StaticMeshData>;

#[derive(Debug)]
pub struct EngineState {
    pub renderer: Renderer,

    pub camera: Option<Camera>,
    /// immutable other than for the public mut methods
    pub(super) static_meshes: handle_map::HandleMap<StaticMeshData>,

    pub(super) camera_bind_group_layout: BindGroupLayoutHandle,
    pub(super) object_bind_group_layout: BindGroupLayoutHandle,

    pub materials: MaterialsStore,
}

impl EngineState {
    pub fn new(mut renderer: Renderer) -> Self {
        let camera_bind_group_layout = renderer.create_bind_group_layout()
            .entry().binding(0).uniform_buffer().add()
            .create();
        let object_bind_group_layout = renderer.create_bind_group_layout()
            .entry().binding(0).uniform_buffer().add()
            .create();

        let mut this = Self {
            renderer,

            camera: default(),
            static_meshes: default(),

            camera_bind_group_layout,
            object_bind_group_layout,

            materials: default(),
        };

        let factory = super::create_pbr_material_factory(&mut this);
        this.materials.register(factory);

        this
    }

    pub fn insert_static_mesh(&mut self, mesh: StaticMesh) -> StaticMeshHandle {
        let renderer = &mut self.renderer;

        let transform: Mat4 = mesh.transform.into();
        let uniform_buffer = renderer.create_buffer()
            .size(size_of::<Mat4>() as u64)
            .data(bytemuck::bytes_of(&transform))
            .create();

        let object_bind_group = renderer.create_bind_group()
            .layout(self.object_bind_group_layout)
            .entry(0, uniform_buffer)
            .create();

        self.static_meshes.insert(StaticMeshData {
            mesh,
            object_bind_group,
        })
    }

    pub fn remove_static_mesh(&mut self, handle: StaticMeshHandle) -> Option<StaticMesh> {
        let _ = handle;
        // This leads to leaks (the uniform buffer) which without reference counted handles i dont
        // know how to fix (other than just included the uniform buffer in StaticMeshData)
        unimplemented!()
    }
}
