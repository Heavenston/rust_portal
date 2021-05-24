use portal_engine::renderer::Renderer;
use portal_engine::camera::PerspectiveCamera;
use winit::dpi::LogicalSize;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title("Portal !");
    window.set_inner_size(LogicalSize::new(500,500));

    println!("Creating renderer...");
    let renderer =
        Box::into_raw(Box::new(pollster::block_on(Renderer::new(&window, 100, 100))));
    println!("Created renderer");
    let camera = PerspectiveCamera::new();

    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };
        let renderer = unsafe { &mut *renderer };

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
