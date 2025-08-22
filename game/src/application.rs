use engine::input::{CursorGrabMode, InputButton, KeyCode};
use pgk::color::{Srgb, Srgba};
use pgk::color::LinearRgba;
use utils::prelude::*;

use glam::{ Affine3A, IVec3, Mat4, Vec3, Vec3A };
use winit::event::MouseButton;
use crevice::std140::AsStd140;

mod physics;
use physics::*;

mod maps;

// const MOVEMENT_SPEED: f32 = 150.;
const DEFAULT_MOVEMENT_SPEED: f32 = 5.;
const MOVEMENT_SPEED_SCROLL_CHANGE: f32 = 1.3;
const LOOK_SPEED: f32 = 0.0008;
const FIXED_TIMESTEP: f32 = 1.0 / 20.0; // 20 Hz physics

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
            znear: 0.1,
            zfar: Some(1000.),
        }
    }
}

pub struct Application {
    camera: Camera,
    movement_speed: f32,
    map_mesher: maps::MapMesher,
    current_map: maps::Map,
    spot_light: engine::SpotLightHandle,
    physics: PhysicsWorld,
    player_body: rapier3d::prelude::RigidBodyHandle,
    physics_accumulator: f32,
    wish_vel: Vec3A,
    player_prev_pos: Vec3,
    player_curr_pos: Vec3,
    cubes: Vec<CubeEntity>,
}

struct CubeEntity {
    body: rapier3d::prelude::RigidBodyHandle,
    mesh: engine::MeshHandle,
    prev_pos: Vec3,
    prev_rot: glam::Quat,
    curr_pos: Vec3,
    curr_rot: glam::Quat,
}

impl CubeEntity {
    fn new(body: rapier3d::prelude::RigidBodyHandle, mesh: engine::MeshHandle, pos: Vec3) -> Self {
        let rot = glam::Quat::IDENTITY;
        Self {
            body,
            mesh,
            prev_pos: pos,
            prev_rot: rot,
            curr_pos: pos,
            curr_rot: rot,
        }
    }
}

impl Application {
    pub fn new(state: &mut engine::EngineState) -> Self {
        let mut default_map = maps::Map::default();

        let height = 3i32;
        let width = 3i32;

        default_map.clear(
            IVec3::new(0,     0,      0),
            IVec3::new(width, height, width),
            AxisDirection::BITS_ALL,
            maps::MapCellMaterial::Metal,
        );
        default_map.set_air_cell_walls(
            IVec3::new(0, 0, 0),
            AxisDirection::BITS_ALL,
            maps::MapCellMaterial::Concrete,
        );
        default_map.clear(
            IVec3::new(0,     height-1,      0),
            IVec3::new(width, height  , width),
            AxisDirection::BITS_ALL,
            maps::MapCellMaterial::Concrete,
        );

        // Add a tunnel
        default_map.clear(
            IVec3::new(width/2, 0, -5),
            IVec3::new(width/2, 0, -1),
            AxisDirection::BITS_ALL,
            maps::MapCellMaterial::Metal,
        );
        default_map.clear(
            IVec3::new(width/2, 0, -5),
            IVec3::new(width/2, 0, -1),
            AxisDirection::PosY | AxisDirection::NegY,
            maps::MapCellMaterial::Concrete,
        );

        let mut physics = PhysicsWorld::new();

        // Spawn player dynamic capsule body
        let player_body = physics.spawn_player(glam::Vec3::new(1.5, 1.8, 1.5));

        // Spawn a pyramid of five 0.7m cubes at the middle of the map (not at origin)
        let half = 0.35;
        let side = 0.7;
        let spacing = 0.02;
        let center_xz = glam::Vec3::new(
            (width as f32) * maps::WORLD_SCALE / 2.0,
            0.0,
            (width as f32) * maps::WORLD_SCALE / 2.0,
        );
        let mut cubes_vec: Vec<CubeEntity> = Vec::new();
        // Base layer: three cubes along X
        for ix in -1..=1 {
            let x = ix as f32 * (side + spacing);
            let y = half;
            let pos = center_xz + glam::Vec3::new(x, y, 0.0);
            let body = physics.spawn_cube(pos, glam::Vec3::splat(half), 20.0);
            let mesh = Self::create_cube_mesh(state, glam::Vec3::splat(half), LinearRgba::new(0.9, 0.6, 0.2, 1.0), Affine3A::from_translation(pos));
            cubes_vec.push(CubeEntity::new(body, mesh, pos));
        }
        // Second layer: two cubes between base cubes
        for ix in [-1, 1] {
            let x = ix as f32 * (0.5 * (side + spacing));
            let y = half + (side + spacing);
            let pos = center_xz + glam::Vec3::new(x, y, 0.0);
            let body = physics.spawn_cube(pos, glam::Vec3::splat(half), 20.0);
            let mesh = Self::create_cube_mesh(state, glam::Vec3::splat(half), LinearRgba::new(0.9, 0.6, 0.2, 1.0), Affine3A::from_translation(pos));
            cubes_vec.push(CubeEntity::new(body, mesh, pos));
        }

        let mut this = Application {
            camera: default(),
            movement_speed: DEFAULT_MOVEMENT_SPEED,
            map_mesher: default(),
            current_map: default_map,
            spot_light: default(),
            physics,
            player_body,
            physics_accumulator: 0.0,
            wish_vel: Vec3A::ZERO,
            player_prev_pos: Vec3::ZERO,
            player_curr_pos: Vec3::ZERO,
            cubes: cubes_vec,
        };
        this.init(state).expect("Could not init");
        // Initialize interpolation buffers from current physics position
        if let Some(p) = this.physics.body_position(this.player_body) {
            this.player_prev_pos = p;
            this.player_curr_pos = p;
        }
        this
    }

