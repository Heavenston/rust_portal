use portal_engine::renderer::{Renderer, MeshComponent};
use portal_engine::camera::{PerspectiveCameraMatrix, CameraComponent};
use winit::dpi::LogicalSize;
use std::borrow::Cow;
use wgpu::util::DeviceExt;
use portal_engine::transform::TransformComponent;
use nalgebra::UnitQuaternion;
use std::f32;
use std::time::Instant;
use legion::query::IntoQuery;

fn main() {
    let mut world = legion::World::new(legion::WorldOptions::default());

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title("Portal !");
    window.set_inner_size(LogicalSize::new(500,500));
    window.set_min_inner_size(Some(LogicalSize::new(100, 100)));

    println!("Creating renderer...");
    let mut renderer = Box::new(pollster::block_on(Renderer::new(&window, 100, 100)));
    println!("Created renderer");
    let camera_entity = world.push((CameraComponent {
        clear_color: Some(wgpu::Color { r: 88. / 255., g: 101. / 255., b: 242. / 255., a: 1. }),
        matrix: Box::new({
            let mut m = PerspectiveCameraMatrix::new();
            m.0.set_znear_and_zfar(0.01, 5_000.);
            m
        }),
        is_enabled: true,
    }, {
        let mut transform = TransformComponent::default();
        transform.position.y = 100.;
        transform
    }));

    let uniform_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::bytes_of(&[0.0f32, 1., 0., 1.]),
        usage: wgpu::BufferUsage::UNIFORM
    });

    let shader_m = wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        flags: Default::default()
    };
    let shader = renderer.create_shader(
        &shader_m,
        &shader_m,
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
        }]
    );
    let material = renderer.create_material(shader, &[&[
        wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
        }
    ]], &[
        wgpu::VertexBufferLayout {
            array_stride: 3 * std::mem::size_of::<f32>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0
            }]
        },
        wgpu::VertexBufferLayout {
            array_stride: 3 * std::mem::size_of::<f32>() as u64,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 1
            }]
        }
    ], Some(wgpu::Face::Front));

    let (models, materials) = tobj::load_obj("resources/crytek-sponza-huge-vray-obj/crytek-sponza-huge-vray.obj", &tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
    }).unwrap();
    let materials = materials.unwrap();
    println!("Loading {} models and {} materials", models.len(), materials.len());
    let _ = models.iter()
        .map(|model| renderer.create_mesh(material, &model.mesh.indices, &[
            &model.mesh.positions,
            &model.mesh.normals
        ]))
        .map(|mesh| world.push((
            MeshComponent(mesh),
            TransformComponent::default()
        )))
        .collect::<Vec<_>>();

    let start = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::{ControlFlow},
        };

        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                renderer.resize(size.width, size.height);

                let camera_component = <&mut CameraComponent>::query().get_mut(&mut world, camera_entity).unwrap();
                let matrix: &mut Box<PerspectiveCameraMatrix> = unsafe { std::mem::transmute(&mut camera_component.matrix) };
                matrix.0.set_aspect(size.width as f32 / size.height as f32);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            Event::MainEventsCleared  => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                {
                    let t = <&mut TransformComponent>::query().get_mut(&mut world, camera_entity).unwrap();
                    t.rotation = UnitQuaternion::from_euler_angles(0., start.elapsed().as_secs_f32() / 3., 0.);
                }
                renderer.render(&world);
            }

            _ => {}
        }
    });
}
