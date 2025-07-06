use crate::*;

pub trait GameLoop {
    fn start(&mut self);
    fn update(&mut self);
}

#[derive(Default)]
pub struct MyGame {
    pub r: f32,
}

impl GameLoop for MyGame {
    fn start(&mut self) {
        clear_background(BLACK);

        // renderer.set_camera(Camera3D::new(BaseCamera::new(vec3(0.0, 0.0, -90.), 0.01, 10000.0), 60.0));
        // renderer.set_default_camera();

        self.r = 187.841705;

        
    }

    fn update(&mut self) {
        draw_rect_rot(vec2(self.r, 0.), vec2(1920., 3.), 90f32.to_radians(), WHITE, 0);


        self.r = 500.0 * get_time().sin();
    }
}