use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use pgk::color::LinearRgb;

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct VhvHeader {
    pub version: i32,
    pub checksum: u32,
    pub vertex_flags: u32,
    pub vertex_size: u32,
    pub num_total_vertices: u32,
    pub num_meshes: i32,
    pub unused: [u32; 4],
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct VhvMesh {
    pub lod: u32,
    pub num_vertices: u32,
    pub vertex_offset: u32,
    pub unused: [u32; 4],
}

#[derive(Debug, Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct VhvVertex {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub exponent: u8,
}

impl VhvVertex {
    pub fn to_rgb(self) -> LinearRgb {
        // The exponent is stored with a bias of 128.
        let mul = 2f32.powi(self.exponent as i32 - 128);
        LinearRgb {
            r: (self.r as f32 / 255.0) * mul,
            g: (self.g as f32 / 255.0) * mul,
            b: (self.b as f32 / 255.0) * mul,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Vhv<'a> {
    pub header: &'a VhvHeader,
    pub meshes: &'a [VhvMesh],
    vertex_data: &'a [u8],
}

impl<'a> Vhv<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Self, bytemuck::PodCastError> {
        let header: &VhvHeader = bytemuck::try_from_bytes(&bytes[..size_of::<VhvHeader>()])?;

        let mesh_headers_start = size_of::<VhvHeader>();
        let num_meshes = header.num_meshes as usize;
        let mesh_headers_end = mesh_headers_start + num_meshes * size_of::<VhvMesh>();
        let meshes: &[VhvMesh] = bytemuck::try_cast_slice(&bytes[mesh_headers_start..mesh_headers_end])?;

        let vertex_data_start = if let Some(first_mesh) = meshes.first() {
            first_mesh.vertex_offset as usize
        } else {
            bytes.len()
        };

        let vertex_data = &bytes[vertex_data_start..];

        Ok(Self {
            header,
            meshes,
            vertex_data,
        })
    }

    pub fn mesh_vertices(&self, mesh_index: usize) -> Option<&'a [VhvVertex]> {
        let mesh = self.meshes.get(mesh_index)?;
        let vertex_data_start_offset = self.meshes.first()?.vertex_offset as usize;

        let relative_start = mesh.vertex_offset as usize - vertex_data_start_offset;
        let vertices_len = mesh.num_vertices as usize * size_of::<VhvVertex>();
        let relative_end = relative_start + vertices_len;

        bytemuck::try_cast_slice(&self.vertex_data.get(relative_start..relative_end)?).ok()
    }
}
