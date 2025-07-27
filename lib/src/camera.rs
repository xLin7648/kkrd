use crate::*;

use std::fmt::Debug;

pub trait Camera: Send + Sync + Debug {
    fn matrix(&self) -> Mat4;
    fn resize(&mut self, new_size: PhysicalSize<u32>);
    fn set_position(&mut self, position: Vec3);
    fn set_rotation(&mut self, rotation: Quat);
    fn set_rotation_angle(&mut self, angle: Vec3);
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
        self.rect.w = self.size * 2.0 * aspect_ratio;
        self.rect.h = self.size * 2.0;
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
