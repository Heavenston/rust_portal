use rapier3d::prelude::*;

use crate::application::maps::Map;
use crate::application::maps::MapModel;

pub const PLAYER_RADIUS: f32 = 0.3;
pub const PLAYER_HALF_HEIGHT: f32 = 0.5;
pub const PLAYER_EYE_HEIGHT: f32 = PLAYER_HALF_HEIGHT + PLAYER_RADIUS - 0.1;
const JUMP_VELOCITY: f32 = 3.0;
const GROUND_CHECK_DIST: f32 = 0.08;

pub struct PhysicsWorld {
    pipeline: PhysicsPipeline,
    gravity: Vector<Real>,
    integration: IntegrationParameters,
    island_manager: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,

    pending_jump: bool,
}

impl Default for PhysicsWorld {
    fn default() -> Self { Self::new() }
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: vector![0.0, -9.81, 0.0],
            integration: IntegrationParameters::default(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            pending_jump: false,
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.integration.dt = dt;
        let physics_hooks = (); // no custom hooks for now
        let event_handler = (); // no event handling

        self.pipeline.step(
            &self.gravity,
            &self.integration,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &physics_hooks,
            &event_handler,
        );

        // No query pipeline kept; queries avoided for now.
    }

    pub fn rebuild_map_collider_from_model(&mut self, model: &MapModel) {
        let mut vertices: Vec<Point<Real>> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();

        for mesh in &model.meshes {
            let base = vertices.len() as u32;
            vertices.extend(mesh.positions.iter().map(|p| point![p.x, p.y, p.z]));
            for tri in mesh.indices.chunks(3) {
                if let [a, b, c] = *tri { indices.push([base + a, base + b, base + c]); }
            }
        }

        if vertices.is_empty() || indices.is_empty() { return; }

        let rb = RigidBodyBuilder::fixed().build();
        let rb_handle = self.bodies.insert(rb);
        let co = ColliderBuilder::trimesh(vertices, indices)
            .expect("valid trimesh from mesher model")
            .friction(0.8)
            .restitution(0.0)
            .build();
        self.colliders.insert_with_parent(co, rb_handle, &mut self.bodies);
    }

    pub fn spawn_player(&mut self, start: glam::Vec3) -> RigidBodyHandle {
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![start.x, start.y, start.z])
            .lock_rotations()
            .ccd_enabled(true)
            .linear_damping(0.01)
            .build();
        let handle = self.bodies.insert(rb);

        // Capsule aligned with Y axis
        let collider = ColliderBuilder::capsule_y(PLAYER_HALF_HEIGHT, PLAYER_RADIUS)
            .friction(0.8)
            .restitution(0.0)
            .density(50.0)
            .build();
        self.colliders.insert_with_parent(collider, handle, &mut self.bodies);

        handle
    }

    pub fn spawn_cube(&mut self, center: glam::Vec3, half_extents: glam::Vec3, density: f32) -> RigidBodyHandle {
        let rb = RigidBodyBuilder::dynamic()
            .translation(vector![center.x, center.y, center.z])
            .ccd_enabled(true)
            .build();
        let handle = self.bodies.insert(rb);
        let co = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .friction(0.8)
            .restitution(0.0)
            .density(density)
            .build();
        self.colliders.insert_with_parent(co, handle, &mut self.bodies);
        handle
    }

    pub fn spawn_cube_stack(&mut self,
        base_center: glam::Vec3,
        grid_x: u32,
        grid_z: u32,
        layers: u32,
        half_extents: glam::Vec3,
        spacing: f32,
        density: f32,
    ) {
        let size = half_extents * 2.0;
        let step = glam::Vec3::new(size.x + spacing, size.y + spacing, size.z + spacing);

        // Arrange centered around base_center in XZ; Y stacks upward from base
        let total_wx = (grid_x as f32 - 1.0) * step.x;
        let total_wz = (grid_z as f32 - 1.0) * step.z;
        let origin = base_center - glam::Vec3::new(total_wx * 0.5, 0.0, total_wz * 0.5);

        for y in 0..layers {
            for ix in 0..grid_x {
                for iz in 0..grid_z {
                    let center = glam::Vec3::new(
                        origin.x + ix as f32 * step.x,
                        origin.y + half_extents.y + y as f32 * step.y,
                        origin.z + iz as f32 * step.z,
                    );
                    self.spawn_cube(center, half_extents, density);
                }
            }
        }
    }

    pub fn drive_character(&mut self, handle: RigidBodyHandle, wish_vel: glam::Vec3A, _dt: f32) {
        // Compute grounded using a downward raycast against the world
        let grounded = self.is_on_ground(handle);

        if let Some(rb) = self.bodies.get_mut(handle) {
            let mut v = *rb.linvel();
            v.x = wish_vel.x;
            v.z = wish_vel.z;

            if self.pending_jump {
                if grounded {
                    v.y = JUMP_VELOCITY;
                }
                self.pending_jump = false;
            }

            rb.set_linvel(v, true);
        }
    }

    pub fn body_position(&self, handle: RigidBodyHandle) -> Option<glam::Vec3> {
        self.bodies.get(handle).map(|rb| {
            let t = rb.translation();
            glam::Vec3::new(t.x, t.y, t.z)
        })
    }

    pub fn bodies(&self) -> &RigidBodySet { &self.bodies }

    pub fn request_jump(&mut self) { self.pending_jump = true; }

    fn is_on_ground(&self, handle: RigidBodyHandle) -> bool {
        let Some(rb) = self.bodies.get(handle) else { return false };
        let t = rb.translation();
        let feet_y = t.y - (PLAYER_HALF_HEIGHT + PLAYER_RADIUS);
        let origin = point![t.x, feet_y + 0.02, t.z];
        let ray = Ray::new(origin, vector![0.0, -1.0, 0.0]);
        let max_toi = GROUND_CHECK_DIST + 0.02;

        let filter = QueryFilter::default().exclude_rigid_body(handle);
        let queries = self.broad_phase.as_query_pipeline(
            self.narrow_phase.query_dispatcher(),
            &self.bodies,
            &self.colliders,
            filter,
        );

        if let Some((_h, hit)) = queries.cast_ray_and_get_normal(&ray, max_toi, true) {
            // Optionally check normal.y to ensure it's a floor. For now, any hit counts.
            return hit.time_of_impact <= max_toi;
        }

        false
    }
}
