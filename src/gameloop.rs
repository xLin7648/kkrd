use crate::*;

#[async_trait]
pub trait GameLoop : Send {
    async fn start(&mut self);
    async fn update(&mut self);
}

#[derive(Default)]
pub struct MyGame {
    pub r: f32,
}

impl MyGame {
    pub async fn line(&mut self)
    {
        draw_rect_rot(Vec2::ZERO, vec2(1920., 3.), self.r, WHITE, 0);
        draw_rect_rot(Vec2::ZERO, vec2(1920., 3.), -self.r, WHITE, 0);
        draw_rect_rot(Vec2::ZERO, vec2(1920., 3.), self.r + 90.0, WHITE, 0);
        draw_rect_rot(Vec2::ZERO, vec2(1920., 3.), -(self.r + 90.0), WHITE, 0);
        self.r = 5.0 * get_time().sin();
    }
}

#[async_trait]
impl GameLoop for MyGame {
    async fn start(&mut self) {
        clear_background(RED);

        let main_camera =
            Camera2D::new(BaseCamera::new(vec3(0.0, 0.0, -1.), 0.01, 10000.0), 540.0);

        set_camera(main_camera);

        self.r = 187.841705;
    }

    async fn update(&mut self) {
        self.line().await;
    }
}