use nalgebra::{Perspective3, Matrix4, Translation3, UnitQuaternion};
use crate::transform::TransformComponent;

pub trait CameraMatrix: Send + Sync {
    fn get_view_matrix(&self, transform: &TransformComponent) -> Matrix4<f32> {
        transform.to_homogeneous().try_inverse().unwrap()
    }
    fn get_projection_matrix(&self) -> Matrix4<f32>;
    fn get_vp_matrix(&self, transform: &TransformComponent) -> Matrix4<f32> {
        self.get_projection_matrix() * self.get_view_matrix(transform)
    }
}
pub struct CameraComponent {
    pub matrix: Box<dyn CameraMatrix>,
    pub is_enabled: bool,
}

pub struct PerspectiveCameraMatrix(pub Perspective3<f32>);
impl PerspectiveCameraMatrix {
    pub fn new() -> Self {
        Self(Perspective3::new(1., 60.0f32.to_radians(), 0.01, 200.))
    }
}
impl CameraMatrix for PerspectiveCameraMatrix {
    fn get_projection_matrix(&self) -> Matrix4<f32> {
        self.0.to_homogeneous()
    }
}

pub struct PerspectiveCameraSystem(pub PerspectiveCameraMatrix);
