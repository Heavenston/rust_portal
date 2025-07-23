use super::*;
use crate::handle_map;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba8Unorm,
}

impl TextureFormat {
    pub fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }
}

#[derive(Debug)]
pub struct TextureData {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
}

impl TextureData {
    pub(super) fn from_wgpu(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&default()),
            texture,
        }
    }
}

pub type TextureHandle = handle_map::Handle<TextureData>;

#[bon::builder(finish_fn = create)]
pub fn create_texture_builder(
    #[builder(start_fn)]
    renderer: &mut Renderer,
    width: u32,
    height: u32,
    format: TextureFormat,
) -> TextureHandle {
    assert!(width >= 1 && height >= 1, "Texture must not be of size 0 (given {width}x{height})");

    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: format.to_wgpu(),
        usage: wgpu::TextureUsages::COPY_DST |
            wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    renderer.resources.textures.insert(TextureData::from_wgpu(texture))
}