    fn init(&mut self, state: &mut engine::EngineState) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading scene...");

        let map_mesh = self.map_mesher.mesh(&self.current_map);
        map_mesh.upload(&mut self.map_mesher, state);
        // Build physics collider from the already-generated render mesh
        self.physics.rebuild_map_collider_from_model(&map_mesh);

        self.spot_light = state.insert_spot_light(engine::SpotLight {
            position: default(),
            intensity: 15.,
            color: Srgb::WHITE,
            direction: Vec3::NEG_Z,
            inner_cone_angle: self.camera.fov * 0.5,
            outer_cone_angle: self.camera.fov * 0.6,
        });

        println!("Loaded: {} static meshes", state.meshes().len());
        println!("Loaded: {} directional lights", state.directional_lights().len());
        println!("Loaded: {} spot lights", state.spot_lights().len());

        Ok(())
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
        // Only move on the XZ plane; Y handled by physics (gravity/jump)
        input_vector.y = 0.0;

        // Normalize and scale by speed
        let wish_dir = input_vector.normalize_or_zero();
        let wish_vel = wish_dir * self.movement_speed;
        self.wish_vel = wish_vel;

        // Jump when grounded
        if state.input.just_pressed(KeyCode::Space) {
            self.physics.request_jump();
        }

        // Fixed-step physics at 20 Hz
        self.physics_accumulator += dt;
        while self.physics_accumulator >= FIXED_TIMESTEP {
            self.fixed_step(FIXED_TIMESTEP);
            self.physics_accumulator -= FIXED_TIMESTEP;
        }

        // Interpolate camera position between physics frames
        let alpha = (self.physics_accumulator / FIXED_TIMESTEP).clamp(0.0, 1.0);
        let interp_pos = self.player_prev_pos.lerp(self.player_curr_pos, alpha);
        let eye_height = physics::PLAYER_EYE_HEIGHT;
        self.camera.transform.translation = (interp_pos + glam::Vec3::Y * eye_height).into();

