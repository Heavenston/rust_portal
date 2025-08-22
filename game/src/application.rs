use engine::input::{CursorGrabMode, InputButton, KeyCode};
use pgk::color::{Srgb, Srgba};
use utils::prelude::*;

use glam::{ Affine3A, IVec3, Mat4, Vec3, Vec3A };
use winit::event::MouseButton;

mod physics;
use physics::*;

mod maps;

// const MOVEMENT_SPEED: f32 = 150.;
const DEFAULT_MOVEMENT_SPEED: f32 = 5.;
const MOVEMENT_SPEED_SCROLL_CHANGE: f32 = 1.3;
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

        // Spawn a 2x2x3 cube stack near the player that can be pushed
        physics.spawn_cube_stack(
            glam::Vec3::new(2.5, 0.0, 1.5), // base center on floor
            2, // grid_x
            2, // grid_z
            3, // layers
            glam::Vec3::splat(0.3), // half extents (0.6m cubes)
            0.04, // spacing
            250.0, // density
        );

        let mut this = Application {
            camera: default(),
            movement_speed: DEFAULT_MOVEMENT_SPEED,
            map_mesher: default(),
            current_map: default_map,
            spot_light: default(),
            physics,
            player_body,
        };
        this.init(state).expect("Could not init");
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

        // Jump when grounded
        if state.input.just_pressed(KeyCode::Space) {
            self.physics.request_jump();
        }

        // Apply movement to character body and step physics
        self.physics.drive_character(self.player_body, wish_vel, dt);
        self.physics.step(dt);

        // Update camera position to player head
        if let Some(player_pos) = self.physics.body_position(self.player_body) {
            let eye_height = physics::PLAYER_EYE_HEIGHT;
            self.camera.transform.translation = (player_pos + glam::Vec3::Y * eye_height).into();
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
