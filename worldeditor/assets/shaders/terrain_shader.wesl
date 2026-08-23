import bevy_pbr::{
    render::{
        pbr_types,
        pbr_functions::alpha_discard,
        pbr_fragment::pbr_input_from_standard_material,
    },
    decal::clustered::apply_decals,
};
@if(PREPASS_PIPELINE)
import bevy_render::globals::Globals;
@if(!PREPASS_PIPELINE)
import bevy_pbr::render::mesh_view_bindings::globals;

@if(PREPASS_PIPELINE)
import bevy_pbr::{
    prepass::io::{VertexOutput, FragmentOutput},
    deferred::functions::deferred_output,
};
@else
import bevy_pbr::render::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT,
};

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::resolve_vertex_output;

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

@if(FORWARD_DECAL)
import bevy_pbr::decal::forward::get_forward_decal_info;

@if(PREPASS_PIPELINE)
@group(0) @binding(1)
var<uniform> globals: Globals;

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var terrain_textures: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var terrain_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var alpha_map: texture_2d<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var alpha_sampler: sampler;

@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var layer_map: texture_2d<u32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(105)
var animation_map: texture_2d<u32>;

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
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
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
    packed_animation: u32,
    animation_time: f32,
) -> vec2<f32> {
    let animation_speed = packed_animation >> 4u;
    let animation_direction = vec2<i32>(
        i32((packed_animation >> 2u) & 3u) - 1,
        i32(packed_animation & 3u) - 1,
    );
    let animation_speed_float = f32(animation_speed) / 11.2;
    let animation_direction_float = vec2<f32>(animation_direction);
    return animation_direction_float * animation_time * animation_speed_float + uv;
}

fn calculate_color(in: VertexOutput) -> vec4<f32> {
    let animation_time = globals.time;
    let chunk_coord = vec2<i32>(clamp(floor(in.uv / 8.0), vec2<f32>(0.0), vec2<f32>(15.0)));
    let chunk_uv = in.uv - vec2<f32>(chunk_coord) * 8.0;
    let uv = chunk_uv;
    let alpha_half_texel = 0.5 / 16.0;
    let chunk_alpha_uv = clamp(
        chunk_uv / 8.0,
        vec2<f32>(alpha_half_texel),
        vec2<f32>(1.0 - alpha_half_texel),
    );
    let alpha_uv = (vec2<f32>(chunk_coord) + chunk_alpha_uv) / 16.0;
    let alpha_sample = textureSample(alpha_map, alpha_sampler, alpha_uv);
    var alpha_values = alpha_sample.rgb;
    if alpha_sample.a > 0.5 {
        alpha_values *= 0.7;
    }

    let texture_layers = textureLoad(layer_map, chunk_coord, 0);
    let animations = textureLoad(animation_map, chunk_coord, 0);
    let layer_1_color = textureSample(
        terrain_textures,
        terrain_sampler,
        animation_uv(uv, animations.x, animation_time),
        i32(texture_layers.x),
    ).rgb;
    let layer_2_color = textureSample(
        terrain_textures,
        terrain_sampler,
        animation_uv(uv, animations.y, animation_time),
        i32(texture_layers.y),
    ).rgb;
    let layer_3_color = textureSample(
        terrain_textures,
        terrain_sampler,
        animation_uv(uv, animations.z, animation_time),
        i32(texture_layers.z),
    ).rgb;
    let layer_4_color = textureSample(
        terrain_textures,
        terrain_sampler,
        animation_uv(uv, animations.w, animation_time),
        i32(texture_layers.w),
    ).rgb;

    var rgb = layer_1_color;
    rgb = mix(rgb, layer_2_color, clamp(alpha_values.r, 0.0, 1.0));
    rgb = mix(rgb, layer_3_color, clamp(alpha_values.g, 0.0, 1.0));
    rgb = mix(rgb, layer_4_color, clamp(alpha_values.b, 0.0, 1.0));
    return vec4<f32>(rgb, 1.0);
}