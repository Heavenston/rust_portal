use super::*;
use utils::handle_map;

#[derive(Debug)]
pub struct BufferData {
    pub(super) buffer: wgpu::Buffer,
}
pub type BufferHandle = handle_map::Handle<BufferData>;

#[bon::builder(finish_fn = create)]
pub fn create_buffer_builder(
    #[builder(start_fn)]
    kernel: &mut GraphicsKernel,
    size: u64,
    data: Option<&[u8]>,
) -> BufferHandle {
    let buffer = kernel.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        // TODO: FIXME: As needed instead of everything
        usage: wgpu::BufferUsages::VERTEX
            | wgpu::BufferUsages::INDEX
            | wgpu::BufferUsages::UNIFORM
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: data.is_some(),
    });

    if let Some(data) = data {
        assert_eq!(data.len() as u64, size);

        buffer.get_mapped_range_mut(..).copy_from_slice(data);
        buffer.unmap();
    }

    kernel.resources.buffers.insert(BufferData { buffer })
}
