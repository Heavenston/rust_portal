use std::num::NonZero;

use super::*;
use utils::handle_map;

#[derive(Debug)]
pub struct BindGroupLayoutData {
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
}
pub type BindGroupLayoutHandle = handle_map::Handle<BindGroupLayoutData>;

#[bon::builder(finish_fn = add)]
pub fn add_bind_group_layout_entry<S>(
    #[builder(start_fn)]
    mut parent: CreateBindGroupLayoutBuilder<'_, S>,
    binding: u32,
    #[builder(setters(vis = "", name = ty_internal))]
    ty: wgpu::BindingType,
    count: Option<NonZero<u32>>,
) -> CreateBindGroupLayoutBuilder<'_, S>
where S: create_bind_group_layout_builder::State {
    parent.entries.push(wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty,
        count,
    });
    parent
}

impl<'f1, PS, S> AddBindGroupLayoutEntryBuilder<'f1, PS, S>
    where PS: create_bind_group_layout_builder::State,
          S: add_bind_group_layout_entry_builder::State,
          S::Ty: add_bind_group_layout_entry_builder::IsUnset,
{
    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn uniform_buffer(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty_internal(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        })
    }

    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn texture(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty_internal(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        })
    }

    // NOTE: If more options is required making another sub-builder should probably
    //       be prefered
    pub fn sampler(
        self
    ) -> AddBindGroupLayoutEntryBuilder<'f1, PS, add_bind_group_layout_entry_builder::SetTy<S>> {
        self.ty_internal(wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering))
    }
}

#[bon::builder(finish_fn = create)]
pub fn create_bind_group_layout(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    #[builder(field)]
    entries: Vec<wgpu::BindGroupLayoutEntry>,
) -> BindGroupLayoutHandle {
    let bind_group_layout = renderer.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &entries,
    });

    renderer.resources.bind_group_layouts.insert(BindGroupLayoutData { bind_group_layout })
}

impl<'a, S> CreateBindGroupLayoutBuilder<'a, S>
    where S: create_bind_group_layout_builder::State,
{
    pub fn entry(self) -> AddBindGroupLayoutEntryBuilder<'a, S> {
        add_bind_group_layout_entry(self)
    }
}
