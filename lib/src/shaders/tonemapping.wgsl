// 顶点着色器
@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    return vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
}

// 片段着色器
@group(0) @binding(0)
var t_color: texture_2d<f32>;
@group(0) @binding(1)
var s_color: sampler;

@fragment
fn fs_main(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_coord.xy / vec2<f32>(textureDimensions(t_color));
    let color = textureSample(t_color, s_color, uv);
   
    // 简单的 Reinhard 色调映射
    let mapped = color.rgb / (color.rgb + vec3<f32>(1.0));
   
    // Gamma 校正
    let gamma = 2.2;
    let corrected = pow(mapped, vec3<f32>(1.0 / gamma));
   
    return vec4<f32>(corrected, color.a);
}