#![feature(iter_array_chunks)]
#![feature(array_repeat)]

mod application;
use application::*;

use utils::prelude::*;

#[derive(rust_embed::Embed)]
#[folder = "../resources"]
pub struct Resources;

fn upload_image_resource(state: &mut engine::EngineState, path: &str, srgb: bool) -> Option<pgk::TextureHandle> {
    let texture_file = Resources::get(path)?;
    let texture_data = &*texture_file.data;
    let texture_image = image::load_from_memory(&texture_data).ok()?.to_rgba8();

    let texture_handle = state.kernel.create_texture()
        .usages(pgk::TextureUsages {
            copy_dst: true,
            texture_binding: true,
            ..default()
        })
        .format(if srgb {
            pgk::TextureFormat::Rgba8UnormSrgb
        } else {
            pgk::TextureFormat::Rgba8Unorm
        })
        .width(texture_image.width()).height(texture_image.height())
        .create();
    state.kernel.write_texture(texture_handle, &texture_image.as_raw());
    Some(texture_handle)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
    event_loop.run_app(&mut engine::Engine::builder()
        .window_attributes(winit::window::WindowAttributes::default()
            .with_title("Rust Portal")
            .with_resizable(true)
            .with_min_inner_size(winit::dpi::LogicalSize::new(100, 100))
        )
        .application_factory(Application::new)
        .build()
    )?;

    Ok(())
}
