use crate::*;

pub trait GameLoop {
    async fn start(&mut self);
    async fn update(&mut self);
}

#[derive(Default)]
pub struct MyGame {
    pub r: f32,
}

impl GameLoop for MyGame {
    async fn start(&mut self) {
        clear_background(RED);

        // renderer.set_camera(Camera3D::new(BaseCamera::new(vec3(0.0, 0.0, -90.), 0.01, 10000.0), 60.0));
        // renderer.set_default_camera();

        self.r = 187.841705;
    }

    async fn update(&mut self) {
        draw_rect_rot(Vec2::ZERO, vec2(1920., 3.), self.r, WHITE, 0);


        self.r = 5.0 * get_time().sin();
    }
}