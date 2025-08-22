//! Based on https://sotrh.github.io/learn-wgpu/intermediate/tutorial11-normals/#the-tangent-and-the-bitangent

use glam::{ Vec2, Vec3 };

#[derive(Debug, Clone, Copy)]
pub struct ComputeTangentOutput {
    pub tangent: Vec3,
    pub bitangent: Vec3,
}

pub fn compute_tangent([pos0, pos1, pos2]: [Vec3; 3], [uv0, uv1, uv2]: [Vec2; 3]) -> ComputeTangentOutput {
    // Calculate the edges of the triangle
    let delta_pos1 = pos1 - pos0;
    let delta_pos2 = pos2 - pos0;

    // This will give us a direction to calculate the
    // tangent and bitangent
    let delta_uv1 = uv1 - uv0;
    let delta_uv2 = uv2 - uv0;

    // Solving the following system of equations will
    // give us the tangent and bitangent.
    //     delta_pos1 = delta_uv1.x * T + delta_u.y * B
    //     delta_pos2 = delta_uv2.x * T + delta_uv2.y * B
    // Luckily, the place I found this equation provided
    // the solution!
    let r = 1.0 / (delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x);
    let tangent = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
    // We flip the bitangent to enable right-handed normal
    // maps with wgpu texture coordinate system
    let bitangent = (delta_pos2 * delta_uv1.x - delta_pos1 * delta_uv2.x) * -r;

    ComputeTangentOutput {
        tangent, bitangent,
    }
}

#[derive(Debug, Clone)]
pub struct ComputeAllTangentOutput {
    pub tangents: Vec<Vec3>,
    pub bitangents: Vec<Vec3>,
}

pub fn compute_all_tangents(positions: &[Vec3], uvs: &[Vec2], indices: &[u32]) -> ComputeAllTangentOutput {
    let mut triangles_included = vec![0u32; positions.len()];
    let mut tangents = vec![Vec3::ZERO; positions.len()];
    let mut bitangents = vec![Vec3::ZERO; positions.len()];

    for is in indices.iter().map(|&i| i as usize).array_chunks::<3>() {
        let pos = is.map(|i| positions[i]);
        let uv = is.map(|i| uvs[i]);
        let ComputeTangentOutput {
            tangent, bitangent,
        } = compute_tangent(pos, uv);
        for i in is {
            triangles_included[i] += 1;
            tangents[i] += tangent;
            bitangents[i] += bitangent;
        }
    }

    for i in 0..positions.len() {
        if triangles_included[i] == 0 { continue }
        tangents[i] /= triangles_included[i] as f32;
        bitangents[i] /= triangles_included[i] as f32;
    }

    ComputeAllTangentOutput {
        tangents,
        bitangents,
    }
}
