use crate::*;

use std::ops::Mul;

#[derive(Default, Clone, Copy, Debug)]
pub struct Rect
{
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Mul<f32> for Rect {
    type Output = Rect;

    fn mul(self, scalar: f32) -> Rect {
        Rect {
            x: self.x * scalar,
            y: self.y * scalar,
            w: self.w * scalar,
            h: self.h * scalar,
        }
    }
}

// 也可以实现反向乘法，让 f32 也能乘以 Rect
impl Mul<Rect> for f32 {
    type Output = Rect;

    fn mul(self, rect: Rect) -> Rect {
        Rect {
            x: rect.x * self,
            y: rect.y * self,
            w: rect.w * self,
            h: rect.h * self,
        }
    }
}

impl Rect {
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.x && point.x <= self.x + self.w &&
        point.y >= self.y && point.y <= self.y + self.h
    }
}

#[derive(Copy, Clone, Debug)]
pub struct IRect {
    pub offset: IVec2,
    pub size: IVec2,
}

impl IRect {
    pub fn new(offset: IVec2, size: IVec2) -> Self {
        IRect { offset, size }
    }
}