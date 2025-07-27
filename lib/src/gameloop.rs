use crate::*;

#[async_trait]
pub trait GameLoop: Send {
    async fn start(&mut self);
    async fn update(&mut self);
}

#[derive(Default)]
pub struct MyGame {
    pub r: f32,
    pub glitch_shader_id: Option<ShaderId>,
    pub my_render_target1: Option<RenderTargetId>,
    pub my_render_target2: Option<RenderTargetId>,
}

impl MyGame {}

#[async_trait]
impl GameLoop for MyGame {
    async fn start(&mut self) {
        let base_camera = BaseCamera::new(vec3(0.0, 0.0, -100.), 0.01, 10000.0);
        let main_camera: Camera2D = Camera2D::new(base_camera, 540.0);

        set_camera(main_camera);

        self.glitch_shader_id =
            Some(create_shader("glitch", &include_str!("shaders/glitch.wgsl")).unwrap());

        self.my_render_target1 = Some(UserRenderTarget::new(&RenderTargetParams {
            label: "my-render-target1".to_string(),
            size: uvec2(1280, 720),
        }));

        self.my_render_target2 = Some(UserRenderTarget::new(&RenderTargetParams {
            label: "my-render-target2".to_string(),
            size: uvec2(1280, 720),
        }));
    }

    async fn update(&mut self) {
        // TODO: 需要解决 render_target 分辨率映射错误问题
        // 我认为应该重构相机模块


        let shader_id = self.glitch_shader_id.unwrap();
        let render_target1_id = self.my_render_target1.unwrap();
        let render_target2_id = self.my_render_target2.unwrap();

        {
            use_render_target(render_target1_id);

            clear_background(BLACK);

            draw_sprite_ex(
                texture_id("1px"),
                DrawTextureParams {
                    raw_draw_params: RawDrawParams {
                        dest_size: Some(uvec2(1280, 720)),
                        color: BLUE,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );

            draw_sprite_ex(
                texture_id("Tap"),
                DrawTextureParams {
                    raw_draw_params: RawDrawParams {
                        position: vec3(200.0, 200.0, 0.0),
                        scale: vec2(0.5, 0.5),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );

            draw_sprite_ex(
                texture_id("1px"),
                DrawTextureParams {
                    raw_draw_params: RawDrawParams {
                        position: vec3(-640.0, -360.0, 0.0),
                        scale: vec2(0.5, 0.5),
                        dest_size: Some(uvec2(50, 50)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        }

        // {
        //     use_render_target(render_target2_id);
        //     use_shader(shader_id);

        //     clear_background(RED);

        //     set_uniform("power", Uniform::F32(OrderedFloat::<f32>(0.03)));
        //     set_uniform("rate", Uniform::F32(OrderedFloat::<f32>(0.6)));
        //     set_uniform("speed", Uniform::F32(OrderedFloat::<f32>(5.0)));
        //     set_uniform("blockCount", Uniform::F32(OrderedFloat::<f32>(30.5)));
        //     set_uniform("colorRate", Uniform::F32(OrderedFloat::<f32>(0.01)));

        //     draw_sprite_ex(
        //         TextureHandle::RenderTarget(render_target1_id),
        //         DrawTextureParams {
        //             raw_draw_params: RawDrawParams { 
        //                 position: Vec3::ZERO,
        //                 rotation: Rotation::Zero,
        //                 pivot: Some(vec2(0.5, 0.5)),
        //                 ..Default::default()
        //             },
        //             ..Default::default()
        //         },
        //     );

        //     use_default_shader();
        // }

        use_default_render_target();
        clear_background(RED);

        draw_sprite_ex(
            TextureHandle::RenderTarget(render_target1_id),
            DrawTextureParams {
                raw_draw_params: RawDrawParams { 
                    position: Vec3::ZERO,
                    rotation: Rotation::Zero,
                    pivot: Some(vec2(0.5, 0.5)),
                    dest_size: Some(uvec2(1280, 720)),
                    ..Default::default()
                },
                ..Default::default()
            },
        );
    }
}
