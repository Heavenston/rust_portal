// based on https://learnopengl.com/PBR/Theory
// 
// useful list of equations: https://graphicrants.blogspot.com/2013/08/specular-brdf-reference.html

// Old tests remnents
//! define ENABLE_INPUT_LINEAR_CONVERSION false
//! define ENABLE_OUTPUT_LINEAR_CONVERSION false

override PI: f32 = 3.14159265359;
// override AMBIENT_LIGHT: f32 = 0.1;
override AMBIENT_LIGHT: f32 = 1.;

fn distributionGGX(normal_direction: vec3<f32>, halfway_direction: vec3<f32>, roughness_value: f32) -> f32 {
    let alpha = roughness_value * roughness_value;
    let alpha_squared = alpha * alpha;
    let normal_dot_halfway = max(dot(normal_direction, halfway_direction), 0.0);
    let normal_dot_halfway_squared = normal_dot_halfway * normal_dot_halfway;

    let numerator = alpha_squared;
    var denominator = (normal_dot_halfway_squared * (alpha_squared - 1.0) + 1.0);
    denominator = PI * denominator * denominator;

    return numerator / (denominator + 0.0001);
}

fn geometrySchlickGGX(normal_dot_vector: f32, roughness_value: f32) -> f32 {
    let k = pow(roughness_value + 1.0, 2.0) / 8.0;
    let numerator = normal_dot_vector;
    let denominator = normal_dot_vector * (1.0 - k) + k;
    return numerator / (denominator + 0.0001);
}

fn geometrySmith(normal_direction: vec3<f32>, view_direction: vec3<f32>, light_direction: vec3<f32>, roughness_value: f32) -> f32 {
    let normal_dot_view = max(dot(normal_direction, view_direction), 0.0);
    let normal_dot_light = max(dot(normal_direction, light_direction), 0.0);
    let geometry_for_view = geometrySchlickGGX(normal_dot_view, roughness_value);
    let geometry_for_light = geometrySchlickGGX(normal_dot_light, roughness_value);
    return geometry_for_view * geometry_for_light;
}

fn fresnelSchlick(cos_theta: f32, surface_reflectivity: vec3<f32>) -> vec3<f32> {
    return surface_reflectivity + (vec3<f32>(1.0) - surface_reflectivity) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn calculatePBRDirectLighting(
    world_position: vec3<f32>,
    normal_direction: vec3<f32>,
    view_direction: vec3<f32>,
    light_direction: vec3<f32>,
    light_color: vec4<f32>,
    albedo_color: vec3<f32>,
    metallic_value: f32,
    roughness_value: f32
) -> vec3<f32> {
    // 1. Calculate per-light vectors.
    let halfway_direction = normalize(view_direction + light_direction);

    // 2. Calculate base reflectivity (F0).
    let base_reflectivity = vec3<f32>(0.04);
    let surface_reflectivity = mix(base_reflectivity, albedo_color, metallic_value);

    // 3. Get Cook-Torrance BRDF terms from helper functions.
    let distribution = distributionGGX(normal_direction, halfway_direction, roughness_value);
    let geometry = geometrySmith(normal_direction, view_direction, light_direction, roughness_value);
    let fresnel = fresnelSchlick(max(dot(halfway_direction, view_direction), 0.0), surface_reflectivity);

    // 4. Calculate the specular BRDF component.
    let normal_dot_view = max(dot(normal_direction, view_direction), 0.0);
    let normal_dot_light = max(dot(normal_direction, light_direction), 0.0);
    let specular_numerator = distribution * geometry * fresnel;
    let specular_denominator = 4.0 * normal_dot_view * normal_dot_light + 0.001;
    let specular_contribution = specular_numerator / specular_denominator;

    // 5. Determine diffuse and specular ratios.
    let specular_ratio = fresnel;
    var diffuse_ratio = vec3<f32>(1.0) - specular_ratio;
    diffuse_ratio *= (1.0 - metallic_value); // Metals have no diffuse reflection.

    // 6. Calculate the final outgoing radiance for this light.
    let incoming_light_radiance = light_color.rgb * light_color.a;
    let lambertian_diffuse = albedo_color / PI;
    
    // Combine diffuse and specular contributions, scaled by light and surface angle.
    let combined_brdf = (diffuse_ratio * lambertian_diffuse) + specular_contribution;
    let outgoing_radiance = combined_brdf * incoming_light_radiance * normal_dot_light;

    return outgoing_radiance;
}

override LIGHT_KIND_DIRECTIONAL: u32 = 0;
override LIGHT_KIND_SPOT: u32 = 1;

struct Light {
    kind: u32,
    position: vec3<f32>,
    direction: vec3<f32>,
    color: vec3<f32>,
    intensity: f32,
    inner_cone_angle: f32,
    outer_cone_angle: f32,
}

struct WorldUniforms {
    view_projection: mat4x4<f32>,
    camera_world_pos: vec3<f32>,
    light_count: u32,
}

@group(0) @binding(0)
var<uniform> world_uniforms: WorldUniforms;
@group(0) @binding(1)
var<uniform> lights: array<Light, #MAX_LIGHTS>;

struct ObjectUniforms {
    model: mat4x4<f32>,
}

@group(1) @binding(0)
var<uniform> object_uniforms: ObjectUniforms;

struct MaterialUniforms {
    base_color: vec4<f32>,
    metallic: f32,
    roughness: f32,
}

@group(2) @binding(0)
var<uniform> material_uniforms: MaterialUniforms;
//! if ENABLE_BASE_COLOR_TEXTURE
    @group(2) @binding(1)
    var t_diffuse: texture_2d<f32>;
    @group(2) @binding(2)
    var s_diffuse: sampler;
//! endif

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texcoords: vec2<f32>,
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) texcoords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_pos: vec3<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let world_pos = object_uniforms.model * vec4<f32>(input.position, 1.);
    let normal_matrix = mat3x3<f32>(
        object_uniforms.model[0].xyz,
        object_uniforms.model[1].xyz,
        object_uniforms.model[2].xyz
    );

    var out: VertexOutput;
    out.clip_position = world_uniforms.view_projection * world_pos;
    out.texcoords = input.texcoords;
    out.world_normal = normalize(normal_matrix * input.normal);
    out.world_pos = world_pos.xyz;
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
}

