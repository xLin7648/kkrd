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

        self.my_render_target = Some(create_render_target(&RenderTargetParams {
            label: "my-render-target".to_string(),
            size: uvec2(1280, 720),
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
                    scale: vec2(0.2, 0.2),
                    rotation: Default::default(),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        draw_sprite_ex(
            texture_id("Tap"),
            DrawTextureParams {
                raw_draw_params: RawDrawParams {
                    scale: vec2(0.2, 0.2),
                    rotation: Default::default(),
                    pivot: None,
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        use_default_render_target();
        
        use_shader(shader_id);

        // var<uniform> power: f32;
        // var<uniform> rate: f32;
        // var<uniform> speed: f32;
        // var<uniform> blockCount: f32;
        // var<uniform> colorRate: f32;

        set_uniform("power", Uniform::F32(OrderedFloat::<f32>(0.03)));
        set_uniform("rate", Uniform::F32(OrderedFloat::<f32>(0.6)));
        set_uniform("speed", Uniform::F32(OrderedFloat::<f32>(5.0)));
        set_uniform("blockCount", Uniform::F32(OrderedFloat::<f32>(30.5)));
        set_uniform("colorRate", Uniform::F32(OrderedFloat::<f32>(0.01)));

        draw_sprite_ex(
            TextureHandle::RenderTarget(render_target_id),
            DrawTextureParams {
                raw_draw_params: RawDrawParams { 
                    dest_size: Some(uvec2(1280, 720)),
                    ..Default::default()
                },
                ..Default::default()
            },
        );

        use_default_shader();
    }
}
