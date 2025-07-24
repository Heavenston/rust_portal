use super::*;
use crate::handle_map;

use derive_more::From;

#[derive(Debug)]
pub struct BindGroupData {
    pub(super) bind_group: wgpu::BindGroup,
}
pub type BindGroupHandle = handle_map::Handle<BindGroupData>;

#[derive(Debug, Clone, Copy, From)]
pub enum BindGroupResourceHandle {
    Buffer(BufferHandle),
    Texture(TextureHandle),
}

impl BindGroupResourceHandle {
    pub(super) fn to_wgpu<'a>(self, resources: &'a RendererResources) -> Option<wgpu::BindingResource<'a>> {
        Some(match self {
            Self::Buffer(handle) => {
                let buffer = &resources.buffers.get(handle)?.buffer;
                buffer.as_entire_binding()
            },
            Self::Texture(handle) => {
                let texture_view = &resources.textures.get(handle)?.view;
                wgpu::BindingResource::TextureView(&texture_view)
            },
        })
    }
}

#[bon::builder(finish_fn = create)]
pub fn create_bind_group(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    #[builder(field)]
    entries: Vec<(u32, BindGroupResourceHandle)>,
    #[builder(name = layout)]
    layout_handle: BindGroupLayoutHandle,
) -> BindGroupHandle {
    let layout = &renderer.resources.bind_group_layouts.get(layout_handle)
        .expect("Invalid layout handle given").bind_group_layout;

    let bind_group = renderer.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("object_bind_group"),
        layout,
        entries: &entries.iter().map(|&(binding, handle)| wgpu::BindGroupEntry {
            binding,
            resource: handle.to_wgpu(&renderer.resources)
                .expect("Invalid bind group handle given"),
        }).collect::<Vec<_>>(),
    });

    renderer.resources.bind_groups.insert(BindGroupData { bind_group })
}

impl<'a, S> CreateBindGroupBuilder<'a, S>
    where S: create_bind_group_builder::State,
{
    pub fn entry(mut self, binding: u32, handle: impl Into<BindGroupResourceHandle>) -> Self {
        self.entries.push((binding, handle.into()));
        self
    }
}
