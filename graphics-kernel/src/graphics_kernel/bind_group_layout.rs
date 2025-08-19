use std::{borrow::Cow, num::NonZero};

use super::*;

#[derive(Debug)]
pub struct BindGroupLayoutData {
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
    pub label: Option<String>,
    pub entries: Box<[BindGroupLayoutEntry]>,
}
pub type BindGroupLayoutHandle = handle_map::Handle<BindGroupLayoutData>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BindingEntryType {
    UniformBuffer,
    Texture,
    FilteringSampler,
}

impl BindingEntryType {
    pub(crate) fn to_wgpu(&self) -> wgpu::BindingType {
        match self {
            BindingEntryType::UniformBuffer => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BindingEntryType::Texture => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            BindingEntryType::FilteringSampler => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BindGroupLayoutEntry {
    pub binding: u32,
    pub ty: BindingEntryType,
    pub count: Option<NonZero<u32>>,
}

impl BindGroupLayoutEntry {
    pub(crate) fn to_wgpu(&self) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding: self.binding,
            ty: self.ty.to_wgpu(),
            count: self.count,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        }
    }
}

#[bon::builder(finish_fn = add)]
pub fn add_bind_group_layout_entry<'f1, 'f2, S>(
    #[builder(start_fn)]
    mut parent: CreateBindGroupLayoutBuilder<'f1, 'f2, S>,
    binding: u32,
    ty: BindingEntryType,
    count: Option<NonZero<u32>>,
) -> CreateBindGroupLayoutBuilder<'f1, 'f2, S>
where S: create_bind_group_layout_builder::State {
    parent.entries.push(BindGroupLayoutEntry {
        binding,
        ty,
        count,
    });
    parent
}

impl<'f1, 'f2, PS, S> AddBindGroupLayoutEntryBuilder<'f1, 'f2, PS, S>
    where PS: create_bind_group_layout_builder::State,
          S: add_bind_group_layout_entry_builder::State,
          S::Ty: add_bind_group_layout_entry_builder::IsUnset,
{
    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn uniform_buffer(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, 'f2, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty(BindingEntryType::UniformBuffer)
    }

    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn texture(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, 'f2, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty(BindingEntryType::Texture)
    }

    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn sampler(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, 'f2, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty(BindingEntryType::FilteringSampler)
    }
}

#[bon::builder(finish_fn = create)]
pub fn create_bind_group_layout(
    #[builder(start_fn)]
    kernel: &mut GraphicsKernel,
    #[builder(field)]
    entries: Vec<BindGroupLayoutEntry>,
    #[builder(into)]
    label: Option<Cow<'_, str>>,
) -> BindGroupLayoutHandle {
    let wgpu_entries = entries.iter().map(BindGroupLayoutEntry::to_wgpu).collect::<Vec<_>>();

    let bind_group_layout = kernel.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: label.as_deref(),
        entries: &wgpu_entries,
    });

    kernel.resources.bind_group_layouts.insert(BindGroupLayoutData {
        bind_group_layout,
        label: label.map(String::from),
        entries: entries.into_boxed_slice(),
    })
}

impl<'f1, 'f2, S> CreateBindGroupLayoutBuilder<'f1, 'f2, S>
    where S: create_bind_group_layout_builder::State,
{
    pub fn entry(self) -> AddBindGroupLayoutEntryBuilder<'f1, 'f2, S> {
        add_bind_group_layout_entry(self)
    }
}
