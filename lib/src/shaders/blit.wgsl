@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // 0 -> (0,0)  1 -> (2,0)  2 -> (0,2)   → 顺时针
    let x = i32(vertex_index) & 1;
    let y = i32(vertex_index) / 2;

    let tc = vec2<f32>(
        f32(x) * 2.0, 
        f32(y) * 2.0
    );

    out.clip_position = vec4<f32>(
        tc.x * 2.0 - 1.0,
        1.0 - tc.y * 2.0,
        0.0,
        1.0
    );
    out.tex_coords = tc;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    //return vec4(1.0,0.0,0.0,1.0);
    return textureSample(t_diffuse, s_diffuse, input.tex_coords);
}