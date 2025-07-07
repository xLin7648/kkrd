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
        clear_background(BLACK);

        // renderer.set_camera(Camera3D::new(BaseCamera::new(vec3(0.0, 0.0, -90.), 0.01, 10000.0), 60.0));
        // renderer.set_default_camera();

        self.r = 187.841705;
    }

    async fn update(&mut self) {
        clear_background(BLUE);
        draw_rect_rot(vec2(self.r, 0.), vec2(1920., 3.), 90f32.to_radians(), WHITE, 0);


        self.r = 500.0 * get_time().sin();
    }
}