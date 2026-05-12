#import bevy_pbr::{
    pbr_types,
    pbr_functions::alpha_discard,
    pbr_fragment::pbr_input_from_standard_material,
    decal::clustered::apply_decals,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
}
#endif

#ifdef VISIBILITY_RANGE_DITHER
#import bevy_pbr::pbr_functions::visibility_range_dither;
#endif

#ifdef OIT_ENABLED
#import bevy_core_pipeline::oit::oit_draw
#endif // OIT_ENABLED

#ifdef FORWARD_DECAL
#import bevy_pbr::decal::forward::get_forward_decal_info
#endif




@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var layer_1: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var layer_1_sampler: sampler;


@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var layer_2: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var layer_2_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var layer_3: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(105)
var layer_3_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(106)
var layer_4: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(107)
var layer_4_sampler: sampler;

@group(#{MATERIAL_BIND_GROUP}) @binding(108)
var alpha: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(109)
var alpha_sampler: sampler;

struct IVec2Pad {
    v1: vec2<i32>,
    v2: vec2<i32>,
}

struct AnimationData {
    animation_directions: array<IVec2Pad, 2>,
    animation_speed_1: u32,
    animation_speed_2: u32,
    animation_speed_3: u32,
    animation_speed_4: u32,
};

@group(#{MATERIAL_BIND_GROUP}) @binding(110)
var<uniform> animation_data: AnimationData;

@group(#{MATERIAL_BIND_GROUP}) @binding(111)
var<uniform> fix_alpha: u32;

fn animation_uv(uv: vec2<f32>, animation_direction: vec2<i32>, animation_speed: u32, animation_time: f32) -> vec2<f32> {
    var animation_speed_float = f32(animation_speed) / 7.0;
    var animation_direction_float: vec2<f32> = vec2<f32>(animation_direction);

    return ((animation_direction_float * ((animation_time * animation_speed_float) % 1.0)) + uv) % 1.0;
}

@fragment
fn fragment(
    vertex_output: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {

    var in = vertex_output;

    // If we're in the crossfade section of a visibility range, conditionally
    // discard the fragment according to the visibility pattern.
#ifdef VISIBILITY_RANGE_DITHER
    visibility_range_dither(in.position, in.visibility_range_dither);
#endif

#ifdef FORWARD_DECAL
    let forward_decal_info = get_forward_decal_info(in);
    in.world_position = forward_decal_info.world_position;
    in.uv = forward_decal_info.uv;
#endif

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    pbr_input.material.base_color = calculate_color(in);

    // alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    // clustered decals
    apply_decals(&pbr_input);

#ifdef PREPASS_PIPELINE
    // write the gbuffer, lighting pass id, and optionally normal and motion_vector textures
    let out = deferred_output(in, pbr_input);
#else
    // in forward mode, we calculate the lit color immediately, and then apply some post-lighting effects here.
    // in deferred mode the lit color and these effects will be calculated in the deferred lighting shader
    var out: FragmentOutput;

    //if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        //out.color = apply_pbr_lighting(pbr_input);
    //} else {
        out.color = pbr_input.material.base_color;
    //}

    // apply in-shader post processing (fog, alpha-premultiply, and also tonemapping, debanding if the camera is non-hdr)
    // note this does not include fullscreen postprocessing effects like bloom.
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    #ifdef OIT_ENABLED
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        // The fragments will only be drawn during the oit resolve pass.
        oit_draw(in.position, out.color);
        discard;
    }
#endif // OIT_ENABLED

#ifdef FORWARD_DECAL
    out.color.a = min(forward_decal_info.alpha, out.color.a);
#endif

    return out;
}

fn calculate_color(in: VertexOutput) -> vec4<f32> {
    let animation_time: f32 = 1.0;
    let uv: vec2<f32> = in.uv % 1.0;

    let alpha_uv: vec2<f32> = in.uv / 8.0;

    var alpha_values: vec3<f32> = textureSample(alpha, alpha_sampler, alpha_uv).rgb;

    if fix_alpha != 0 {
        alpha_values = alpha_values * 0.7;
    }

    let a2: f32 = clamp(alpha_values.r, 0.0, 1.0);
    let a3: f32 = clamp(alpha_values.g, 0.0, 1.0);
    let a4: f32 = clamp(alpha_values.b, 0.0, 1.0);

    let layer_1_color: vec3<f32> = textureSample(layer_1, layer_1_sampler, animation_uv(uv, animation_data.animation_directions[0].v1, animation_data.animation_speed_1, animation_time)).rgb;
    let layer_2_color: vec3<f32> = textureSample(layer_2, layer_2_sampler, animation_uv(uv, animation_data.animation_directions[0].v2, animation_data.animation_speed_2, animation_time)).rgb;
    let layer_3_color: vec3<f32> = textureSample(layer_3, layer_3_sampler, animation_uv(uv, animation_data.animation_directions[1].v1, animation_data.animation_speed_3, animation_time)).rgb;
    let layer_4_color: vec3<f32> = textureSample(layer_4, layer_4_sampler, animation_uv(uv, animation_data.animation_directions[1].v2, animation_data.animation_speed_4, animation_time)).rgb;

    var rgb: vec3<f32> = layer_1_color;
    rgb = mix(rgb, layer_2_color, a2);
    rgb = mix(rgb, layer_3_color, a3);
    rgb = mix(rgb, layer_4_color, a4);

    return vec4<f32>(rgb, 1.0);
}