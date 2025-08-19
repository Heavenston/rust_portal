use super::*;
use utils::prelude::*;

use derive_more::{ BitAnd, BitOr };

macro_rules! gen_texture_format {
    ($($name: ident => $size: expr),*$(,)?) => {
        #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
        pub enum TextureFormat {
            $(
                #[doc = concat!("See [wgpu::TextureFormat::", stringify!($name), "]")]
                $name,
            )*
        }

        impl TextureFormat {
            pub fn pixel_byte_size(self) -> u32 {
                match self {
                    $(Self::$name => $size,)*
                }
            }
        }

        impl From<TextureFormat> for wgpu::TextureFormat {
            fn from(value: TextureFormat) -> Self {
                match value {
                    $(TextureFormat::$name => wgpu::TextureFormat::$name,)*
                }
            }
        }

        impl TryFrom<wgpu::TextureFormat> for TextureFormat {
            type Error = ();

            fn try_from(value: wgpu::TextureFormat) -> Result<Self, Self::Error> {
                match value {
                    $(wgpu::TextureFormat::$name => Ok(TextureFormat::$name),)*
                    _ => Err(()),
                }
            }
        }
    };
}

gen_texture_format!(
    R8Unorm => 1,
    R8Snorm => 1,
    R8Uint => 1,
    R8Sint => 1,

    R16Uint => 2,
    R16Sint => 2,
    R16Float => 2,
    Rg8Unorm => 2,
    Rg8Snorm => 2,
    Rg8Uint => 2,
    Rg8Sint => 2,

    R32Uint => 4,
    R32Sint => 4,
    R32Float => 4,
    Rg16Uint => 4,
    Rg16Sint => 4,
    Rg16Float => 4,
    Rgba8Unorm => 4,
    Rgba8UnormSrgb => 4,
    Rgba8Snorm => 4,
    Rgba8Uint => 4,
    Rgba8Sint => 4,
    Bgra8Unorm => 4,
    Bgra8UnormSrgb => 4,
    // too weird for me lol
    // Rgb9e5Ufloat => 0,
    // Rgb10a2Uint => 0,
    // Rgb10a2Unorm => 0,
    // Rg11b10Ufloat => 0,

    Rg32Uint => 8,
    Rg32Sint => 8,
    Rg32Float => 8,
    Rgba16Uint => 8,
    Rgba16Sint => 8,
    Rgba16Float => 8,

    Rgba32Uint => 16,
    Rgba32Sint => 16,
    Rgba32Float => 16,

    // TODO: FIXME: They do not actually have a guarenteed size,
    // splitting into ColorTextureFormat and DepthStencilTextureFormat
    // and having a merged TextureFormat would be ideal
    
    Stencil8 => 1,
    Depth16Unorm => 2,
    // FIXME: What is its actual size? (spoiler it is either 4 or 5 depending on wgpu backend)
    Depth24Plus => 4,
    Depth24PlusStencil8 => 4,
    Depth32Float => 4,
);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, BitOr, BitAnd)]
pub struct TextureUsages {
    pub copy_src: bool,
    pub copy_dst: bool,
    pub texture_binding: bool,
    pub storage_binding: bool,
    pub render_attachment: bool,
}

impl From<wgpu::TextureUsages> for TextureUsages {
    fn from(value: wgpu::TextureUsages) -> Self {
        Self {
            copy_src: value.contains(wgpu::TextureUsages::COPY_SRC),
            copy_dst: value.contains(wgpu::TextureUsages::COPY_DST),
            texture_binding: value.contains(wgpu::TextureUsages::TEXTURE_BINDING),
            storage_binding: value.contains(wgpu::TextureUsages::STORAGE_BINDING),
            render_attachment: value.contains(wgpu::TextureUsages::RENDER_ATTACHMENT),
        }
    }
}

impl Into<wgpu::TextureUsages> for TextureUsages {
    fn into(self) -> wgpu::TextureUsages {
        wgpu::TextureUsages::from_bits(
            wgpu::TextureUsages::COPY_SRC.bits() * self.copy_src as u32 +
            wgpu::TextureUsages::COPY_DST.bits() * self.copy_dst as u32 +
            wgpu::TextureUsages::TEXTURE_BINDING.bits() * self.texture_binding as u32 +
            wgpu::TextureUsages::STORAGE_BINDING.bits() * self.storage_binding as u32 +
            wgpu::TextureUsages::RENDER_ATTACHMENT.bits() * self.render_attachment as u32
        ).expect("valid")
    }
}

#[derive(Debug)]
pub struct TextureData {
    pub(super) texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub format: TextureFormat,
    pub usages: TextureUsages,
}

impl TextureData {
    pub(super) fn from_wgpu(texture: wgpu::Texture) -> Self {
        Self {
            view: texture.create_view(&default()),
            format: texture.format().try_into().expect("Unsupported Format"),
            usages: texture.usage().into(),
            texture,
        }
    }
}

pub type TextureHandle = handle_map::Handle<TextureData>;

#[bon::builder(finish_fn = create)]
pub fn create_texture_builder(
    #[builder(start_fn)]
    kernel: &mut GraphicsKernel,
    usages: TextureUsages,
    width: u32,
    height: u32,
    #[builder(default = 1)]
    depth: u32,
    format: TextureFormat,
) -> TextureHandle {
    assert!(width >= 1 && height >= 1, "Texture must not be of size 0 (given {width}x{height})");

    let format: wgpu::TextureFormat = format.into();
    let texture = kernel.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: usages.into(),
        view_formats: &[format.remove_srgb_suffix(), format.add_srgb_suffix()],
    });
    kernel.resources.textures.insert(TextureData::from_wgpu(texture))
}

impl<'a, S> CreateTextureBuilderBuilder<'a, S>
    where S: create_texture_builder_builder::State,
          S::Width: create_texture_builder_builder::IsUnset,
          S::Height: create_texture_builder_builder::IsUnset,
{
    pub fn size(self, size: UVec2) -> CreateTextureBuilderBuilder<'a,
        create_texture_builder_builder::SetHeight<
            create_texture_builder_builder::SetWidth<S>
        >,
    > {
        self.width(size.x).height(size.y)
    }
}
