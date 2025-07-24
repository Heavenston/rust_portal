use super::*;
use crate::handle_map;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TextureFormat {
    Rgba8Unorm,
}

impl TextureFormat {
    pub fn pixel_byte_size(self) -> u32 {
        match self {
            TextureFormat::Rgba8Unorm => 4,
        }
    }
}

impl From<TextureFormat> for wgpu::TextureFormat {
    fn from(value: TextureFormat) -> Self {
        match value {
            TextureFormat::Rgba8Unorm => Self::Rgba8Unorm,
        }
    }
}

impl TryFrom<wgpu::TextureFormat> for TextureFormat {
    type Error = ();

    fn try_from(value: wgpu::TextureFormat) -> Result<Self, Self::Error> {
        match value {
            wgpu::TextureFormat::Rgba8Unorm => Ok(Self::Rgba8Unorm),
            _ => Err(()),
        }
    }
}

#[derive(Debug)]
pub struct TextureData {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) format: Option<TextureFormat>,
}

impl TextureData {
    pub(super) fn from_wgpu(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&default()),
            format: texture.format().try_into().ok(),
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
        format: format.into(),
        usage: wgpu::TextureUsages::COPY_DST |
            wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    renderer.resources.textures.insert(TextureData::from_wgpu(texture))
}

