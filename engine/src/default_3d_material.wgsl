
struct WorldUniforms {
    view_projection: mat4x4<f32>,
}

@group(0) @binding(0)
var<uniform> world_uniforms: WorldUniforms;

struct ObjectUniforms {
    model: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> object_uniforms: ObjectUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var mvp = world_uniforms.view_projection * object_uniforms.model;

    var out: VertexOutput;
    out.clip_position = mvp * vec4<f32>(input.position, 1.);
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main() -> FragmentOutput {
    var out: FragmentOutput;
    out.color = vec4<f32>(0.3, 0.2, 0.1, 1.0);
    return out;
}
