#![feature(iter_array_chunks)]

mod application;
use application::*;
mod bsp_loader;
mod vhv_parser;

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
