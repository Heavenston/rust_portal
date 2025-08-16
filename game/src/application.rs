use std::{collections::HashMap, iter::repeat_n};

use engine::input::{CursorGrabMode, InputButton, KeyCode};
use pgk::color::{ LinearRgb, LinearRgba, Srgb, Srgba };
use utils::*;

use glam::{ Affine3A, Mat4, Vec2, Vec3, Vec3A };
use itertools::Itertools;
use crevice::std140::AsStd140 as _;
use winit::event::MouseButton;

static SCENE_BYTES: &[u8] = include_bytes!("../../resources/simple_scene.glb");
// static SCENE_BYTES: &[u8] = include_bytes!("../../resources/just-sun.glb");
// static SCENE_BYTES: &[u8] = include_bytes!("../../resources/outdoor_scene.glb");

const MOVEMENT_SPEED: f32 = 8.;
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

#[derive(Default)]
struct BatchingStaticMesh {
    positions: Vec<Vec3>,
    texcoords: Vec<Vec2>,
    normals: Vec<Vec3>,
    indices: Vec<u32>,
}

struct GltfLoadingData<'a> {
    state: &'a mut engine::EngineState,
    gltf_buffers: Vec<gltf::buffer::Data>,
    gltf_textures: Vec<gltf::image::Data>,
    created_materials: HashMap<Option<usize>, engine::MaterialInstance>,
    batched_static_meshes: HashMap<Option<usize>, BatchingStaticMesh>,
}

pub struct Application {
    camera: Camera,
}

impl Application {
    pub fn new(state: &mut engine::EngineState) -> Self {
        let mut this = Application {
            camera: default(),
        };
        this.init(state);
        this
    }

    fn init(&mut self, state: &mut engine::EngineState) {
        println!("Loading scene...");
        let (document, gltf_buffers, gltf_textures) = gltf::import_slice(SCENE_BYTES).expect("Could not load scene");

        let mut data = GltfLoadingData {
            state,
            gltf_buffers,
            gltf_textures,

            created_materials: default(),
            batched_static_meshes: default(),
        };
        println!("Creating materials...");
        for material in document.materials() {
            self.load_gltf_material(&mut data, material);
        }
        println!("Creating meshes...");
        for scene in document.scenes() {
            for node in scene.nodes() {
                self.load_gltf_node(&mut data, Mat4::default(), node);
            }
        }
        for (material_index, batching_mesh) in &data.batched_static_meshes {
            let Some(&material_instance) = data.created_materials.get(material_index)
            else {
                println!("Skipped a mesh with not material\n");
                continue;
            };
            self.commit_batching_mesh(data.state, material_instance, batching_mesh);
        }
        println!("Done loading");

        println!("Loaded: {} static meshes", state.static_meshes().len());
        println!("Loaded: {} directional lights", state.directional_lights().len());
        println!("Loaded: {} spot lights", state.spot_lights().len());
    }

    fn upload_texture(
        &mut self,
        kernel: &mut pgk::GraphicsKernel,
        image_data: &gltf::image::Data,
    ) -> pgk::TextureHandle {
        let texture_handle = kernel.create_texture()
            .usages(pgk::TextureUsages {
                copy_dst: true,
                texture_binding: true,
                ..default()
            })
            .width(image_data.width).height(image_data.height)
            .format(pgk::TextureFormat::Rgba8UnormSrgb)
            .create();

        let data = match image_data.format {
            gltf::image::Format::R8G8B8 => {
                image_data.pixels.iter().copied()
                    .array_chunks::<3>()
                    .map(|[r, g, b]| [r, g, b, 255])
                    .flatten()
                    .collect_vec()
            },
            gltf::image::Format::R8G8B8A8 => {
                image_data.pixels.clone()
            },

            _ => panic!("Unsupported image gltf format '{:?}'", image_data.format),
        };

        kernel.write_texture(texture_handle, &data);

        texture_handle
    }

