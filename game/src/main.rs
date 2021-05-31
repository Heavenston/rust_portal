use std::{borrow::Cow, convert::TryInto, f32, path::PathBuf, str::FromStr, time::Instant};

use imgui::im_str;
use nalgebra::UnitQuaternion;
use portal_engine::{
    camera::{CameraComponent, PerspectiveCameraMatrix},
    renderer::{MeshComponent, Renderer, Texture},
    resource_manager::ResourceManager,
    transform::TransformComponent,
};
use rayon::prelude::*;
use wgpu::util::DeviceExt;
use winit::dpi::LogicalSize;

fn main() {
    let mut world = hecs::World::new();
    let mut imgui_ctx = imgui::Context::create();
    let mut imgui_platform = imgui_winit_support::WinitPlatform::init(&mut imgui_ctx);

    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();
    window.set_title("Portal !");
    window.set_inner_size(LogicalSize::new(500, 500));
    window.set_min_inner_size(Some(LogicalSize::new(100, 100)));
    imgui_platform.attach_window(
        imgui_ctx.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );

    println!("Creating renderer...");
    let mut renderer = Box::new(pollster::block_on(Renderer::new(
        &window,
        100,
        100,
        &mut imgui_ctx,
    )));
    println!("Created renderer");

    let resource_manager = ResourceManager::new();

    let camera_entity = world.spawn((
        CameraComponent {
            clear_color: Some(wgpu::Color {
                r: 88. / 255.,
                g: 101. / 255.,
                b: 242. / 255.,
                a: 1.,
            }),
            matrix: Box::new({
                let mut m = PerspectiveCameraMatrix::new();
                m.0.set_znear_and_zfar(0.1, 2000.);
                m
            }),
            is_enabled: true,
        },
        {
            let mut transform = TransformComponent::default();
            transform.position.y = 200.;
            transform
        },
    ));

    let uniform_buffer = renderer
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&[0.0f32, 1., 0., 1.]),
            usage: wgpu::BufferUsage::UNIFORM,
        });

    let shader_m = wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        flags: Default::default(),
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
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStage::FRAGMENT,
                    ty: wgpu::BindingType::Sampler {
                        filtering: true,
                        comparison: false,
                    },
                    count: None,
                },
            ],
        }],
        &[
            wgpu::VertexBufferLayout {
                array_stride: 3 * std::mem::size_of::<f32>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 3 * std::mem::size_of::<f32>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 1,
                }],
            },
            wgpu::VertexBufferLayout {
                array_stride: 2 * std::mem::size_of::<f32>() as u64,
                step_mode: wgpu::InputStepMode::Vertex,
                attributes: &[wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 2,
                }],
            },
        ],
    );

    let (models, materials) = tobj::load_obj(
        "resources/crytek-sponza-huge-vray-obj/crytek-sponza-huge-vray.obj",
        &tobj::LoadOptions {
            single_index: true,
            triangulate: true,
            ignore_points: true,
            ignore_lines: true,
        },
    )
    .unwrap();
    let materials = materials.unwrap();
    println!(
        "Loading {} models and {} materials",
        models.len(),
        materials.len()
    );

    let material_refs = materials
        .par_iter()
        .map(|material| {
            let mut texture = if material.diffuse_texture != "" {
                resource_manager
                    .load_texture_from_file(
                        &renderer,
                        &PathBuf::from_str("resources/crytek-sponza-huge-vray-obj")
                            .unwrap()
                            .join(&material.diffuse_texture),
                    )
                    .unwrap()
            }
            else {
                Texture::create_plain_color_texture(
                    &renderer,
                    image::Rgba::<u8>(
                        material
                            .diffuse
                            .iter()
                            .map(|a| (a * 255.) as u8)
                            .chain(std::iter::once(255u8))
                            .collect::<Vec<_>>()
                            .try_into()
                            .unwrap(),
                    ),
                    Some(&material.name),
                )
            };
            texture.create_sampler(
                &renderer,
                wgpu::AddressMode::Repeat,
                wgpu::FilterMode::Linear,
                wgpu::FilterMode::Linear,
            );

            renderer.create_material(
                shader,
                &[&[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer(
                            uniform_buffer.as_entire_buffer_binding(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(texture.sampler.as_ref().unwrap()),
                    },
                ]],
                Some(wgpu::Face::Front),
            )
        })
        .collect::<Vec<_>>();

    let _ = world
        .spawn_batch(
            models
                .par_iter()
                .map(|model| {
                    renderer.create_mesh(
                        material_refs[model.mesh.material_id.unwrap()],
                        &model.mesh.indices,
                        &[
                            &model.mesh.positions,
                            &model.mesh.normals,
                            &model.mesh.texcoords,
                        ],
                    )
                })
                .map(|mesh| (MeshComponent(mesh), TransformComponent::default()))
                .collect::<Vec<_>>()
                .into_iter(),
        )
        .collect::<Vec<_>>();

    let start = Instant::now();
    let mut last_frame = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        use winit::{
            event::{Event, WindowEvent},
            event_loop::ControlFlow,
        };

        *control_flow = ControlFlow::Poll;
        match event {
            Event::NewEvents(_) => {
                imgui_ctx.io_mut().update_delta_time(last_frame.elapsed());
                last_frame = Instant::now();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                imgui_platform.handle_event(imgui_ctx.io_mut(), &window, &event);
                renderer.resize(size.width, size.height);

                let mut camera_component = world.get_mut::<CameraComponent>(camera_entity).unwrap();
                let matrix: &mut Box<PerspectiveCameraMatrix> =
                    unsafe { std::mem::transmute(&mut camera_component.matrix) };
                matrix.0.set_aspect(size.width as f32 / size.height as f32);
            }

            Event::MainEventsCleared => {
                imgui_platform
                    .prepare_frame(imgui_ctx.io_mut(), &window)
                    .expect("Failed to prepare frame");
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                let ui = imgui_ctx.frame();

                let mut is_vsync_enabled = renderer.get_vsync();
                imgui::Window::new(im_str!("Performances"))
                    .size([300.0, 110.0], imgui::Condition::FirstUseEver)
                    .build(&ui, || {
                        ui.text(format!("FPS: {}", ui.io().framerate));
                        ui.text(format!(
                            "Meshes: {}",
                            world.query_mut::<&MeshComponent>().into_iter().count()
                        ));
                        ui.separator();
                        ui.checkbox(im_str!("Enable VSYNC"), &mut is_vsync_enabled);
                    });
                renderer.set_vsync(is_vsync_enabled);

                {
                    let mut t = world.get_mut::<TransformComponent>(camera_entity).unwrap();
                    t.rotation = UnitQuaternion::from_euler_angles(
                        0.,
                        start.elapsed().as_secs_f32() / 5.,
                        0.,
                    );
                    t.position.x = (start.elapsed().as_secs_f32() / 5.).cos() * 1000.;
                }

                imgui_platform.prepare_render(&ui, &window);
                renderer.render(ui.render(), &world);
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,

            event => {
                imgui_platform.handle_event(imgui_ctx.io_mut(), &window, &event);
            }
        }
    });
}
