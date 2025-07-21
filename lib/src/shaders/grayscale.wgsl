var<uniform> factor: f32;  // 灰度化系数 (0.0-1.0)

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // 采样纹理获取原始颜色
    let color = textureSample(t_diffuse, s_diffuse, input.tex_coords).rgb;
    
    // 灰度转换系数 (ITU-R BT.709)
    let lum = vec3(0.299, 0.587, 0.114);
    let gray = vec3(dot(lum, color));

    let n = (time.y + 1) / 2.0;
    
    // 根据系数混合原始色和灰度
    let result = mix(color, gray, n);
    
    return vec4(result, 1.0);
}