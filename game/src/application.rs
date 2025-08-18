use std::{ iter::empty, path::PathBuf, sync::LazyLock };

use engine::input::{CursorGrabMode, InputButton, KeyCode};
use pgk::color::{ Srgba };
use utils::{ itertools::Itertools as _, * };

use glam::{ Affine3A, Mat4, Vec3, Vec3A };
use winit::event::MouseButton;

use crate::bsp_loader::create_bsp_meshes;

static PORTAL_GAME_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    std::env::var("PORTAL_GAME_PATH")
        .expect("Could not find env variable PORTAL_GAME_PATH")
        .into()
});
const MOVEMENT_SPEED: f32 = 100.;
const LOOK_SPEED: f32 = 0.0008;

#[derive(Debug, Clone, Copy)]
struct Camera {
    transform: Affine3A,
    fov: f32,
    znear: f32,
    zfar: Option<f32>,
}

impl Camera {
    fn get_projection(&self, aspect_ratio: f32) -> Mat4 {
        if let Some(zfar) = self.zfar {
            Mat4::perspective_rh(self.fov, aspect_ratio, self.znear, zfar)
        } else {
            Mat4::perspective_infinite_rh(self.fov, aspect_ratio, self.znear)
        }
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            transform: default(),
            fov: (90f32).to_radians(),
            znear: 0.001,
            zfar: None,
        }
    }
}

fn fan_triangulate(mut vertices: impl Iterator<Item = Vec3>) -> impl Iterator<Item = Vec3> {
    let Some(first) = vertices.next()
    else { return itertools::Either::Right(empty()) };

    itertools::Either::Left(
        vertices.tuple_windows()
            .map(move |(a, b)| [first, a, b])
            .flatten()
    )
}

pub struct Application {
    camera: Camera,
}

impl Application {
    pub fn new(state: &mut engine::EngineState) -> Self {
        let mut this = Application {
            camera: default(),
        };
        this.init(state).expect("Could not init");
        this
    }

    fn init(&mut self, state: &mut engine::EngineState) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading scene...");

        let maps_path = PORTAL_GAME_PATH.join("portal/maps/");
        let pak_path = PORTAL_GAME_PATH.join("portal/portal_pak_dir.vpk");

        let map_path = maps_path.join("testchmb_a_04.bsp");
        let bsp = vbsp::Bsp::read(&std::fs::read(&map_path).unwrap()).unwrap();

        println!("Loading map '{}'", map_path.to_str().expect("Valid utf8"));
        create_bsp_meshes(state, &bsp)?;

        println!("Loaded: {} static meshes", state.meshes().len());
        println!("Loaded: {} directional lights", state.directional_lights().len());
        println!("Loaded: {} spot lights", state.spot_lights().len());

