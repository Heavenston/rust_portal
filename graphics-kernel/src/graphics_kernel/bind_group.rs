use std::borrow::Cow;

use super::*;
use utils::{ itertools::chain, prelude::* };

use derive_more::From;

#[derive(Debug)]
pub struct BindGroupData {
    pub(super) bind_group: wgpu::BindGroup,
}
pub type BindGroupHandle = handle_map::Handle<BindGroupData>;

#[derive(Debug, Clone, From, kinded::Kinded)]
pub enum BindGroupResourceHandle {
    Buffer(BufferHandle),
    Texture(TextureHandle),
    Sampler(wgpu::Sampler),
}

impl BindGroupResourceHandle {
    pub fn matches_type(&self, ty: BindingEntryType) -> bool {
        match (self, ty) {
            (Self::Buffer(..), BindingEntryType::UniformBuffer) => true,
            (Self::Buffer(..), _) => false,
            (Self::Texture(..), BindingEntryType::Texture) => true,
            (Self::Texture(..), _) => false,
            (Self::Sampler(..), BindingEntryType::FilteringSampler) => true,
            (Self::Sampler(..), _) => false,
        }
    }

    pub(super) fn to_wgpu<'a>(&'a self, resources: &'a GraphicsKernelResources) -> Option<wgpu::BindingResource<'a>> {
        Some(match *self {
            Self::Buffer(handle) => {
                let buffer = &resources.buffers.get(handle)?.buffer;
                buffer.as_entire_binding()
            },
            Self::Texture(handle) => {
                let texture_view = &resources.textures.get(handle)?.view;
                wgpu::BindingResource::TextureView(&texture_view)
            },
            Self::Sampler(ref sampler) => {
                wgpu::BindingResource::Sampler(sampler)
            }
        })
    }
}

#[bon::builder(finish_fn = create)]
pub fn create_bind_group(
    #[builder(start_fn)]
    kernel: &mut GraphicsKernel,
    #[builder(field)]
    entries: Vec<(u32, BindGroupResourceHandle)>,
    #[builder(name = layout)]
    layout_handle: BindGroupLayoutHandle,
    #[builder(into)]
    label: Option<Cow<'_, str>>,
) -> BindGroupHandle {
    let layout_data = kernel.resources.bind_group_layouts.get(layout_handle)
        .expect("Invalid layout handle given");
    let layout = &layout_data.bind_group_layout;

    if cfg!(feature = "checks") {
        let layout_name = layout_data.label.as_ref()
            .map(|name| Cow::Owned(format!(" '{name}'")))
            .unwrap_or(Cow::Borrowed(" unnamed"));
        chain(
            layout_data.entries.iter()
                .map(|entry| (entry.binding, (Some(entry.ty), None))),
            entries.iter().map(|&(a, ref b)| (a, b))
                .map(|(binding, handle)| (binding, (None, Some(handle))))
        ).into_grouping_map().reduce(|(ty, handle), _binding, (nty, nhandle)| {
            (ty.or(nty), handle.or(nhandle))
        }).into_iter().for_each(|(binding, (ty, handle))| {
            match (ty, handle) {
                (None, None) => unreachable!(),
                (Some(ty), Some(handle)) => assert!(handle.matches_type(ty), "Binding {binding} has a mismatched type with the bind group layout{layout_name}, binding type: {ty:?}, got handle type: {}", handle.kind()),
                (None, Some(handle)) => panic!("Given binding {binding} that isn't present in the bind group layout{layout_name}, got handle type: {}", handle.kind()),
                (Some(ty), None) => panic!("Required binding {binding} of type {ty:?} from bind group layout{layout_name} was not given"),
            }
        });
    }

    let bind_group = kernel.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: label.as_deref(),
        layout,
        entries: &entries.iter().map(|&(binding, ref handle)| wgpu::BindGroupEntry {
            binding,
            resource: handle.to_wgpu(&kernel.resources)
                .expect("Invalid bind group handle given"),
        }).collect::<Vec<_>>(),
    });

    kernel.resources.bind_groups.insert(BindGroupData { bind_group })
}

impl<'a, 'b, S> CreateBindGroupBuilder<'a, 'b, S>
    where S: create_bind_group_builder::State,
{
    pub fn entry(mut self, binding: u32, handle: impl Into<BindGroupResourceHandle>) -> Self {
        self.entries.push((binding, handle.into()));
        self
    }

    /// I was to lazy to make anything other than hard coded default sampler descriptor
    pub fn sampler(mut self, binding: u32) -> Self {
        let sampler = self.kernel.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            anisotropy_clamp: 3,
            ..default()
        });
        self.entries.push((binding, BindGroupResourceHandle::Sampler(sampler)));
        self
    }
}
