@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@vertex
fn vs_main(
    i: VertexInput,
) -> FragmentInput {
    var o: FragmentInput;

    o.clip_position = camera.view_proj * vec4<f32>(i.position, 1.0);
    o.world_position = i.position;
    o.tex_coords = i.tex_coords;
    o.color = i.color;

    return o;
}