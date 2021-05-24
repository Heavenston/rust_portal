use portal_engine::*;
use portal_engine::renderer::Renderer;
use portal_engine::camera::PerspectiveCamera;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };
        let mut renderer = pollster::block_on(Renderer::new(&window, 100, 100));
        let camera = PerspectiveCamera::new();

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.resize(size.width, size.height);
            }
            Event::RedrawRequested(_) => {
                renderer.render(&camera, &[]);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}