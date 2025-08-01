use crate::*;

const Z_DIV: f32 = 1000.0;

pub fn draw_circle(center: Vec2, r: f32, color: Color, z_index: i32) {
    draw_poly_z(center, 40, r, 0.0, color, z_index, BlendMode::Alpha);
}

pub fn draw_poly_z(
    position: Vec2,
    sides: u8,
    radius: f32,
    rotation: f32,
    color: Color,
    z_index: i32,
    blend_mode: BlendMode,
) {
    draw_poly2_z(
        position,
        sides,
        Vec2::splat(radius),
        rotation,
        color,
        z_index,
        blend_mode,
    );
}

pub fn draw_poly2_z(
    position: Vec2,
    sides: u8,
    radius: Vec2,
    rotation: f32,
    color: Color,
    z_index: i32,
    blend_mode: BlendMode,
) {
    let (x, y) = (position.x, position.y);
    let z = z_index as f32 / Z_DIV;

    let mut vertices = Vec::<SpriteVertex>::with_capacity(sides as usize + 2);
    let mut indices = Vec::<u32>::with_capacity(sides as usize * 3);

    let rot = rotation.to_radians();
    vertices.push(SpriteVertex::new(vec3(x, y, z), vec2(0.0, 0.0), color));

    for i in 0..sides + 1 {
        let rx = (i as f32 / sides as f32 * std::f32::consts::PI * 2. + rot).cos();
        let ry = (i as f32 / sides as f32 * std::f32::consts::PI * 2. + rot).sin();

        let vertex = SpriteVertex::new(
            vec3(x + radius.x * rx, y + radius.y * ry, z),
            vec2(rx, ry),
            color,
        );

        vertices.push(vertex);

        if i != sides {
            indices.extend_from_slice(&[0, i as u32 + 1, i as u32 + 2]);
        }
    }

    draw_mesh_ex(
        Mesh {
            origin: position.extend(z_index as f32),
            vertices: vertices.into(),
            indices: indices.into(),
            z_index,
            ..Default::default()
        },
        blend_mode,
    );
}

pub fn draw_mesh(mesh: Mesh) {
    draw_mesh_ex(mesh, BlendMode::default());
}

pub fn draw_mesh_ex(mesh: Mesh, blend_mode: BlendMode) {
    queue_mesh_draw(mesh, blend_mode);
}

pub fn draw_line(p1: Vec2, p2: Vec2, thickness: f32, color: Color, z_index: i32) {
    draw_line_tex(p1, p2, thickness, z_index, color, None);
}

pub fn draw_line_tex(
    p1: Vec2,
    p2: Vec2,
    thickness: f32,
    z_index: i32,
    color: Color,
    texture: Option<TextureHandle>,
) {
    let (x1, y1) = (p1.x, p1.y);
    let (x2, y2) = (p2.x, p2.y);

    let dx = x2 - x1;
    let dy = y2 - y1;

    // https://stackoverflow.com/questions/1243614/how-do-i-calculate-the-normal-vector-of-a-line-segment

    let nx = -dy;
    let ny = dx;

    let tlen = (nx * nx + ny * ny).sqrt() / (thickness * 0.5);
    if tlen < std::f32::EPSILON {
        return;
    }
    let tx = nx / tlen;
    let ty = ny / tlen;

    // 0 0      1 0
    //
    // 0 1      1 1

    let z = z_index as f32 / Z_DIV;

    let vertices = [
        SpriteVertex::new(vec3(x1 + tx, y1 + ty, z), vec2(0.0, 0.0), color),
        SpriteVertex::new(vec3(x1 - tx, y1 - ty, z), vec2(1.0, 0.0), color),
        SpriteVertex::new(vec3(x2 + tx, y2 + ty, z), vec2(0.0, 1.0), color),
        SpriteVertex::new(vec3(x2 - tx, y2 - ty, z), vec2(1.0, 1.0), color),
    ];

    // let vertices = vec![
    //     SpriteVertex::new(vec2(x1 + tx, y1 + ty), vec2(0.0, 0.0), color),
    //     SpriteVertex::new(vec2(x1 - tx, y1 - ty), vec2(1.0, 0.0), color),
    //     SpriteVertex::new(vec2(x2 + tx, y2 + ty), vec2(1.0, 1.0), color),
    //     SpriteVertex::new(vec2(x2 - tx, y2 - ty), vec2(0.0, 1.0), color),
    // ];

    let indices = [0, 1, 2, 2, 1, 3];

    draw_mesh(Mesh {
        origin: vec3((x1 + x2) / 2.0, (y1 + y2) / 2.0, z_index as f32),
        vertices: SmallVec::from_slice(&vertices),
        indices: indices.into(),
        z_index,
        texture,
        y_sort_offset: 0.0,
        ..Default::default()
    })
}

