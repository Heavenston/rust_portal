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
use image::EncodableLayout;

fn create_flat_texture(renderer: &Renderer, color: [f32; 4]) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = renderer.device.create_texture_with_data(&renderer.queue, &wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsage::COPY_DST | wgpu::TextureUsage::SAMPLED,
    }, &color.iter().map(|c| (c * 255.).floor() as u8).collect::<Vec<_>>().as_slice());
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        format: Some(wgpu::TextureFormat::Rgba8Unorm),
        dimension: Some(wgpu::TextureViewDimension::D2),
        aspect: wgpu::TextureAspect::All,
        base_mip_level: 0,
        mip_level_count: None,
        base_array_layer: 0,
        array_layer_count: None
    });

    (texture, view)
}
fn load_texture(renderer: &Renderer, path: &str) -> (wgpu::Texture, wgpu::TextureView) {
    println!("Loading texture {}", path);
    let image = image::io::Reader::open(path).unwrap().decode().unwrap().to_rgba8();

    let texture = renderer.device.create_texture_with_data(&renderer.queue, &wgpu::TextureDescriptor {
        label: Some(path),
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

    (texture, view)
}

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
            m.0.set_znear_and_zfar(0.1, 2000.);
            m
        }),
        is_enabled: true,
    }, {
        let mut transform = TransformComponent::default();
        transform.position.y = 200.;
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
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float {
                            filterable: true,
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false
                    },
                    count: None
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false
                    },
                    count: None
                },
            ]
        }]
    );

    let (models, materials) = tobj::load_obj("resources/crytek-sponza-huge-vray-obj/crytek-sponza-huge-vray.obj", &tobj::LoadOptions {
        single_index: true,
        triangulate: true,
        ignore_points: true,
        ignore_lines: true,
    }).unwrap();
    let materials = materials.unwrap();
    println!("Loading {} models and {} materials", models.len(), materials.len());

    let material_refs = materials.iter()
        .map(|material| {
            let (_, view) = match material.diffuse_texture.as_str() {
                "" => create_flat_texture(&renderer, [material.diffuse[0], material.diffuse[1], material.diffuse[2], 1.]),
                p => load_texture(&renderer, &("resources/crytek-sponza-huge-vray-obj/".to_string()+p))
            };
            let sampler = renderer.device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            renderer.create_material(shader, &[&[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(uniform_buffer.as_entire_buffer_binding()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
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
                },
                wgpu::VertexBufferLayout {
                    array_stride: 2 * std::mem::size_of::<f32>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &[wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 0,
                        shader_location: 2
                    }]
                }
            ], Some(wgpu::Face::Front))
        })
        .collect::<Vec<_>>();

    let _ = models.iter()
        .map(|model| renderer.create_mesh(material_refs[model.mesh.material_id.unwrap()], &model.mesh.indices, &[
            &model.mesh.positions,
            &model.mesh.normals,
            &model.mesh.texcoords
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
                    t.rotation = UnitQuaternion::from_euler_angles(0., start.elapsed().as_secs_f32() / 5., 0.);
                    t.position.x = (start.elapsed().as_secs_f32() / 5.).cos() * 1000.;
                }
                renderer.render(&world);
            }

            _ => {}
        }
    });
}
