use glam::{ Affine3A, Mat4 };

use crate::{ *, utils::*, handle_map::HandleMap };

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub transform: Affine3A,
    pub projection: Mat4,

    pub clear_color: wgpu::Color,
}

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
}

#[derive(Default, Debug, Clone)]
pub(crate) struct StaticMeshGPUCache {
    pub object_bind_group: ObjectBindGroupHandle,
}

pub struct StaticMeshData {
    pub mesh: StaticMesh,
    pub(crate) cache: Option<StaticMeshGPUCache>,
}

impl From<StaticMesh> for StaticMeshData {
    fn from(mesh: StaticMesh) -> Self {
        Self {
            mesh,
            cache: default(),
        }
    }
}

pub type StaticMeshHandle = handle_map::Handle<StaticMeshData>;

#[derive(Default)]
pub struct World {
    pub camera: Option<Camera>,
    pub(crate) static_meshes: HandleMap<StaticMeshData>,
}

impl World {
    pub fn static_meshes(&self) -> &HandleMap<StaticMeshData> {
        &self.static_meshes
    }

    pub fn instert_static_mesh(&mut self, mesh: StaticMesh) -> StaticMeshHandle {
        self.static_meshes.insert(mesh.into())
    }

    pub fn remove_static_mesh(&mut self, handle: StaticMeshHandle) {
        self.static_meshes.remove(handle);
    }
}
