#import bevy_pbr::forward_io::VertexOutput


@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var layer_1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var layer_1_sampler: sampler;


@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var layer_2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3)
var layer_2_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(4)
var layer_3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(5)
var layer_3_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(6)
var layer_4: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(7)
var layer_4_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(8)
var alpha: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(9)
var alpha_sampler: sampler;

struct IVec2Pad {
    v: vec2<i32>,
    pad: vec2<i32>
}

struct AnimationData {
    animation_directions: array<IVec2Pad, 4>,
    animation_speed_1: u32,
    animation_speed_2: u32,
    animation_speed_3: u32,
    animation_speed_4: u32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(10)
var<uniform> animation_data: AnimationData;

fn animation_uv(uv: vec2<f32>, animation_direction: vec2<i32>, animation_speed: u32, animation_time: f32) -> vec2<f32> {
    var animation_speed_float = f32(animation_speed) / 7.0;
    var animation_direction_float: vec2<f32> = vec2<f32>(animation_direction);

    return ((animation_direction_float * ((animation_time * animation_speed_float) % 1.0)) + uv) % 1.0;
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var animation_time = 1.0;
    let uv: vec2<f32> = mesh.uv % 1.0;
    
    let alpha_uv: vec2<f32> = mesh.uv / 8.0;
    let alpha_values = textureSample(alpha, alpha_sampler, alpha_uv).rgb;

    var layer_1_color: vec3<f32> = textureSample(layer_1, layer_1_sampler, animation_uv(uv, animation_data.animation_directions[0].v, animation_data.animation_speed_1, animation_time)).rgb;
    var layer_2_color: vec3<f32> = textureSample(layer_2, layer_2_sampler, animation_uv(uv, animation_data.animation_directions[1].v, animation_data.animation_speed_2, animation_time)).rgb;
    var layer_3_color: vec3<f32> = textureSample(layer_3, layer_3_sampler, animation_uv(uv, animation_data.animation_directions[2].v, animation_data.animation_speed_3, animation_time)).rgb;
    var layer_4_color: vec3<f32> = textureSample(layer_4, layer_4_sampler, animation_uv(uv, animation_data.animation_directions[3].v, animation_data.animation_speed_4, animation_time)).rgb;

    var alpha_2_value: f32 = alpha_values.r;
    var alpha_3_value: f32 = alpha_values.g;
    var alpha_4_value: f32 = alpha_values.b;
    var alpha_1_value = 1.0 - (alpha_2_value + alpha_3_value + alpha_4_value);

    var final_color1: vec3<f32> = layer_1_color * alpha_1_value;
    var final_color2: vec3<f32> = layer_2_color * alpha_2_value;
    var final_color3: vec3<f32> = layer_3_color * alpha_3_value;
    var final_color4: vec3<f32> = layer_4_color * alpha_4_value;

    // FIXME: for some reason this color blending returns incorrect colors, have yet to figure out why
    var final_color: vec4<f32> = vec4(final_color1 + final_color2 + final_color3 + final_color4, 1.0);

    return final_color;
}