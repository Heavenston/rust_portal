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

    pub material: MaterialHandle,
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
    pub(super) object_bind_group: ObjectBindGroupHandle,
}

pub type StaticMeshHandle = handle_map::Handle<StaticMeshData>;

#[derive(Debug)]
pub struct EngineState {
    pub renderer: Renderer,

    pub camera: Option<Camera>,
    /// immutable other than for the public mut methods
    pub(super) static_meshes: handle_map::HandleMap<StaticMeshData>,

    pub materials: MaterialsStore,
}

impl EngineState {
    pub fn new(mut renderer: Renderer) -> Self {
        let mut materials: MaterialsStore = default();
        materials.register(super::pbr_material_factory(&mut renderer));
        Self {
            renderer,

            camera: default(),
            static_meshes: default(),

            materials,
        }
    }

    pub fn insert_static_mesh(&mut self, mesh: StaticMesh) -> StaticMeshHandle {
        let renderer = &mut self.renderer;

        let transform: Mat4 = mesh.transform.into();
        let uniform_buffer = renderer.create_buffer()
            .size(size_of::<Mat4>() as u64)
            .data(bytemuck::bytes_of(&transform))
            .create();

        let object_bind_group = renderer.create_object_bind_group()
            .uniform_buffer(uniform_buffer)
            .create();

        self.static_meshes.insert(StaticMeshData {
            mesh,
            object_bind_group,
        })
    }

    pub fn remove_static_mesh(&mut self, handle: StaticMeshHandle) -> Option<StaticMesh> {
        let StaticMeshData { mesh, object_bind_group } = self.static_meshes.remove(handle)?;
        self.renderer.delete_object_bind_group(object_bind_group);
        Some(mesh)
    }
}
