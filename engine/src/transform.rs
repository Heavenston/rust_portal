use std::f32;

use approx::*;
use nalgebra::{base::dimension::Dim, Matrix4, Translation3, UnitQuaternion, Vector3, Vector4};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TransformComponent {
    pub scale: Vector3<f32>,
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
}
impl TransformComponent {
    pub fn from_homogeneous(matrix: &Matrix4<f32>) -> Self {
        Self {
            scale: Vector3::from_iterator(matrix.column_iter().map(|a| a.xyz().norm())),
            position: matrix.column(3).xyz(),
            rotation: UnitQuaternion::from_matrix(&matrix.resize_generic(
                Dim::from_usize(3),
                Dim::from_usize(3),
                1.,
            )),
        }
    }

    pub fn to_homogeneous(&self) -> Matrix4<f32> {
        Translation3::from(self.position).to_homogeneous()
            * self.rotation.to_homogeneous()
            * Matrix4::from_diagonal(&Vector4::new(self.scale.x, self.scale.y, self.scale.z, 1.))
    }
}
impl Default for TransformComponent {
    fn default() -> Self {
        Self {
            scale: Vector3::new(1., 1., 1.),
            position: Vector3::zeros(),
            rotation: UnitQuaternion::identity(),
        }
    }
}
impl AbsDiffEq for TransformComponent {
    type Epsilon = f32;

    fn default_epsilon() -> f32 { <f32 as AbsDiffEq>::default_epsilon() }

    fn abs_diff_eq(&self, other: &Self, epsilon: f32) -> bool {
        other.scale.abs_diff_eq(&self.scale, epsilon)
            && other.position.abs_diff_eq(&self.position, epsilon)
            && other.rotation.abs_diff_eq(&self.rotation, epsilon)
    }
}
impl RelativeEq for TransformComponent {
    fn default_max_relative() -> f32 { <f32 as RelativeEq>::default_max_relative() }

    fn relative_eq(&self, other: &Self, epsilon: f32, max_relative: f32) -> bool {
        other.scale.relative_eq(&self.scale, epsilon, max_relative)
            && other
                .position
                .relative_eq(&self.position, epsilon, max_relative)
            && other
                .rotation
                .relative_eq(&self.rotation, epsilon, max_relative)
    }
}

#[cfg(test)]
mod tests {
    use nalgebra::{UnitQuaternion, Vector3};

    use super::*;

    #[test]
    fn matrix_transform_conversion() {
        let transform = TransformComponent {
            scale: Vector3::new(1., 5., 7.),
            position: Vector3::new(8842., -4531., 8.),
            rotation: UnitQuaternion::from_euler_angles(1., f32::consts::PI, f32::consts::PI / 2.),
        };
        let matrix = transform.to_homogeneous();
        let other_transform = TransformComponent::from_homogeneous(&matrix);

        assert_relative_eq!(transform, other_transform);
    }
}
