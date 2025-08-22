use engine::input::{CursorGrabMode, InputButton, KeyCode};
use pgk::color::{Srgb, Srgba};
use utils::prelude::*;

use glam::{ Affine3A, IVec3, Mat4, Vec3, Vec3A };
use winit::event::MouseButton;

mod maps;

// const MOVEMENT_SPEED: f32 = 150.;
const DEFAULT_MOVEMENT_SPEED: f32 = 25.;
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
    point_light: engine::PointLightHandle,
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

        let mut this = Application {
            camera: default(),
            movement_speed: DEFAULT_MOVEMENT_SPEED,
            map_mesher: default(),
            current_map: default_map,
            point_light: default(),
        };
        this.init(state).expect("Could not init");
        this
    }

    fn init(&mut self, state: &mut engine::EngineState) -> Result<(), Box<dyn std::error::Error>> {
        println!("Loading scene...");

        let map_mesh = self.map_mesher.mesh(&self.current_map);
        map_mesh.upload(&mut self.map_mesher, state);

        self.point_light = state.insert_point_light(engine::PointLight {
            position: default(),
            intensity: 5.,
            color: Srgb::WHITE,
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
        if state.input.pressed(KeyCode::Space) {
            input_vector += Vec3A::Y;
        }

        input_vector = input_vector.normalize_or_zero() * dt * self.movement_speed;
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

        state.point_light_mut(self.point_light).expect("pl").position =
            self.camera.transform.translation.into();
    }
}