        // Update dynamic cube meshes transforms using interpolation
        for cube in &self.cubes {
            let alpha = (self.physics_accumulator / FIXED_TIMESTEP).clamp(0.0, 1.0);
            let pos = cube.prev_pos.lerp(cube.curr_pos, alpha);
            let rot = cube.prev_rot.slerp(cube.curr_rot, alpha);
            state.set_mesh_transform(cube.mesh, Affine3A::from_rotation_translation(rot, pos));
        }

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
            self.movement_speed /= MOVEMENT_SPEED_SCROLL_CHANGE;
            println!("Movement speed: {}", self.movement_speed);
        }
        if state.input.just_pressed(InputButton::MouseWheelUp) {
            self.movement_speed *= MOVEMENT_SPEED_SCROLL_CHANGE;
            println!("Movement speed: {}", self.movement_speed);
        }

        if state.input.just_pressed(KeyCode::KeyP) {
            let looking_at = self.camera.transform.transform_vector3(Vec3::NEG_Z);
            let axis = looking_at.abs().max_axis();
            let direction = axis.with_sign(looking_at[axis].strict_sign());
            println!("Looking towards: {direction:?}");
        }

        state.camera = Some(engine::Camera {
            transform: self.camera.transform,
            projection: self.camera.get_projection(state.kernel.aspect_ration()),
            clear_color: Srgba::BLACK,
        });

        let spot_light = state.spot_light_mut(self.spot_light).expect("pl");
        spot_light.position = self.camera.transform.translation.into();
        spot_light.direction = forward_vector.into();
    }
}

impl Application {
    fn fixed_step(&mut self, dt: f32) {
        // Drive character using the last computed desired velocity
        self.physics.drive_character(self.player_body, self.wish_vel, dt);
        self.physics.step(dt);
        // Update interpolation state
        if let Some(p) = self.physics.body_position(self.player_body) {
            self.player_prev_pos = self.player_curr_pos;
            self.player_curr_pos = p;
        }
        // Update cubes interpolation state from physics bodies
        for cube in &mut self.cubes {
            if let Some(rb) = self.physics.bodies().get(cube.body) {
                let t = rb.translation();
                let r = rb.rotation();
                let pos = glam::Vec3::new(t.x, t.y, t.z);
                let rot = glam::Quat::from_xyzw(r.i, r.j, r.k, r.w);
                cube.prev_pos = cube.curr_pos;
                cube.prev_rot = cube.curr_rot;
                cube.curr_pos = pos;
                cube.curr_rot = rot;
            }
        }
    }
}

