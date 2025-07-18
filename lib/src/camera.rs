use crate::*;

use std::fmt::Debug;

pub trait Camera: Send + Sync + Debug {
    fn matrix(&self) -> Mat4;
    fn resize(&mut self, new_size: PhysicalSize<u32>);
    fn set_position(&mut self, position: Vec3);
    fn set_rotation(&mut self, rotation: Quat);
    fn set_rotation_angle(&mut self, angle: Vec3);
    fn world_to_screen(&self, world_position: Vec3) -> Vec2;
    fn screen_to_world(&self, screen_position: Vec2) -> Vec3;
}

#[derive(Debug)]
pub struct BaseCamera {
    pos: Vec3,
    rot: Quat, // 修改为保存旋转的四元数
    target: Vec3,
    near: f32,
    far: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl BaseCamera {
    pub fn new(pos: Vec3, near: f32, far: f32) -> Self {
        let mut camera = Self {
            pos,
            near,
            far,
            target: Vec3::ZERO,
            rot: Quat::IDENTITY, // 初始化为身份四元数
            viewport_width: 0.0,
            viewport_height: 0.0,
        };
        camera.update_target(); // 初始化目标
        camera
    }

    // 设置位置，同时更新目标
    pub fn set_position(&mut self, position: Vec3) {
        self.pos = position;
        self.update_target();
    }

    // 设置旋转，同时更新目标，参数从 Vec3 更改为 Quat
    pub fn set_rotation(&mut self, rotation: Quat) {
        self.rot = rotation;
        self.update_target(); // 更新目标方向
    }

    pub fn set_rotation_angle(&mut self, angle: Vec3) {
        // 将欧拉角转换为四元数
        self.rot = Quat::from_euler(
            EulerRot::XYZ, 
            angle.x.to_radians(), 
            angle.y.to_radians(), 
            angle.z.to_radians()
        );
        self.update_target(); // 更新目标
    }

    fn resize(&mut self, new_width: u32, new_height: u32) {
        self.viewport_width = new_width as f32;
        self.viewport_height = new_height as f32;
    }

    // 更新目标位置
    fn update_target(&mut self) {
        let direction = self.rot * Vec3::Z;
        self.target = self.pos + direction;
    }
}

impl Default for BaseCamera {
    fn default() -> Self {
        Self::new(Vec3::ZERO, 0.01, 1000.0)
    }
}

#[derive(Debug)]
pub struct Camera3D {
    base: BaseCamera,
    fovy: f32,
    aspect: f32,
}

impl Camera3D {
    pub fn new(base: BaseCamera, fovy: f32) -> Self {
        Self {
            base,
            fovy,
            aspect: 0.0,
        }
    }
}

impl Camera for Camera3D {
    fn matrix(&self) -> Mat4 {
        let base = &self.base;
        let up = base.rot * Vec3::Y;
        // 保持右手坐标系函数
        let view = Mat4::look_at_lh(base.pos, base.target, up);
        let proj = Mat4::perspective_lh(self.fovy.to_radians(), self.aspect, base.near, base.far);
        proj * view
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.base.resize(new_size.width, new_size.height);
        self.aspect = new_size.width as f32 / new_size.height as f32; // 更新宽高比
    }

    fn set_rotation(&mut self, rotation: Quat) { // 修改为 Quat 类型
        self.base.set_rotation(rotation);
    }

    fn set_rotation_angle(&mut self, angle: Vec3) {
        self.base.set_rotation_angle(angle); // 调用 BaseCamera 的方法
    }
    
    fn set_position(&mut self, position: Vec3) {
        self.base.set_position(position);
    }

    fn world_to_screen(&self, world_position: Vec3) -> Vec2 {
        let view_proj = self.matrix();
        let clip_space_position = view_proj * world_position.extend(1.0); // 转换为裁剪空间
        
        // 将裁剪空间转换为 NDC
        let ndc_x = clip_space_position.x / clip_space_position.w;
        let ndc_y = clip_space_position.y / clip_space_position.w;

        // 将 NDC 转换为屏幕坐标
        let screen_x = (ndc_x * 0.5 + 0.5) * self.base.viewport_width;
        let screen_y = (ndc_y * 0.5 + 0.5) * self.base.viewport_height;

        vec2(screen_x, self.base.viewport_height - screen_y) // Y轴翻转
    }

