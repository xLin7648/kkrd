@group(0) @binding(0)
var t_multisampled: texture_multisampled_2d<f32>;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(in_vertex_index) - 1);
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1);
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.tex_coords = vec2<f32>(x * 0.5 + 0.5, y * 0.5 + 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dims = textureDimensions(t_multisampled);
    let coords = vec2<i32>(in.tex_coords * vec2<f32>(dims));
    
    let sample_count = i32(textureNumSamples(t_multisampled));
    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    for (var i = 0; i < sample_count; i = i + 1) {
        color += textureLoad(t_multisampled, coords, i);
    }
    return color / f32(sample_count);
}