        Ok(())
    }

    // fn load_model(&mut self, prefix: &str, state: &mut engine::EngineState, model: &vmdl::Model) -> Result<(), Box<dyn std::error::Error>> {
    //     let positions_buffer: pgk::BufferHandle;
    //     let texcoords_buffer: pgk::BufferHandle;
    //     let normals_buffer: pgk::BufferHandle;
    //     {
    //         let (positions, texcoords, normals): (Vec<_>, Vec<_>, Vec<_>) = model.vertices().iter().map(|vertex| (
    //             Vec3::new(vertex.position.x, vertex.position.y, vertex.position.z),
    //             Vec2::from_array(vertex.texture_coordinates),
    //             Vec3::new(vertex.normal.x, vertex.normal.y, vertex.normal.z),
    //         )).collect();

    //         positions_buffer = state.kernel.create_buffer().slice(&positions).create();
    //         texcoords_buffer = state.kernel.create_buffer().slice(&texcoords).create();
    //         normals_buffer = state.kernel.create_buffer().slice(&normals).create();
    //     }

    //     let material = state.materials.get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
    //         enable_base_color_texture: true,
    //     });
    //     let material_data = state.materials.get(material);
    //     let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline).expect("valid");
    //     let bind_group_layout = pipeline_data.bind_group_layouts[2];

    //     println!("{:#?}", model.textures());

    //     for mesh in model.meshes() {
    //         println!("model {} idx {}", mesh.model_name, mesh.material_index());
    //         let texture = &model.textures()[mesh.material_index() as usize];
    //         let color_texture_handle = self.load_vtf_texture(state, texture.search_paths.iter()
    //         .map(|path| format!("{prefix}materials/{}{}", path.to_lowercase(), texture.name.to_lowercase())))?;

    //         let indices: Vec<u32> = mesh.vertex_strip_indices()
    //             .flatten()
    //             .map(|i| u32::try_from(i).expect("No overflow"))
    //             .collect();

    //         let index_buffer = state.kernel.create_buffer().slice(&indices).create();

    //         let material_uniform = state.kernel.create_buffer()
    //             .data(&engine::pbr_material::MaterialUniforms {
    //                 base_color: LinearRgba::new(1., 1., 1., 1.),
    //                 metallic: 0.,
    //                 roughness: 1.,
    //             }.as_std140())
    //             .create();

    //         let bind_group = state.kernel.create_bind_group()
    //             .layout(bind_group_layout)
    //             .entry(0, material_uniform)
    //             .entry(1, color_texture_handle).sampler(2)
    //             .create();

    //         state.insert_mesh(engine::Mesh {
    //             transform: Affine3A::from_mat4(Mat4::from_cols_array(model.root_transform().as_ref())),
    //             positions_buffer, texcoords_buffer, normals_buffer, index_buffer,
    //             vertex_count: indices.len().try_into().unwrap(),
    //             material_instance: engine::MaterialInstance {
    //                 material, bind_group,
    //             },
    //         });
    //     }

    //     Ok(())
    // }

    // fn load_vtf_texture(
    //     &mut self,
    //     state: &mut engine::EngineState,
    //     paths: impl IntoIterator<Item = String>,
    // ) -> Result<pgk::TextureHandle, Box<dyn std::error::Error>> {
    //     let mut bytes = None::<Vec<u8>>;
    //     for path in paths {
    //         print!("Reading {path} - ");
    //         match std::fs::read(path) {
    //             Ok(b) => {
    //                 println!("Found");
    //                 bytes = Some(b);
    //                 break;
    //             },
    //             Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
    //                 println!("Not Found");
    //                 continue;
    //             },
    //             Err(e) => {
    //                 println!("Error: {e}");
    //                 return Err(e.into());
    //             },
    //         }
    //     }
    //     let Some(bytes) = bytes
    //     else { return Err(format!("Could not find any valid paths").into()) };
    //     let vtf = vtf::from_bytes(&bytes)?;

    //     println!("{} of {}x{}", vtf.highres_image.format, vtf.highres_image.width, vtf.highres_image.height);

    //     let rgba8_data = match vtf.highres_image.format {
    //         // the `vtf` lib seems to break this conversion somehow
    //         vtf::ImageFormat::Bgr888 => vtf.highres_image.get_frame(0)?.iter().copied()
    //             .array_chunks()
    //             .flat_map(|[b, g, r]| [r, g, b, 255])
    //             .collect(),
    //         _ => vtf.highres_image.decode(0)?.into_rgba8().into_vec(),
    //     };

    //     let texture_handle = state.kernel.create_texture()
    //         .width(vtf.highres_image.width.into())
    //         .height(vtf.highres_image.height.into())
    //         .usages(pgk::TextureUsages {
    //             copy_dst: true,
    //             texture_binding: true,
    //             ..default()
    //         })
    //         .format(pgk::TextureFormat::Rgba8UnormSrgb)
    //         .create()
    //     ;
    //     state.kernel.write_texture(texture_handle, &rgba8_data);

    //     Ok(texture_handle)
    // }
}

impl engine::Application for Application {
    fn pre_frame(&mut self, state: &mut engine::EngineState, dt: f32) {
        // println!("Frame {dt}s (~{}fps)!", 1. / dt);
        
        let forward_vector = Vec3A::from(-self.camera.transform.z_axis).normalize();
        let left_vector = Vec3A::from(-self.camera.transform.x_axis).normalize();

        let mut input_vector = Vec3A::ZERO;
        if state.input.pressed(KeyCode::KeyW) {
            input_vector += forward_vector;
        }
        if state.input.pressed(KeyCode::KeyS) {
            input_vector -= forward_vector;
        }
        if state.input.pressed(KeyCode::KeyA) {
            input_vector += left_vector;
        }
        if state.input.pressed(KeyCode::KeyD) {
            input_vector -= left_vector;
        }
        if state.input.pressed(KeyCode::Space) {
            input_vector += Vec3A::Y;
        }

        input_vector = input_vector.normalize_or_zero() * dt * MOVEMENT_SPEED;
        self.camera.transform.translation += input_vector;

        if state.input.just_pressed(MouseButton::Left) {
            if !state.input.cursor_grab_mode().captured {
                state.input.set_cursor_grab_mode(CursorGrabMode::CAPTURED);
            }
            else {
                state.input.set_cursor_grab_mode(CursorGrabMode::VISIBLE);
            }
        }

        if state.input.cursor_grab_mode().captured {
            let mouse_delta = state.input.mouse_motion() * -LOOK_SPEED;

            let (scale, rotation, translation) = self.camera.transform.to_scale_rotation_translation();
            let (ry, rx, rz) = rotation.to_euler(glam::EulerRot::YXZ);
            let rotation = glam::Quat::from_euler(glam::EulerRot::YXZ,
                ry + mouse_delta.x,
                f32::clamp(rx + mouse_delta.y, -std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2),
                rz,
            );
            self.camera.transform = Affine3A::from_scale_rotation_translation(scale, rotation, translation);

            state.input.set_mouse_pos(
                state.kernel.viewport_size().as_vec2() / 2.
            );
        }

        if state.input.just_pressed(InputButton::MouseWheelDown) {
            self.camera.fov *= 1.1;
        }
        if state.input.just_pressed(InputButton::MouseWheelUp) {
            self.camera.fov /= 1.1;
        }

        state.camera = Some(engine::Camera {
            transform: self.camera.transform,
            projection: self.camera.get_projection(state.kernel.aspect_ration()),
            clear_color: Srgba::BLACK,
        });
    }
}

