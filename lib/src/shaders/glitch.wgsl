var<uniform> power: f32;
var<uniform> rate: f32;
var<uniform> speed: f32;
var<uniform> blockCount: f32;
var<uniform> colorRate: f32;

// 随机数生成函数
fn random(seed: f32) -> f32 {
    let v = vec2<f32>(seed, seed);
    let magic = vec2<f32>(3525.46, -54.3415);
    return fract(543.2543 * sin(dot(v, magic)));
}

// 片段着色器实现
@fragment
fn fs_main(fs_input: FragmentInput) -> @location(0) vec4<f32> {
    // 计算是否启用偏移
    let enable_shift = f32(random(trunc(time.x * speed)) < rate);
    
    // 计算UV偏移
    var fixed_uv = fs_input.tex_coords;
    let y_block = trunc(fs_input.tex_coords.y * blockCount) / blockCount;
    let uv_offset = (random(y_block + time.x) - 0.5) * power * enable_shift;
    fixed_uv.x += uv_offset;
    
    // 采样主纹理
    var pixel_color = textureSample(t_diffuse, s_diffuse, fixed_uv);
    
    // 应用RGB通道分离效果
    if (enable_shift > 0.5) {
        let red_sample = textureSample(t_diffuse, s_diffuse, fixed_uv + vec2<f32>(colorRate, 0.0)).r;
        let blue_sample = textureSample(t_diffuse, s_diffuse, fixed_uv + vec2<f32>(-colorRate, 0.0)).b;
        pixel_color.r = mix(pixel_color.r, red_sample, enable_shift);
        pixel_color.b = mix(pixel_color.b, blue_sample, enable_shift);
    }
    
    return pixel_color;
}
