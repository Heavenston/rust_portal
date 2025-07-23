use super::*;
use crate::handle_map;

#[derive(Debug)]
pub struct ObjectBindGroupData {
    pub(super) bind_group: wgpu::BindGroup,
}
pub type ObjectBindGroupHandle = handle_map::Handle<ObjectBindGroupData>;

#[bon::builder(finish_fn = create)]
pub fn create_object_bind_group(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    uniform_buffer: BufferHandle,
) -> ObjectBindGroupHandle {
    let uniform_buffer = &renderer.resources.buffers.get(uniform_buffer)
        .expect("Invalid positions buffer handle given").buffer;

    let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("object_bind_group"),
        layout: &renderer.object_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    renderer.resources.object_bind_groups.insert(ObjectBindGroupData { bind_group })
}
