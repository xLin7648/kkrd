use crate::*;

#[async_trait]
pub trait GameLoop: Send {
    async fn start(&mut self);
    async fn update(&mut self);
}

#[derive(Default)]
pub struct MyGame {
    pub r: f32,
}

impl MyGame {
    pub async fn line(&mut self) {
        // if let Some(cam) = get_camera() {
        //     cam.write()
        //         .set_rotation_angle(vec3(0., 0., time::get_time() * 20.0));
        // }

        // draw_sprite_ex(
        //     texture_id("Tap"),
        //     Vec2::ZERO,
        //     WHITE,
        //     0,
        //     DrawTextureParams {
        //         dest_size: Some(vec2(989. * 0.2, 100. * 0.2)),
        //         pivot: Some(vec2(0.5, 0.5)),
        //         ..Default::default()
        //     },
        // );

        draw_rect_rot(
            Vec2::ZERO,
            vec2(500., 500.),
            0.0,
            BLUE,
            0,
        );
    }
}

#[async_trait]
impl GameLoop for MyGame {
    async fn start(&mut self) {
        clear_background(BLACK);

        let base_camera = BaseCamera::new(vec3(0.0, -200.0, -1000.), 0.01, 10000.0);
        let main_camera = Camera3D::new(base_camera, 60.0);
        set_camera(main_camera);
        self.r = 187.841705;
    }

    async fn update(&mut self) {
        self.line().await;
    }
}