impl Application {
    fn create_cube_mesh(state: &mut engine::EngineState, half: glam::Vec3, base_color: LinearRgba, transform: Affine3A) -> engine::MeshHandle {
        // 24 vertices (4 per face) so we can have proper normals/UVs per face
        let hx = half.x; let hy = half.y; let hz = half.z;
        let mut positions = Vec::<glam::Vec3>::new();
        let mut normals = Vec::<glam::Vec3>::new();
        let mut uvs = Vec::<glam::Vec2>::new();
        let mut indices = Vec::<u32>::new();

        // Build faces using the same conventions as maps::mesher::create_plane
        let pos = glam::Vec3::new(-hx, -hy, -hz);
        let size = glam::Vec3::new(hx * 2.0, hy * 2.0, hz * 2.0);

        let mut add_plane = |dir: AxisDirection| {
            let base = positions.len() as u32;
            match dir {
                AxisDirection::PosX => {
                    let x = pos.x + size.x;
                    positions.extend([
                        glam::Vec3::new(x, pos.y,          pos.z),
                        glam::Vec3::new(x, pos.y + size.y, pos.z),
                        glam::Vec3::new(x, pos.y + size.y, pos.z + size.z),
                        glam::Vec3::new(x, pos.y,          pos.z + size.z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(size.z, size.y),
                        glam::Vec2::new(size.z, 0.0),
                        glam::Vec2::new(0.0,    0.0),
                        glam::Vec2::new(0.0,    size.y),
                    ]);
                },
                AxisDirection::NegX => {
                    let x = pos.x;
                    positions.extend([
                        glam::Vec3::new(x, pos.y + size.y, pos.z),
                        glam::Vec3::new(x, pos.y,          pos.z),
                        glam::Vec3::new(x, pos.y,          pos.z + size.z),
                        glam::Vec3::new(x, pos.y + size.y, pos.z + size.z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(0.0,    0.0),
                        glam::Vec2::new(0.0,    size.y),
                        glam::Vec2::new(size.z, size.y),
                        glam::Vec2::new(size.z, 0.0),
                    ]);
                },
                AxisDirection::PosY => {
                    let y = pos.y + size.y;
                    positions.extend([
                        glam::Vec3::new(pos.x,          y, pos.z),
                        glam::Vec3::new(pos.x,          y, pos.z + size.z),
                        glam::Vec3::new(pos.x + size.x, y, pos.z + size.z),
                        glam::Vec3::new(pos.x + size.x, y, pos.z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(0.0,    0.0   ),
                        glam::Vec2::new(0.0,    size.z),
                        glam::Vec2::new(size.x, size.z),
                        glam::Vec2::new(size.x, 0.0   ),
                    ]);
                },
                AxisDirection::NegY => {
                    let y = pos.y;
                    positions.extend([
                        glam::Vec3::new(pos.x,          y, pos.z),
                        glam::Vec3::new(pos.x + size.x, y, pos.z),
                        glam::Vec3::new(pos.x + size.x, y, pos.z + size.z),
                        glam::Vec3::new(pos.x,          y, pos.z + size.z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(0.0,    0.0),
                        glam::Vec2::new(size.x, 0.0),
                        glam::Vec2::new(size.x, size.z),
                        glam::Vec2::new(0.0,    size.z),
                    ]);
                },
                AxisDirection::PosZ => {
                    let z = pos.z + size.z;
                    positions.extend([
                        glam::Vec3::new(pos.x,          pos.y,          z),
                        glam::Vec3::new(pos.x + size.x, pos.y,          z),
                        glam::Vec3::new(pos.x + size.x, pos.y + size.y, z),
                        glam::Vec3::new(pos.x,          pos.y + size.y, z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(0.0,    size.y),
                        glam::Vec2::new(size.x, size.y),
                        glam::Vec2::new(size.x, 0.0),
                        glam::Vec2::new(0.0,    0.0),
                    ]);
                },
                AxisDirection::NegZ => {
                    let z = pos.z;
                    positions.extend([
                        glam::Vec3::new(pos.x,          pos.y + size.y, z),
                        glam::Vec3::new(pos.x + size.x, pos.y + size.y, z),
                        glam::Vec3::new(pos.x + size.x, pos.y,          z),
                        glam::Vec3::new(pos.x,          pos.y,          z),
                    ]);
                    uvs.extend([
                        glam::Vec2::new(size.x, 0.0),
                        glam::Vec2::new(0.0,    0.0),
                        glam::Vec2::new(0.0,    size.y),
                        glam::Vec2::new(size.x, size.y),
                    ]);
                },
            }
            normals.extend([dir.as_ivec3().as_vec3(); 4]);
            indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
        };

        for dir in AxisDirection::VARIANTS {
            add_plane(dir);
        }

        // Compute tangents
        let tangents = pgk::utils::compute_all_tangents(&positions, &uvs, &indices);

        // Buffers
        let positions_buffer = state.kernel.create_buffer().slice(&positions).create();
        let texcoords_buffer = state.kernel.create_buffer().slice(&uvs).create();
        let normals_buffer = state.kernel.create_buffer().slice(&normals).create();
        let tangents_buffer = state.kernel.create_buffer().slice(&tangents.tangents).create();
        let bitangents_buffer = state.kernel.create_buffer().slice(&tangents.bitangents).create();
        let index_buffer = state.kernel.create_buffer().slice(&indices).create();

        // Material (simple PBR, no textures)
        let material_uniform_buffer = state.kernel.create_buffer()
            .data(&engine::pbr_material::MaterialUniforms {
                base_color: base_color,
                metallic: 0.0,
                roughness: 0.9,
            }.as_std140())
            .create();
        let material = state.materials.get_handle(&mut state.kernel, &engine::pbr_material::Parameters {
            enable_base_color_texture: false,
            enable_normal_map_texture: false,
            unlit: false,
        });
        let material_data = state.materials.get(material);
        let pipeline_data = state.kernel.get_pipeline_data(material_data.pipeline).expect("pipeline");
        let bind_group_layout = pipeline_data.bind_group_layouts[2];
        let bind_group = state.kernel.create_bind_group()
            .layout(bind_group_layout)
            .entry(0, material_uniform_buffer)
            .create();
        let material_instance = engine::MaterialInstance { material, bind_group };

        state.insert_mesh(engine::Mesh {
            transform,
            vertex_buffers: vec![
                (0, positions_buffer),
                (1, texcoords_buffer),
                (2, normals_buffer),
                (3, tangents_buffer),
                (4, bitangents_buffer),
            ].into_boxed_slice(),
            index_buffer,
            vertex_count: indices.len().try_into().expect("no overflow"),
            material_instance,
        })
    }
}