    fn load_gltf_material(
        &mut self,
        data: &mut GltfLoadingData,
        gltf_material: gltf::Material,
    ) -> engine::MaterialInstance {
        let state = &mut data.state;

        let material_index = gltf_material.index();
        let bmr = gltf_material.pbr_metallic_roughness();

        if let Some(&instance) = data.created_materials.get(&material_index) {
            return instance;
        }

        let base_color_texture = bmr.base_color_texture()
            .map(|diffuse_texture| self.upload_texture(&mut state.kernel, &data.gltf_textures[diffuse_texture.texture().index()]));

        let material = state.materials.get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: base_color_texture.is_some(),
        });
        let material_uniform_buffer = state.kernel.create_buffer()
            .data(&engine::pbr_material::MaterialUniforms {
                base_color: LinearRgba::from_array(bmr.base_color_factor()),
                metallic: bmr.metallic_factor(),
                roughness: bmr.roughness_factor(),
            }.as_std140())
            .create();
        let pipeline_data = state.kernel.get_pipeline_data(state.materials.get(material).pipeline)
            .unwrap();
        let layout = pipeline_data.bind_group_layouts[2];

        let mut bind_group = state.kernel.create_bind_group()
            .label(format!("GLTF Material{} bind group", gltf_material.name().map(|n| format!(" '{n}'")).unwrap_or_default()))
            .layout(layout)
            .entry(0, material_uniform_buffer);
        if let Some(base_color_texture) = base_color_texture {
            bind_group = bind_group.entry(1, base_color_texture).sampler(2);
        }
        let bind_group = bind_group.create();
        let instance = engine::MaterialInstance { material, bind_group };
        data.created_materials.insert(material_index, instance);
        instance
    }

    fn load_gltf_primitive(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        primitive: gltf::Primitive,
    ) {
        let reader = primitive.reader(|buffer| data.gltf_buffers.get(buffer.index()).map(|p| &**p));
        let batched_static_mesh = data.batched_static_meshes.entry(primitive.material().index()).or_default();

        debug_assert_eq!(batched_static_mesh.positions.len(), batched_static_mesh.texcoords.len());
        debug_assert_eq!(batched_static_mesh.texcoords.len(), batched_static_mesh.normals.len());
        let indices_offset = batched_static_mesh.positions.len() as u32;

        let vertex_count = reader.read_positions().unwrap().count();

        batched_static_mesh.positions.extend(
            reader.read_positions().expect("Mesh without positions?")
            .map(Vec3::from_array)
            .map(|point| transform.transform_point3(point))
        );
        if let Some(tex_coords) = reader.read_tex_coords(0) {
            batched_static_mesh.texcoords.extend(
                tex_coords.into_f32().map(Vec2::from_array)
            );
        }
        else {
            batched_static_mesh.texcoords.extend(
                repeat_n(Vec2::ZERO, vertex_count)
            );
        }
        batched_static_mesh.normals.extend(
            reader.read_normals().expect("Mesh without normals?")
            .map(Vec3::from_array)
            .map(|point| transform.transform_vector3(point))
        );
        batched_static_mesh.indices.extend(
            reader.read_indices().expect("Mesh without indices?")
            .into_u32()
            .map(|idx| idx + indices_offset)
        );
    }

    fn load_gltf_mesh(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        mesh: gltf::Mesh,
    ) {
        for primitive in mesh.primitives() {
            self.load_gltf_primitive(data, transform, primitive);
        }
    }

    fn load_gltf_light(
        &mut self,
        data: &mut GltfLoadingData,
        transform: Affine3A,
        light: gltf::khr_lights_punctual::Light,
    ) {
        let position: Vec3 = transform.translation.into();
        let direction = transform.transform_vector3(Vec3::NEG_Z);
        // FIXME: Should not be hard coded ?
        // let intensity = 1.;
        let intensity = light.intensity() * 0.001;
        let color: Srgb = LinearRgb::from_array(light.color()).into();

        use gltf::khr_lights_punctual::Kind;
        match light.kind() {
            Kind::Directional => {
                data.state.insert_directional_light(engine::DirectionalLight {
                    direction,
                    intensity,
                    color,
                });
            },
            Kind::Point => {
                panic!("Unsupported point light!");
            },
            Kind::Spot { inner_cone_angle, outer_cone_angle } => {
                data.state.insert_spot_light(engine::SpotLight {
                    position,
                    direction,
                    intensity,
                    color,
                    inner_cone_angle,
                    outer_cone_angle,
                });
            },
        }

    }

    fn commit_batching_mesh(
        &mut self,
        state: &mut engine::EngineState,
        material_instance: engine::MaterialInstance,
        batching_mesh: &BatchingStaticMesh,
    ) {
        let positions_buffer = state.kernel.create_buffer()
            .slice(&batching_mesh.positions)
            .create();
        let texcoords_buffer = state.kernel.create_buffer()
            .slice(&batching_mesh.texcoords)
            .create();
        let normals_buffer = state.kernel.create_buffer()
            .slice(&batching_mesh.normals)
            .create();
        let index_buffer = state.kernel.create_buffer()
            .slice(&batching_mesh.indices)
            .create();

        state.insert_static_mesh(engine::StaticMesh {
            transform: default(),
            positions_buffer,
            texcoords_buffer,
            normals_buffer,
            index_buffer,
            vertex_count: batching_mesh.indices.len().try_into().expect("No overflow"),
            material_instance,
        }.into());
    }

    fn load_gltf_node(
        &mut self,
        data: &mut GltfLoadingData,
        parent_transform: Mat4,
        node: gltf::Node,
    ) {
        let local_transform = Mat4::from_cols_array(
            &flatten_array::<f32, 4, 4>(node.transform().matrix())
        );
        let global_transform = parent_transform * local_transform;
        let global_affine = Affine3A::from_mat4(global_transform);

        if let Some(light) = node.light() {
            self.load_gltf_light(data, global_affine, light);
        }

        if let Some(mesh) = node.mesh() {
            self.load_gltf_mesh(data, global_affine, mesh);
        }

        if let Some(camera) = node.camera() &&
           let gltf::camera::Projection::Perspective(perspective) = camera.projection()
        {
            self.camera = Camera {
                transform: global_affine,
                fov: perspective.yfov(),
                znear: perspective.znear(),
                zfar: perspective.zfar(),
            };
        }

        for child_node in node.children() {
            self.load_gltf_node(data, global_transform, child_node);
        }
    }
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

