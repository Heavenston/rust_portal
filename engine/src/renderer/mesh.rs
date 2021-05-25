use super::*;

pub struct Mesh {
    pub material: MaterialRef,

    pub vertex_buffers: SmallVec<[wgpu::Buffer; 2]>,
    pub indices: wgpu::Buffer,
    pub indices_size: usize,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MeshRef(pub(crate) usize);
