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
    pub my_render_target: Option<RenderTargetId>,
}

impl MyGame {}

#[async_trait]
impl GameLoop for MyGame {
    async fn start(&mut self) {
        let base_camera = BaseCamera::new(vec3(0.0, 0.0, -100.), 0.01, 10000.0);
        let main_camera = Camera2D::new(base_camera, 540.0);
        set_camera(main_camera);

        self.glitch_shader_id =
            Some(create_shader("glitch", &include_str!("shaders/glitch.wgsl")).unwrap());

        self.my_render_target = Some(UserRenderTarget::new(&RenderTargetParams {
            label: "my-render-target".to_string(),
            size: uvec2(640, 360),
            filter_mode: wgpu::FilterMode::Nearest,
        }));
    }

    async fn update(&mut self) {
        let shader_id = self.glitch_shader_id.unwrap();
        let render_target_id = self.my_render_target.unwrap();

        use_render_target(render_target_id);

        draw_sprite_ex(
            texture_id("Tap"),
            DrawTextureParams {
                raw_draw_params: RawDrawParams {
                    position: vec3(200.0, 200.0, 0.0),
                    scale: vec2(0.5, 0.5),
                    rotation: Default::default(),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        clear_background(BLUE);

        draw_sprite_ex(
            texture_id("Tap"),
            DrawTextureParams {
                raw_draw_params: RawDrawParams {
                    scale: vec2(0.5, 0.5),
                    rotation: Default::default(),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        use_default_render_target();

        clear_background(RED);
        
        use_shader(shader_id);

        set_uniform("power", Uniform::F32(OrderedFloat::<f32>(0.03)));
        set_uniform("rate", Uniform::F32(OrderedFloat::<f32>(0.6)));
        set_uniform("speed", Uniform::F32(OrderedFloat::<f32>(5.0)));
        set_uniform("blockCount", Uniform::F32(OrderedFloat::<f32>(30.5)));
        set_uniform("colorRate", Uniform::F32(OrderedFloat::<f32>(0.01)));

        draw_sprite_ex(
            TextureHandle::RenderTarget(render_target_id),
            DrawTextureParams {
                raw_draw_params: RawDrawParams { 
                    position: Vec3::ZERO,
                    rotation: Rotation::Z(45.0),
                    dest_size: Some(uvec2(640, 360)),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        use_default_shader();
    }
}
