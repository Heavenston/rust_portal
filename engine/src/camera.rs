use nalgebra::{Vector3, Perspective3, Matrix4, Translation3, UnitQuaternion};

pub trait Camera {
    fn get_view_matrix(&self) -> Matrix4<f32>;
    fn get_projection_matrix(&self) -> Matrix4<f32>;
    fn get_vp_matrix(&self) -> Matrix4<f32> {
        self.get_view_matrix() * self.get_projection_matrix()
    }
}
impl<T: Camera> Camera for &T {
    fn get_view_matrix(&self) -> Matrix4<f32> { (*self).get_view_matrix() }
    fn get_projection_matrix(&self) -> Matrix4<f32> { (*self).get_projection_matrix() }
}

pub struct PerspectiveCamera {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub projection: Perspective3<f32>,
}
impl PerspectiveCamera {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0., 0., 0.),
            rotation: UnitQuaternion::default(),
            projection: Perspective3::new(1., 90., 0.01, 200.)
        }
    }
}
impl Camera for PerspectiveCamera {
    fn get_view_matrix(&self) -> Matrix4<f32> {
        (Translation3::from(self.position).to_homogeneous() * self.rotation.to_homogeneous()).try_inverse().unwrap()
    }
    fn get_projection_matrix(&self) -> Matrix4<f32> {
        self.projection.to_homogeneous()
    }
}