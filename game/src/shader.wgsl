[[block]] struct RenderUniforms {
    view_projection: mat4x4<f32>;
    model_matrix: mat4x4<f32>;
};
[[group(0), binding(0)]]
var<uniform> render_uniforms: RenderUniforms;

struct VertexOutputs {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vertex(
    [[location(0)]] position: vec3<f32>,
    [[location(1)]] uv: vec2<f32>
) -> VertexOutputs {
    return VertexOutputs(
        render_uniforms.view_projection * render_uniforms.model_matrix * vec4<f32>(position, 1.0),
        uv
    );
}

[[block]] struct Uniforms {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(fragment)]]
fn fragment(vertex_output: VertexOutputs) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(vertex_output.uv, 0., 1.);
}
