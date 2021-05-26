use nalgebra::{Vector3, Perspective3, Matrix4, Translation3, UnitQuaternion};

pub trait CameraMatrix: Send + Sync {
    fn get_view_matrix(&self) -> Matrix4<f32>;
    fn get_projection_matrix(&self) -> Matrix4<f32>;
    fn get_vp_matrix(&self) -> Matrix4<f32> {
        self.get_projection_matrix() * self.get_view_matrix()
    }
}
pub struct CameraComponent {
    pub matrix: Box<dyn CameraMatrix>,
    pub is_enabled: bool,
}

pub struct PerspectiveCameraMatrix {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub projection: Perspective3<f32>,
}
impl PerspectiveCameraMatrix {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0., 0., 0.),
            rotation: UnitQuaternion::default(),
            projection: Perspective3::new(1., 90., 0.01, 200.),
        }
    }
}
impl CameraMatrix for PerspectiveCameraMatrix {
    fn get_view_matrix(&self) -> Matrix4<f32> {
        (Translation3::from(self.position).to_homogeneous() * self.rotation.to_homogeneous()).try_inverse().unwrap()
    }
    fn get_projection_matrix(&self) -> Matrix4<f32> {
        self.projection.to_homogeneous()
    }
}

pub struct PerspectiveCameraSystem(pub PerspectiveCameraMatrix);
