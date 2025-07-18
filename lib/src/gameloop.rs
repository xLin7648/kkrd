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

        draw_sprite_ex(
            texture_id("Tap"),
            DrawTextureParams {
                raw_draw_params: RawDrawParams {
                    scale: Some(vec2(0.2, 0.2)),
                    rotation: Default::default(),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            }
        );

        // draw_rect_rot(
        //     0., 0.,
        //     5., 1080.,
        //     Rotation::Euler(0., 0., 0.),
        //     vec2(0.5, 0.5),
        //     WHITE,
        //     0,
        // );

        let abs_difference = (time::get_time().sin() - 1.0).abs();
        self.r = abs_difference * 100.0;
    }
}

#[async_trait]
impl GameLoop for MyGame {
    async fn start(&mut self) {
        clear_background(BLACK);

        let base_camera = BaseCamera::new(vec3(0.0, 0.0, -100.), 0.01, 10000.0);
        let main_camera = Camera2D::new(base_camera, 540.0);
        set_camera(main_camera);
    }

    async fn update(&mut self) {
        self.line().await;
    }
}