pub fn draw_quad(raw_draw_params: RawDrawParams) {
    draw_sprite_ex(
        texture_id("1px"),
        DrawTextureParams {
            raw_draw_params,
            ..Default::default()
        },
    );
}

pub fn draw_sprite_ex(texture: TextureHandle, params: DrawTextureParams) {
    let mut params = params.clone();
    if params.raw_draw_params.dest_size.is_none() {
        params.raw_draw_params.dest_size = Some(match texture {
            TextureHandle::Path(_) | TextureHandle::Raw(_) => match Assets::image_size(texture) {
                ImageSizeResult::Loaded(size) => size,
                ImageSizeResult::LoadingInProgress => {
                    return;
                }
                ImageSizeResult::ImageNotFound => {
                    error!("NO SIZE FOR TEXTURE {:?}", texture);
                    UVec2::ONE
                }
            },
            TextureHandle::RenderTarget(render_target_id) => {
                let rts = get_global_render_targets().read();

                if let Some(rt) = rts.get(&render_target_id) {
                    rt.read().size
                } else {
                    return;
                }
            }
        })
    }

    let is_rt = match texture {
        TextureHandle::RenderTarget(_) => true,
        _ => false,
    };

    let vertices = rotated_rectangle(params.scroll_offset, &params.raw_draw_params, is_rt);

    const QUAD_INDICES_U32: &[u32] = &[0, 1, 2, 0, 2, 3];

    let mesh = Mesh {
        origin: params.raw_draw_params.position,
        vertices: SmallVec::from_slice(&vertices),
        indices: QUAD_INDICES_U32.into(),
        z_index: params.raw_draw_params.z_index,
        texture: Some(texture),
        y_sort_offset: params.y_sort_offset,
    };

    draw_mesh_ex(mesh, params.raw_draw_params.blend_mode);
}

#[derive(Clone, Debug)]
pub struct RawDrawParams {
    pub position: Vec3,
    pub rotation: Rotation,
    pub scale: Vec2,
    pub dest_size: Option<UVec2>,
    pub z_index: i32,

    pub pivot: Option<Vec2>,
    pub color: Color,
    pub flip_x: bool,
    pub flip_y: bool,
    pub blend_mode: BlendMode,
}

