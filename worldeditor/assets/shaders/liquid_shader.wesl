import bevy_pbr::{
    render::{
        pbr_types,
        pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing},
        pbr_fragment::pbr_input_from_standard_material,
        forward_io::{VertexOutput, FragmentOutput},
    },
    decal::clustered::apply_decals,
};
import bevy_pbr::render::mesh_view_bindings::globals;
import bevy_pbr::render::pbr_types::STANDARD_MATERIAL_FLAGS_UNLIT_BIT;

@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;

@if(MESHLET_MESH_MATERIAL_PASS)
import bevy_pbr::meshlet::visibility_buffer_resolve::resolve_vertex_output;

@if(MATERIAL_OIT_ENABLED)
import bevy_core_pipeline::oit::draw::oit_draw;

@if(FORWARD_DECAL)
import bevy_pbr::decal::forward::get_forward_decal_info;

@group(constants::MATERIAL_BIND_GROUP) @binding(100)
var liquid_frames: texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101)
var liquid_sampler: sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(102)
var<uniform> frame_count: u32;
@group(constants::MATERIAL_BIND_GROUP) @binding(103)
var<uniform> add_base_color: u32;
@group(constants::MATERIAL_BIND_GROUP) @binding(104)
var<uniform> uv_scroll: vec2<f32>;

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
    let animation_time = globals.time * (1000.0 / 60.0);
    let frame = i32(u32(floor(animation_time)) % max(frame_count, 1u));
    let animated_uv = in.uv + uv_scroll * animation_time * (1.0 / 48.0);
    let liquid_color = textureSample(liquid_frames, liquid_sampler, animated_uv, frame);
    if add_base_color != 0u {
        pbr_input.material.base_color = vec4(
            clamp(pbr_input.material.base_color.rgb + liquid_color.rgb, vec3(0.0), vec3(1.0)),
            pbr_input.material.base_color.a,
        );
    } else {
        pbr_input.material.base_color *= liquid_color;
    }
    pbr_input.material.base_color = alpha_discard(
        pbr_input.material.flags,
        pbr_input.material.alpha_cutoff,
        pbr_input.material.base_color
    );
    apply_decals(&pbr_input);

    var out: FragmentOutput;
    if (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_UNLIT_BIT) == 0u {
        out.color = apply_pbr_lighting(pbr_input);
    } else {
        out.color = pbr_input.material.base_color;
    }
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);

@if(MATERIAL_OIT_ENABLED) {
    let alpha_mode = pbr_input.material.flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if (alpha_mode == pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND) {
        oit_draw(in.position, vec4(out.color.rgb * out.color.a, out.color.a));
        discard;
    }
}

@if(FORWARD_DECAL)
    out.color.a = min(forward_decal_info.alpha, out.color.a);

    return out;
}