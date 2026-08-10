import bevy_pbr::{
    render::{
        pbr_types,
        pbr_functions::alpha_discard,
        pbr_fragment::pbr_input_from_standard_material,
    },
    decal::clustered::apply_decals,
};

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::main_pass_post_lighting_processing,
};

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::resolve_vertex_output;

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

@if(FORWARD_DECAL)
import bevy_pbr::decal::forward::get_forward_decal_info;

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var layer_1: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var layer_1_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var layer_2: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var layer_2_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var layer_3: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var layer_3_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(106)
var layer_4: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(107)
var layer_4_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(108)
var alpha: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(109)
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
}

@group(constants::MATERIAL_BIND_GROUP) @binding(110)
var<uniform> animation_data: AnimationData;

@group(constants::MATERIAL_BIND_GROUP) @binding(111)
var<uniform> fix_alpha: u32;

@fragment
fn fragment(
@if(MESHLET_MESH_MATERIAL_PASS)
    @builtin(position) frag_coord: vec4<f32>,
@if(!MESHLET_MESH_MATERIAL_PASS)
    vertex_output: VertexOutput,
@if(!MESHLET_MESH_MATERIAL_PASS)
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
@if(MESHLET_MESH_MATERIAL_PASS)
    let vertex_output = resolve_vertex_output(frag_coord);
@if(MESHLET_MESH_MATERIAL_PASS)
    let is_front = true;

    var in = vertex_output;

@if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position, in.visibility_range_dither);

@if(FORWARD_DECAL)
    let forward_decal_info = get_forward_decal_info(in);
@if(FORWARD_DECAL)
    in.world_position = forward_decal_info.world_position;
@if(FORWARD_DECAL)
    in.uv = forward_decal_info.uv;

    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = calculate_color(in);
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );
    apply_decals(&pbr_input);

@if(PREPASS_PIPELINE)
    let out = deferred_output(in, pbr_input);
@if(!PREPASS_PIPELINE)
    var out: FragmentOutput;
@if(!PREPASS_PIPELINE)
    out.color = pbr_input.material.base_color;
@if(!PREPASS_PIPELINE)
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

@if(MATERIAL_OIT_ENABLED) {
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND {
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
    if alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_PREMULTIPLIED
        || alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_ADD {
        oit_draw(in.position, out.color);
        discard;
    }
}

@if(FORWARD_DECAL)
    out.color.a = min(forward_decal_info.alpha, out.color.a);

    return out;
}

fn animation_uv(
    uv: vec2<f32>,
    animation_direction: vec2<i32>,
    animation_speed: u32,
    animation_time: f32,
) -> vec2<f32> {
    let animation_speed_float = f32(animation_speed) / 7.0;
    let animation_direction_float = vec2<f32>(animation_direction);
    return ((animation_direction_float * ((animation_time * animation_speed_float) % 1.0)) + uv) % 1.0;
}

fn calculate_color(in: VertexOutput) -> vec4<f32> {
    let animation_time = 1.0;
    let uv = in.uv % 1.0;
    let alpha_uv = in.uv / 8.0;
    var alpha_values = textureSample(alpha, alpha_sampler, alpha_uv).rgb;

    if fix_alpha != 0u {
        alpha_values *= 0.7;
    }

    let layer_1_color = textureSample(
        layer_1,
        layer_1_sampler,
        animation_uv(uv, animation_data.animation_directions[0].v1, animation_data.animation_speed_1, animation_time)
    ).rgb;
    let layer_2_color = textureSample(
        layer_2,
        layer_2_sampler,
        animation_uv(uv, animation_data.animation_directions[0].v2, animation_data.animation_speed_2, animation_time)
    ).rgb;
    let layer_3_color = textureSample(
        layer_3,
        layer_3_sampler,
        animation_uv(uv, animation_data.animation_directions[1].v1, animation_data.animation_speed_3, animation_time)
    ).rgb;
    let layer_4_color = textureSample(
        layer_4,
        layer_4_sampler,
        animation_uv(uv, animation_data.animation_directions[1].v2, animation_data.animation_speed_4, animation_time)
    ).rgb;

    var rgb = layer_1_color;
    rgb = mix(rgb, layer_2_color, clamp(alpha_values.r, 0.0, 1.0));
    rgb = mix(rgb, layer_3_color, clamp(alpha_values.g, 0.0, 1.0));
    rgb = mix(rgb, layer_4_color, clamp(alpha_values.b, 0.0, 1.0));
    return vec4<f32>(rgb, 1.0);
}