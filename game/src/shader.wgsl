[[block]] struct RenderUniforms {
    view_projection: mat4x4<f32>;
    model_matrix: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> render_uniforms: RenderUniforms;

struct VertexOutputs {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] normal: vec3<f32>;
    [[location(1)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] normal: vec3<f32>,
    [[location(2)]] uv: vec2<f32>,
) -> VertexOutputs {
    return VertexOutputs(
        render_uniforms.view_projection * render_uniforms.model_matrix * vec4<f32>(position, 1.0),
        normal,
        uv
    );
}

[[block]] struct Uniforms {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;
[[group(1), binding(1)]]
var u_diffuse_texture: texture_2d<f32>;
[[group(1), binding(2)]]
var u_diffuse_sampler: sampler;

[[stage(fragment)]]
fn fragment(vertex_outputs: VertexOutputs) -> [[location(0)]] vec4<f32> {
    return textureSample(u_diffuse_texture, u_diffuse_sampler, vec2<f32>(vertex_outputs.uv.x, 1. - vertex_outputs.uv.y));
}
