[[stage(vertex)]]
fn vertex([[location(0)]] position: vec2<f32>) -> [[builtin(position)]] vec4<f32> {
    return vec4<f32>(position.xy, 0.0, 1.0);
}

[[block]] struct Uniforms {
    color: vec4<f32>;
};
[[group(1), binding(0)]]
var<uniform> uniforms: Uniforms;

[[stage(fragment)]]
fn fragment() -> [[location(0)]] vec4<f32> {
    return uniforms.color;
}
