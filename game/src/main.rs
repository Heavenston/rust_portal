use portal_engine::renderer::Renderer;
use portal_engine::camera::PerspectiveCamera;
use winit::dpi::LogicalSize;
use std::borrow::Cow;
use wgpu::util::DeviceExt;

fn main() {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title("Portal !");
    window.set_inner_size(LogicalSize::new(500,500));
    window.set_min_inner_size(Some(LogicalSize::new(100, 100)));

    println!("Creating renderer...");
    let mut renderer = Box::new(pollster::block_on(Renderer::new(&window, 100, 100)));
    println!("Created renderer");
    let camera = PerspectiveCamera::new();

    let uniform_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&[0.0f32, 1., 0., 1.]),
        usage: wgpu::BufferUsage::UNIFORM
    });

    #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
    #[repr(C)]
    struct Vertex {
        position: [f32; 2],
    }

    let shader = renderer.create_shader(
        &wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
            flags: Default::default()
        },
        &[&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStage::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None
                },
                count: None
            }]
        }],
        &[
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2
                ]
            }
        ]
    );
    let material = renderer.create_material(shader, &[
        &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
            }
        ]
    ]);

    let mesh = renderer.create_mesh(material, &[0, 1, 2], &[
        Vertex {
            position: [-1., -1.]
        },
        Vertex {
            position: [0., 1.]
        },
        Vertex {
            position: [1., -1.]
        }
    ]);

    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };

        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.resize(size.width, size.height);
            }
            Event::RedrawRequested(_) => {
                renderer.render(&camera, &[mesh]);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
