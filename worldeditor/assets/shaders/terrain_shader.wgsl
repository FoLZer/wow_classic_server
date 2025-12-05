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

fn saturation(color: vec4<f32>, adjustment: f32) -> vec4<f32>
{
    // Algorithm from Chapter 16 of OpenGL Shading Language
    let W: vec4<f32> = vec4(0.2125, 0.7154, 0.0721, 1.0);
    let intensity: vec4<f32> = vec4(dot(color, W));
    return mix(intensity, color, adjustment);
}

@fragment
fn fragment(
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    var uv = mesh.uv;

    let alpha_values = textureSample(alpha, alpha_sampler, uv);

    var layer_1_color: vec4<f32> = textureSample(layer_1, layer_1_sampler, uv);

    var layer_2_color: vec4<f32> = textureSample(layer_2, layer_2_sampler, uv);
    var alpha_2_value: f32 = alpha_values.r;

    var layer_3_color: vec4<f32> = textureSample(layer_3, layer_3_sampler, uv);
    var alpha_3_value: f32 = alpha_values.g;

    var layer_4_color: vec4<f32> = textureSample(layer_4, layer_4_sampler, uv);
    var alpha_4_value: f32 = alpha_values.b;

    // finalColor = tex0 * (1.0 - (alpha1 + alpha2 + alpha3)) + tex1 * alpha1 + tex2 * alpha2 + tex3 * alpha3
    //var final_color: vec4<f32> = layer_1_color * (1.0 - (alpha_2_value + alpha_3_value + alpha_4_value)) + (layer_2_color * alpha_2_value) + (layer_3_color * alpha_3_value) + (layer_4_color * alpha_4_value);

    //var final_color: vec4<f32> = layer_1_color + layer_2_color * alpha_2_value;
    var final_color1: vec4<f32> = layer_1_color;
    var final_color2: vec4<f32> = layer_2_color;
    var final_color2a: vec4<f32> = layer_2_color * alpha_2_value;
    var final_color3: vec4<f32> = layer_3_color;
    var final_color3a: vec4<f32> = layer_3_color * alpha_3_value;
    var final_color4: vec4<f32> = layer_4_color;
    var final_color4a: vec4<f32> = layer_4_color * alpha_4_value;

    //return vec4((alpha_2_value + alpha_3_value + alpha_4_value), 0, 0, 1.0);

    var layer_1_alpha = 1.0 - (alpha_2_value + alpha_3_value + alpha_4_value);
    var final_color1a: vec4<f32> = layer_1_color * layer_1_alpha;
    //return final_color2a + final_color3a + final_color4a;
    var final_color: vec4<f32> = final_color1a + final_color2a + final_color3a + final_color4a;

    // return layer_1_color * (1.0 - (alpha_2_value + alpha_3_value + alpha_4_value)) + (layer_2_color * alpha_2_value);

    return final_color;
}