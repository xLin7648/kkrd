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
    })
}

pub fn draw_rect_rot(x: f32, y: f32, w: f32, h: f32, rotation: Rotation, pivot: Vec2, color: Color, z_index: i32) {
    draw_quad(
        vec2(x, y),
        vec2(w, h),
        rotation,
        pivot,
        color,
        z_index,
        texture_id("1px"),
        Vec2::ZERO,
    );
}

pub fn draw_quad(
    position: Vec2,
    size: Vec2,
    rotation: Rotation,
    pivot: Vec2,
    color: Color,
    z_index: i32,
    texture: TextureHandle,
    scroll_offset: Vec2,
) {
    draw_sprite_ex(
        texture,
        position,
        color,
        z_index,
        DrawTextureParams {
            dest_size: Some(size),
            scroll_offset,
            rotation,
            pivot: Some(pivot),
            ..Default::default()
        },
    );
}

pub fn draw_sprite_ex(
    texture: TextureHandle,
    position: Vec2,
    tint: Color,
    z_index: i32,
    params: DrawTextureParams,
) {
    let raw = RawDrawParams {
        dest_size: params.dest_size.map(|s| s),
        source_rect: params.source_rect,
        rotation: params.rotation,
        flip_x: params.flip_x,
        flip_y: params.flip_y,
        pivot: params.pivot,
    };

    // if !CAMERA_BOUNDS
    //     .load()
    //     .contains_rect(position, raw.dest_size.unwrap_or(Vec2::ONE))
    // {
    //     return;
    // }

    let size = match Assets::image_size(texture) {
        ImageSizeResult::Loaded(size) => size,
        ImageSizeResult::LoadingInProgress => {
            return;
        }
        ImageSizeResult::ImageNotFound => {
            error!("NO SIZE FOR TEXTURE {:?}", texture);
            UVec2::ONE
        }
    };

    let vertices = rotated_rectangle(
        position.extend(z_index as f32 / Z_DIV),
        raw,
        size.x as f32,
        size.y as f32,
        tint,
        params.scroll_offset,
    );

    const QUAD_INDICES_U32: &[u32] = &[0, 1, 2, 0, 2, 3];

    let mesh = Mesh {
        origin: position.extend(z_index as f32),
        vertices: SmallVec::from_slice(&vertices),
        indices: QUAD_INDICES_U32.into(),
        z_index,
        texture: Some(texture),
        y_sort_offset: params.y_sort_offset,
    };

    draw_mesh_ex(mesh, params.blend_mode);
}

#[derive(Clone, Debug, Default)]
pub struct RawDrawParams {
    pub dest_size: Option<Vec2>,
    pub source_rect: Option<IRect>,
    pub rotation: Rotation,
    pub flip_x: bool,
    pub flip_y: bool,
    pub pivot: Option<Vec2>,
}

#[derive(Clone, Debug)]
pub struct DrawTextureParams {
    pub dest_size: Option<Vec2>,
    pub source_rect: Option<IRect>,
    pub scroll_offset: Vec2,
    pub rotation: Rotation,
    pub flip_x: bool,
    pub flip_y: bool,
    pub pivot: Option<Vec2>,
    pub blend_mode: BlendMode,
    pub y_sort_offset: f32,
}

impl Default for DrawTextureParams {
    fn default() -> DrawTextureParams {
        DrawTextureParams {
            dest_size: None,
            source_rect: None,
            scroll_offset: Vec2::ZERO,
            rotation: Rotation::default(),
            pivot: None,
            flip_x: false,
            flip_y: false,
            blend_mode: BlendMode::None,
            y_sort_offset: 0.0,
        }
    }
}

impl DrawTextureParams {
    pub fn blend(blend_mode: BlendMode) -> DrawTextureParams {
        DrawTextureParams {
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
    Euler(Vec3)
}

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Z(0f32)
    }
}

pub fn rotated_rectangle(
    position: Vec3,
    params: RawDrawParams,
    tex_width: f32,
    tex_height: f32,
    color: Color,
    scroll_offset: Vec2,
) -> [SpriteVertex; 4] {
    // 获取源矩形尺寸
    let dims = params
        .source_rect
        .map(|rect| IRect {
            size: rect.size,
            offset: ivec2(rect.offset.x, rect.offset.y),
        })
        .unwrap_or(IRect::new(
            ivec2(0, 0),
            ivec2(tex_width as i32, tex_height as i32),
        ));

    let sx = dims.offset.x as f32;
    let sy = dims.offset.y as f32;
    let sw = dims.size.x as f32;
    let sh = dims.size.y as f32;

    // 处理目标尺寸和翻转
    let (mut w, mut h) = {
        let size = params.dest_size.unwrap_or((1.0, 1.0).into());
        (size.x, size.y)
    };
    if params.flip_x { w = -w; }
    if params.flip_y { h = -h; }

    // 计算实际尺寸（考虑翻转）
    let abs_w = w.abs();
    let abs_h = h.abs();
    
    // 计算 pivot 偏移（Unity 风格，0-1 范围）
    let pivot_offset = match params.pivot {
        Some(p) => vec2(p.x * abs_w, p.y * abs_h),
        None => vec2(abs_w / 2.0, abs_h / 2.0),
    };

    // 获取旋转角度（只使用Z轴旋转）
    let r = match params.rotation {
        Rotation::Z(angle) => angle,
        Rotation::Euler(v) => v.z,
        _ => 0.0,
    };

    // 计算旋转矩阵（左手坐标系）
    let (sin, cos) = r.sin_cos();
    let rotation_matrix = Mat2::from_cols(
        vec2(cos, sin),
        vec2(-sin, cos)
    );

    // 定义基础顶点（相对于矩形左上角）
    let base_vertices = [
        vec2(0.0, 0.0),      // 左上
        vec2(0.0, h),        // 左下
        vec2(w, h),          // 右下
        vec2(w, 0.0),        // 右上
    ];

    // 应用旋转和平移
    let world_vertices: [Vec2; 4] = base_vertices.map(|v| {
        // 转换为 pivot 相对坐标
        let pivot_relative = v - pivot_offset;
        // 应用旋转
        let rotated = rotation_matrix * pivot_relative;
        // 转换回世界坐标
        rotated + position.truncate() + pivot_offset
    });

    // 创建最终顶点
    [
        SpriteVertex::new(
            vec3(world_vertices[0].x, world_vertices[0].y, position.z),
            vec2(sx / tex_width, sy / tex_height) + scroll_offset,
            color,
        ),
        SpriteVertex::new(
            vec3(world_vertices[1].x, world_vertices[1].y, position.z),
            vec2(sx / tex_width, (sy + sh) / tex_height) + scroll_offset,
            color,
        ),
        SpriteVertex::new(
            vec3(world_vertices[2].x, world_vertices[2].y, position.z),
            vec2((sx + sw) / tex_width, (sy + sh) / tex_height) + scroll_offset,
            color,
        ),
        SpriteVertex::new(
            vec3(world_vertices[3].x, world_vertices[3].y, position.z),
            vec2((sx + sw) / tex_width, sy / tex_height) + scroll_offset,
            color,
        ),
    ]
}
