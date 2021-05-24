use super::*;

pub struct Mesh<'a> {
    pub material: &'a Material<'a>,

    pub vertices: wgpu::Buffer,
    pub vertices_size: usize,
    pub indices: wgpu::Buffer,
    pub indices_size: usize,
}
