use super::*;
use crate::handle_map;

#[derive(Debug)]
pub struct CameraBindGroupData {
    pub(super) bind_group: wgpu::BindGroup,
}
pub type CameraBindGroupHandle = handle_map::Handle<CameraBindGroupData>;

#[bon::builder(finish_fn = create)]
pub fn create_camera_bind_group(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    uniform_buffer: BufferHandle,
) -> CameraBindGroupHandle {
    let uniform_buffer = &renderer.resources.buffers.get(uniform_buffer)
        .expect("Invalid uniform buffer handle given").buffer;

    let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("camera_bind_group"),
        layout: &renderer.camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    renderer.resources.camera_bind_groups.insert(CameraBindGroupData { bind_group })
}