    fn screen_to_world(&self, screen_position: Vec2) -> Vec3 {
        // 将屏幕坐标转换为 NDC
        let ndc_x = (screen_position.x / self.base.viewport_width) * 2.0 - 1.0;
        let ndc_y = (screen_position.y / self.base.viewport_height) * 2.0 - 1.0;

        // 创建裁剪空间位置
        let clip_space_position = vec4(ndc_x, ndc_y, -1.0, 1.0); // Z 设置为 -1

        // 计算逆视图投影矩阵
        let view_proj_inverse = self.matrix().inverse();
        let world_position = view_proj_inverse * clip_space_position;

        world_position.truncate() // 丢弃 w 组件
    }
}

#[derive(Debug)]
pub struct Camera2D {
    base: BaseCamera,
    rect: Rect,
    size: f32,
}

impl Camera2D {
    pub fn new(base: BaseCamera, size: f32) -> Self {
        Self {
            base,
            size,
            rect: Rect::default(), // 确保 rect 被初始化
        }
    }
}

impl Camera for Camera2D {
    fn matrix(&self) -> Mat4 {
        let base = &self.base;
        let up = base.rot * Vec3::Y;
        // 保持左手坐标系函数
        let view = Mat4::look_at_lh(base.pos, base.target, up);
        let proj = Mat4::orthographic_lh(
            self.rect.x,
            self.rect.x + self.rect.w,
            self.rect.y - self.rect.h,
            self.rect.y,
            base.near,
            base.far
        );
        proj * view
    }

    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.base.resize(new_size.width, new_size.height);

        // 计算宽高比
        let aspect_ratio = new_size.width as f32 / new_size.height as f32;
        // 更新正交矩形
        self.rect.w = self.size * aspect_ratio;
        self.rect.h = self.size;
        self.rect.x = -self.rect.w / 2.0;
        self.rect.y =  self.rect.h / 2.0;
    }

    fn set_position(&mut self, position: Vec3) {
        self.base.set_position(position);
    }

    fn set_rotation(&mut self, rotation: Quat) { // 修改为 Quat 类型
        self.base.set_rotation(rotation);
    }

    fn set_rotation_angle(&mut self, angle: Vec3) {
        self.base.set_rotation_angle(angle); // 调用 BaseCamera 的方法
    }

    fn world_to_screen(&self, world_position: Vec3) -> Vec2 {
        let view_proj = self.matrix();
        let clip_space_position = view_proj * world_position.extend(1.0); // 转换为裁剪空间
        let ndc_x = clip_space_position.x / clip_space_position.w;
        let ndc_y = clip_space_position.y / clip_space_position.w;

        // 将 NDC 转换为屏幕坐标
        let screen_x = (ndc_x * 0.5 + 0.5) * self.rect.w; // 使用 rect 的宽度
        let screen_y = (ndc_y * 0.5 + 0.5) * self.rect.h; // 使用 rect 的高度

        vec2(screen_x, self.rect.h - screen_y) // Y轴翻转
    }

    fn screen_to_world(&self, screen_position: Vec2) -> Vec3 {
        let ndc_x = (screen_position.x / self.rect.w) * 2.0 - 1.0;
        let ndc_y = (screen_position.y / self.rect.h) * 2.0 - 1.0;

        let clip_space_position = vec4(ndc_x, ndc_y, -1.0, 1.0); // Z 设置为 -1

        let view_proj_inverse = self.matrix().inverse();
        let world_position = view_proj_inverse * clip_space_position;

        world_position.truncate() // 丢弃 w 组件
    }
}

// 用于相机的统一缓存
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &dyn Camera) {
        self.view_proj = camera.matrix().to_cols_array_2d();
    }

    pub fn update_matrix(&mut self, matrix: Mat4) {
        self.view_proj = matrix.to_cols_array_2d();
    }
}
