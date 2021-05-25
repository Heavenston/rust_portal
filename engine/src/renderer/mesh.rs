use super::*;

pub struct Mesh {
    pub material: MaterialRef,

    pub vertices: wgpu::Buffer,
    pub vertices_size: usize,
    pub indices: wgpu::Buffer,
    pub indices_size: usize,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct MeshRef(pub(crate) usize);
