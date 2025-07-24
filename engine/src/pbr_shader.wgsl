override ENABLE_DIFFUSE_TEXTURE: bool = false;

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

struct MaterialUniforms {
    base_color: vec4<f32>,
}

@group(2) @binding(0)
var<uniform> material_uniforms: MaterialUniforms;
@group(2) @binding(1)
var<uniform> t_diffuse: texture_2d<f32>;
@group(2) @binding(2)
var<uniform> s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texcoords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoords: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var mvp = world_uniforms.view_projection * object_uniforms.model;

    var out: VertexOutput;
    out.clip_position = mvp * vec4<f32>(input.position, 1.);
    out.texcoords = input.texcoords;
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    if ENABLE_DIFFUSE_TEXTURE {
        out.color = textureSample(t_diffuse, s_diffuse, in.texcoords);
    }
    else {
        out.color = material_uniforms.base_color;
    }
    return out;
}
