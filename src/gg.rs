use crate::*;

use anyhow::*;

pub type PipelineMap = HashMap<String, wgpu::RenderPipeline>;
pub type UserPipelineMap = HashMap<String, UserRenderPipeline>;
pub type TextureMap = HashMap<TextureHandle, BindableTexture>;
pub type RenderTargetMap = HashMap<RenderTargetId, UserRenderTarget>;

pub enum RenderPipeline<'a> {
    User(&'a UserRenderPipeline),
    Wgpu(&'a wgpu::RenderPipeline),
}

pub struct UserRenderPipeline {
    pub pipeline: wgpu::RenderPipeline,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub buffers: HashMap<String, wgpu::Buffer>,
}

pub fn depth_stencil_attachment(
    enabled: bool,
    view: &wgpu::TextureView,
    is_first: bool,
) -> Option<wgpu::RenderPassDepthStencilAttachment> {
    let clear_depth = if is_first {
        wgpu::LoadOp::Clear(1.0)
    } else {
        wgpu::LoadOp::Load
    };

    if enabled {
        Some(wgpu::RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(wgpu::Operations {
                load: clear_depth,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        })
    } else {
        None
    }
}

pub fn shader_to_wgpu(shader: &Shader) -> wgpu::ShaderModuleDescriptor<'_> {
    wgpu::ShaderModuleDescriptor {
        label: Some(&shader.name),
        source: wgpu::ShaderSource::Wgsl(shader.source.as_str().into()),
    }
}

pub struct GraphicsContext {
    pub surface: Option<Arc<Surface<'static>>>,
    pub instance: Arc<Instance>,
    pub adapter: Arc<Adapter>,
    pub device: Arc<Device>,
    pub queue: Arc<Queue>,
    pub texture_layout: Arc<BindGroupLayout>,

    pub config: Arc<RwLock<SurfaceConfiguration>>,
    pub textures: Arc<Mutex<TextureMap>>,
}

impl GraphicsContext {
    pub fn resume(&mut self, window: Arc<Window>) {
        // Window size is only actually valid after we enter the event loop.
        let window_size = window.inner_size();
        let width = window_size.width.max(1);
        let height = window_size.height.max(1);

        info!("Surface resume {window_size:?}");

        let surface = self.instance.create_surface(window).unwrap();

        let mut config = self.config.as_ref().write();

        config.width = width;
        config.height = height;

        surface.configure(&self.device, &config);

        self.surface = Some(Arc::new(surface));
    }
}

pub struct WgpuRenderer {
    pub context: GraphicsContext,

    pub pipelines: PipelineMap,
    pub user_pipelines: UserPipelineMap,
    pub shaders: RefCell<ShaderMap>,
    pub render_targets: RefCell<RenderTargetMap>,

    pub vertex_buffer: SizedBuffer,
    pub index_buffer: SizedBuffer,

    pub enable_z_buffer: bool,

    pub textures: Arc<Mutex<TextureMap>>,
    pub texture_layout: Arc<BindGroupLayout>,
    pub depth_texture: Arc<texture::Texture>,
    pub first_pass_texture: BindableTexture,

    pub sprite_shader_id: ShaderId,
    pub error_shader_id: ShaderId,

    pub post_processing_effects: RefCell<Vec<PostProcessingEffect>>,
    pub render_texture_format: wgpu::TextureFormat,

    pub size: PhysicalSize<u32>,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: Buffer,
    pub camera_bind_group: Arc<BindGroup>,
    pub camera_bind_group_layout: BindGroupLayout,

    pub msaa_texture: wgpu::TextureView,

    main_camera: Option<Arc<Mutex<dyn camera::Camera>>>,
}

impl WgpuRenderer {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let context = create_graphics_context(window).await;

        trace!("Loading builtin engine textures");

        {
            let textures = &mut context.textures.lock();

            load_texture_from_engine_bytes(
                &context,
                "1px",
                include_bytes!("assets/1px.png"),
                textures,
                wgpu::AddressMode::Repeat,
            );
        }

        let mut main_camera =
            Camera2D::new(BaseCamera::new(vec3(0.0, 0.0, -1.), 0.01, 10000.0), 540.0);

        main_camera.resize(size);

        // let main_camera = None;

        let camera_uniform = CameraUniform::new();

        let camera_buffer = context
            .device
            .create_buffer_init(&util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });

        let camera_bind_group_layout =
            context
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });

        let camera_bind_group = context.device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let camera_bind_group = Arc::new(camera_bind_group);

        let vertex_buffer = SizedBuffer::new(
            "Mesh Vertex Buffer",
            &context.device,
            1024 * 1024,
            BufferType::Vertex,
        );

        let index_buffer = SizedBuffer::new(
            "Mesh Index Buffer",
            &context.device,
            1024 * 1024,
            BufferType::Index,
        );

        let mut shaders = ShaderMap::new();

        let sprite_shader_id = create_shader(
            &mut shaders,
            "sprite",
            &sprite_shader_from_fragment(include_str!("shaders/sprite.wgsl")),
            HashMap::new(),
        )
        .unwrap();

        let error_shader_id = create_shader(
            &mut shaders,
            "error",
            &sprite_shader_from_fragment(include_str!("shaders/error.wgsl")),
            HashMap::new(),
        )
        .unwrap();

        let render_texture_format = wgpu::TextureFormat::Bgra8UnormSrgb;

        let (width, height) = {
            let config = context.config.read();
            (config.width, config.height)
        };

        let first_pass_texture = BindableTexture::new(
            &context.device,
            &context.texture_layout,
            &TextureCreationParams {
                label: Some("First Pass Texture"),
                width,
                height,
                ..Default::default()
            },
        );

        let depth_texture = texture::Texture::create_depth_texture(
            &context.device,
            &context.config.read(),
            "Depth Texture",
        );

        let hdr_bind_group_layout = create_hdr_bind_group_layout(&context.device);
        let tonemapping_pipeline = create_tonemapping_pipeline(
            &context.device,
            &context.config.read(),
            &hdr_bind_group_layout,
        );
        let hdr_texture = create_hdr_texture(&context.device, &context.config.read());
        let hdr_bind_group =
            create_hdr_bind_group(&context.device, &hdr_bind_group_layout, &hdr_texture);

        let msaa_texture = create_multisampled_framebuffer(
            &context.device,
            &context.config.read(),
            window_config().sample_count.clone().into(),
        );

        let renderer = Self {
            size,

            main_camera: Some(Arc::new(Mutex::new(main_camera))),

            camera_buffer,
            camera_uniform,
            camera_bind_group,
            camera_bind_group_layout,

            render_texture_format,
            post_processing_effects: RefCell::new(Vec::new()),

            pipelines: HashMap::new(),
            user_pipelines: HashMap::new(),

            shaders: RefCell::new(shaders),
            render_targets: RefCell::new(HashMap::new()),

            vertex_buffer,
            index_buffer,
            enable_z_buffer: false,

            sprite_shader_id,
            error_shader_id,

            first_pass_texture,

            depth_texture: Arc::new(depth_texture),
            textures: context.textures.clone(),
            texture_layout: context.texture_layout.clone(),

            context,

            msaa_texture,
        };

        /* {
            let copy_shader_id = create_shader(
                &mut renderer.shaders.borrow_mut(),
                "copy",
                &post_process_shader_from_fragment(COPY_SHADER_SRC),
                HashMap::new(),
            )
            .expect("copy shader creation failed");

            insert_post_processing_effect(&renderer, 0, "copy", copy_shader_id);
        } */

        renderer
    }

    pub(crate) fn resize(&mut self, mut new_size: PhysicalSize<u32>){ //, present_mode: PresentMode) {
        if new_size.width > 0 && new_size.height > 0 {
            new_size.width = new_size.width.max(1);
            new_size.height = new_size.height.max(1);
            self.size = new_size;

            if let Some(main_camera) = &self.main_camera {
                main_camera.lock().resize(new_size);
            }

            if let Some(surface) = &self.context.surface.as_mut() {
                let mut config = self.context.config.write();

                config.width = new_size.width;
                config.height = new_size.height;
                // config.present_mode = present_mode;

                surface.configure(&self.context.device, &config);
            }

            self.update_resources();
        }
    }

    pub(crate) fn set_present_mode(&mut self, present_mode: PresentMode) {
        if let Some(surface) = &self.context.surface.as_mut() {
            let mut config = self.context.config.write();
            config.present_mode = present_mode;
            surface.configure(&self.context.device, &config);
        }
    }

    fn update_resources(&mut self) {
        self.msaa_texture = create_multisampled_framebuffer(
            &self.context.device,
            &self.context.config.read(),
            window_config().sample_count.clone().into(),
        );
    }

    pub(crate) fn update(&mut self) {
        // region: 相机参数设置
        let new_matrix = self.projection_matrix();

        self.camera_uniform.update_matrix(new_matrix);

        // 这里调用后不render会内存溢出
        self.context.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
        // endregion
    }

    pub(crate) fn draw(&mut self) {
        let clear_color = window_config().clear_color;

        // 检查 surface 是否可用
        let output = {
            if let Some(surface) = &self.context.surface.as_mut() {
                match surface.get_current_texture() {
                    core::result::Result::Ok(texture) => texture,
                    Err(_) => return,
                }
            } else {
                return;
            }
        };

        let surface_view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        run_batched_render_passes(
            self,
            clear_color,
            self.sprite_shader_id,
            self.error_shader_id,
        );

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // 解析MSAA纹理到非MSAA的高精度纹理
        if window_config().sample_count != Msaa::Off {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MSAA Resolve Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa_texture,
                    resolve_target: Some(&surface_view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Discard,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            // 注意：我们不需要在这个通道中执行任何绘制操作，因为解析是自动进行的
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));

        output.present();
    }

    pub(crate) fn end_frame(&mut self) {
        self.clear_buffer();
    }

    pub(crate) fn clear_buffer(&mut self) {
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Clear buffer encoder"),
                });

        encoder.clear_buffer(&self.vertex_buffer.buffer, 0, None);
        encoder.clear_buffer(&self.index_buffer.buffer, 0, None);

        self.context.queue.submit(std::iter::once(encoder.finish()));
    }

    pub(crate) fn set_camera(&mut self, camera: impl Camera + Send + Sync + 'static) {
        self.main_camera = Some(Arc::new(Mutex::new(camera)));
    }

    pub(crate) fn set_default_camera(&mut self) {
        self.main_camera = None;
    }

    fn projection_matrix(&self) -> Mat4 {
        if let Some(camera) = &self.main_camera {
            camera.lock().matrix()
        } else {
            self.pixel_perfect_projection_matrix()
        }
    }

    fn pixel_perfect_projection_matrix(&self) -> Mat4 {
        let (width, height) = (self.size.width as f32, self.size.height as f32);

        Mat4::orthographic_rh(0., width, height, 0., -1., 1.)
    }

    pub fn create_material(
        &mut self,
        name: &str,
        source: &str,
        uniform_defs: UniformDefs,
    ) -> Result<ShaderId> {
        create_shader(&mut self.shaders.borrow_mut(), name, source, uniform_defs)
    }
}