@fragment
fn fs_main(in: VertexOutput) -> FragmentOutput {
    let normal = normalize(in.world_normal);
    //! if ENABLE_BASE_COLOR_TEXTURE
        var albedo = textureSample(t_diffuse, s_diffuse, in.texcoords);
    //! else
        var albedo = material_uniforms.base_color;
    //! endif
    //! if ENABLE_INPUT_LINEAR_CONVERSION
    albedo = pow(albedo, vec4(2.2));
    //! endif
    let metallic = material_uniforms.metallic;
    let roughness = material_uniforms.roughness;

    let view_dir = normalize(world_uniforms.camera_world_pos.xyz - in.world_pos);

    var total_radiance = vec3<f32>(0.);
    for (var light_i: u32 = 0; light_i < world_uniforms.light_count; light_i++) {
        let light = lights[light_i];

        var light_direction: vec3<f32>;
        var light_color: vec4<f32>;
        if (light.kind == LIGHT_KIND_DIRECTIONAL) {
            light_direction = -light.direction;
            light_color = vec4<f32>(light.color, light.intensity);
        }
        if (light.kind == LIGHT_KIND_SPOT) {
            let diff = light.position - in.world_pos;
            let dist2 = dot(diff, diff);
            light_direction = normalize(diff);

            let cosInner = cos(light.inner_cone_angle);
            let cosOuter = cos(light.outer_cone_angle);
            let cosTheta = dot(light.direction, -light_direction);
            let conning = smoothstep(cosOuter, cosInner, cosTheta);
            
            let prop = (light.intensity * conning) / (0.001 + dist2);
            light_color = vec4<f32>(light.color, prop);
        }

        total_radiance += calculatePBRDirectLighting(
            in.world_pos,
            normal,
            view_dir,
            light_direction,
            light_color,
            albedo.xyz,
            metallic,
            roughness,
        );
    }

    total_radiance += albedo.xyz * AMBIENT_LIGHT;

    var color = total_radiance;
    //! if ENABLE_OUTPUT_LINEAR_CONVERSION
    color = pow(color, vec3(1.0/2.2)); 
    //! endif

    var out: FragmentOutput;
    out.color = vec4<f32>(color, 1.);
    return out;
}