impl Default for RawDrawParams {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            rotation: Rotation::Zero,
            scale: Vec2::ONE,
            dest_size: None,
            z_index: 0,
            pivot: None,
            color: WHITE,
            flip_x: false,
            flip_y: false,
            blend_mode: BlendMode::None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DrawTextureParams {
    pub scroll_offset: Vec2,
    pub y_sort_offset: f32,
    pub raw_draw_params: RawDrawParams,
}

impl Default for DrawTextureParams {
    fn default() -> DrawTextureParams {
        DrawTextureParams {
            scroll_offset: Vec2::ZERO,

            y_sort_offset: 0.0,
            raw_draw_params: RawDrawParams::default(),
        }
    }
}

impl RawDrawParams {
    pub fn blend(blend_mode: BlendMode) -> RawDrawParams {
        RawDrawParams {
            blend_mode,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug)]
pub enum Rotation {
    X(f32),
    Y(f32),
    Z(f32),
    Euler(f32, f32, f32),
    Quaternion(f32, f32, f32, f32),
    Zero,
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Zero
    }
}

pub fn rotated_rectangle(
    scroll_offset: Vec2,
    params: &RawDrawParams,
    is_rt: bool,
) -> [SpriteVertex; 4] {
    // 处理目标尺寸和翻转
    let (scale_w, scale_h) = {
        let scale = params.scale;
        let wh = params.dest_size.unwrap_or(UVec2::ONE);
        (wh.x as f32 * scale.x, wh.y as f32 * scale.y)
    };

    // 计算 pivot 偏移（Unity 风格，0-1 范围）
    let pivot_offset = match params.pivot {
        Some(p) => vec3(p.x * scale_w, p.y * scale_h, 0.0),
        None => vec3(scale_w / 2.0, scale_h / 2.0, 0.0),
    };

    // 获取旋转角度（支持XYZ三轴）
    let mut rotation_angles = match params.rotation {
        Rotation::Zero => Vec3::ZERO,
        Rotation::X(angle) => vec3(angle, 0.0, 0.0),
        Rotation::Y(angle) => vec3(0.0, angle, 0.0),
        Rotation::Z(angle) => vec3(0.0, 0.0, angle),
        Rotation::Euler(x, y, z) => vec3(x, y, z),
        Rotation::Quaternion(x, y, z, w) => quat(x, y, z, w).to_euler(EulerRot::XYZ).into(),
    };

    rotation_angles.x = rotation_angles.x.to_radians();
    rotation_angles.y = rotation_angles.y.to_radians();
    rotation_angles.z = rotation_angles.z.to_radians();

    // 创建3x3旋转矩阵（左手坐标系，ZXY旋转顺序）
    let rotation_matrix = {
        let (sx, cx) = rotation_angles.x.sin_cos();
        let (sy, cy) = rotation_angles.y.sin_cos();
        let (sz, cz) = rotation_angles.z.sin_cos();

        // 绕X轴旋转矩阵（pitch）
        let rot_x = Mat3::from_cols(
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, cx, sx),
            Vec3::new(0.0, -sx, cx),
        );

        // 绕Y轴旋转矩阵（yaw）
        let rot_y = Mat3::from_cols(
            Vec3::new(cy, 0.0, -sy),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(sy, 0.0, cy),
        );

        // 绕Z轴旋转矩阵（roll）
        let rot_z = Mat3::from_cols(
            Vec3::new(cz, sz, 0.0),
            Vec3::new(-sz, cz, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );

        // 组合旋转：先Z，再X，最后Y (ZXY顺序)
        rot_y * rot_x * rot_z
    };

    // 定义基础顶点（3D空间）
    let base_vertices = [
        vec3(0.0, 0.0, 0.0),         // 左上
        vec3(0.0, scale_h, 0.0),     // 左下
        vec3(scale_w, scale_h, 0.0), // 右下
        vec3(scale_w, 0.0, 0.0),     // 右上
    ];

    // 应用旋转和平移
    let world_vertices: [Vec3; 4] = base_vertices.map(|v| {
        // 转换为 pivot 相对坐标
        let pivot_relative = v - pivot_offset;
        // 应用3D旋转
        let rotated = rotation_matrix * pivot_relative;
        // 转换回世界坐标（包含Z轴）
        rotated + params.position
    });

    let tex_coords: [Vec2; 4] = if is_rt {
        // RT 默认 Y 向上，所以要翻转 UV
        [
            scroll_offset + tex_coord_flip(vec2(0.0, 1.0), params), // 左下 -> 左上
            scroll_offset + tex_coord_flip(vec2(0.0, 0.0), params), // 左上 -> 左下
            scroll_offset + tex_coord_flip(vec2(1.0, 0.0), params), // 右上 -> 右下
            scroll_offset + tex_coord_flip(vec2(1.0, 1.0), params), // 右下 -> 右上
        ]
    } else {
        [
            scroll_offset + tex_coord_flip(vec2(0.0, 0.0), params), // 左上 -> 左下
            scroll_offset + tex_coord_flip(vec2(0.0, 1.0), params), // 左下 -> 右下
            scroll_offset + tex_coord_flip(vec2(1.0, 1.0), params), // 右下 -> 右上
            scroll_offset + tex_coord_flip(vec2(1.0, 0.0), params), // 右上 -> 左上
        ]
    };

    // 创建最终顶点
    [
        SpriteVertex::new(world_vertices[0], tex_coords[0], params.color),
        SpriteVertex::new(world_vertices[1], tex_coords[1], params.color),
        SpriteVertex::new(world_vertices[2], tex_coords[2], params.color),
        SpriteVertex::new(world_vertices[3], tex_coords[3], params.color),
    ]
}

pub fn tex_coord_flip(mut xy: Vec2, params: &RawDrawParams) -> Vec2 {
    fn flip(v: f32) -> f32 {
        if v == 0.0 { 1.0 } else { 0.0 }
    }

    if params.flip_x {
        xy.x = flip(xy.x);
    }

    if params.flip_y {
        xy.y = flip(xy.y);
    }

    xy
}
