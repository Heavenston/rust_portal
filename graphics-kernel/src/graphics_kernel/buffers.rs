use super::*;

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
    #[builder(setters(name = bytes))]
    bytes: Option<&[u8]>,
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
        mapped_at_creation: bytes.is_some(),
    });

    if let Some(bytes) = bytes {
        assert_eq!(bytes.len() as u64, size);

        buffer.get_mapped_range_mut(..).copy_from_slice(bytes);
        buffer.unmap();
    }

    kernel.resources.buffers.insert(BufferData { buffer })
}

impl<'f1, 'f2, S> CreateBufferBuilderBuilder<'f1, 'f2, S>
    where S: create_buffer_builder_builder::State,
          S::Size: create_buffer_builder_builder::IsUnset,
          S::Bytes: create_buffer_builder_builder::IsUnset,
{
    /// Automatically sets the size based on the data's len, use `bytes`
    /// to be able to also specify the size separately
    pub fn data(self, data: &'f2 impl bytemuck::NoUninit) -> CreateBufferBuilderBuilder<'f1, 'f2,
        create_buffer_builder_builder::SetBytes<
            create_buffer_builder_builder::SetSize<
                S
            >
        >
    > {
        let data = bytemuck::bytes_of(data);
        self.size(data.len().try_into().expect("No overflow")).bytes(data)
    }

    /// Automatically sets the size based on the data's len, use `bytes`
    /// to be able to also specify the size separately
    pub fn slice<T: bytemuck::NoUninit>(self, slice: &'f2 [T]) -> CreateBufferBuilderBuilder<'f1, 'f2,
        create_buffer_builder_builder::SetBytes<
            create_buffer_builder_builder::SetSize<
                S
            >
        >
    > {
        let data = bytemuck::cast_slice::<T, u8>(slice);
        self.size(data.len().try_into().expect("No overflow")).bytes(data)
    }
}
