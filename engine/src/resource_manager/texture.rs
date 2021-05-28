use crate::renderer::Renderer;
use image::{RgbaImage, EncodableLayout, ImageBuffer};
use wgpu::util::DeviceExt;

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: Option<wgpu::Sampler>,
}

impl Texture {
    pub fn create_plain_color_texture(renderer: &Renderer, color: image::Rgba<u8>, label: Option<&str>) -> Self {
        Self::create_texture(renderer, &ImageBuffer::from_pixel(4, 4, color), label)
    }
    pub fn create_texture(renderer: &Renderer, image: &RgbaImage, label: Option<&str>) -> Self {
        let texture = renderer.device.create_texture_with_data(&renderer.queue, &wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width: image.dimensions().0,
                height: image.dimensions().1,
                depth_or_array_layers: 1
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
        }, image.as_bytes());
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            sampler: None,
        }
    }
    pub fn create_sampler(&mut self, renderer: &Renderer, address_mode: wgpu::AddressMode, mag_filter: wgpu::FilterMode, min_filter: wgpu::FilterMode) {
        self.sampler = Some(renderer.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            mag_filter,
            min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        }));
    }
}
