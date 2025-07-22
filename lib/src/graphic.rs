use crate::*;

use anyhow::Result;
use wgpu::{AddressMode, CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderModuleDescriptor, ShaderSource, StoreOp, TextureView, TextureViewDescriptor};

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
    pub layout: BindGroupLayout,
    pub bind_group: BindGroup,
    pub buffers: HashMap<String, Buffer>,
}

pub fn depth_stencil_attachment(
    enabled: bool,
    view: &TextureView,
    is_first: bool,
) -> Option<RenderPassDepthStencilAttachment> {
    let clear_depth = if is_first {
        LoadOp::Clear(1.0)
    } else {
        LoadOp::Load
    };

    if enabled {
        Some(RenderPassDepthStencilAttachment {
            view,
            depth_ops: Some(Operations {
                load: clear_depth,
                store: StoreOp::Store,
            }),
            stencil_ops: None,
        })
    } else {
        None
    }
}

pub fn shader_to_wgpu(shader: &Shader) -> ShaderModuleDescriptor<'_> {
    ShaderModuleDescriptor {
        label: Some(&shader.name),
        source: ShaderSource::Wgsl(shader.source.as_str().into()),
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
    pub shaders: Arc<Mutex<ShaderMap>>,
    pub render_targets: Arc<Mutex<RenderTargetMap>>,

    pub vertex_buffer: SizedBuffer,
    pub index_buffer: SizedBuffer,

    pub enable_z_buffer: bool,

    pub textures: Arc<Mutex<TextureMap>>,
    pub texture_layout: Arc<BindGroupLayout>,
    
    pub sprite_shader_id: ShaderId,
    pub error_shader_id: ShaderId,
    
    pub size: PhysicalSize<u32>,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: Buffer,
    pub camera_bind_group: Arc<BindGroup>,
    pub camera_bind_group_layout: BindGroupLayout,

    pub msaa_texture: TextureView,
    pub msaa_depth_texture: TextureView,
    pub depth_texture: TextureView,
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
                AddressMode::Repeat,
            );

            load_texture_from_engine_bytes(
                &context,
                "Tap",
                include_bytes!("assets/Tap2.png"),
                textures,
                AddressMode::Repeat,
            );

            load_texture_from_engine_bytes(
                &context,
                "1",
                include_bytes!("assets/1.png"),
                textures,
                AddressMode::Repeat,
            );
        }

        // let mut main_camera =
        //     Camera2D::new(BaseCamera::new(vec3(0.0, 0.0, -1.), 0.01, 10000.0), 540.0);

        // main_camera.resize(size);

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

        let sprite_shader_id = create_shader1(
            &mut shaders,
            "sprite",
            &include_str!("shaders/sprite.wgsl")
        )
        .unwrap();

        let error_shader_id = create_shader1(
            &mut shaders,
            "error",
            &include_str!("shaders/error.wgsl")
        )
        .unwrap();

        let (width, height) = {
            let config = context.config.read();
            (config.width, config.height)
        };

        let bind = get_run_time_context();
        let run_time_context = bind.read();

        let depth_texture = create_multisampled_depth(
            &context.device,
            &context.config.read(),
            1
        );

        let msaa_depth_texture = create_multisampled_depth(
            &context.device,
            &context.config.read(),
            run_time_context.sample_count.into()
        );

        let msaa_texture = create_multisampled_framebuffer(
            &context.device,
            &context.config.read(),
            run_time_context.sample_count.into(),
        );

        let renderer = Self {
            size,

            camera_buffer,
            camera_uniform,
            camera_bind_group,
            camera_bind_group_layout,

            pipelines: HashMap::new(),
            user_pipelines: HashMap::new(),

            shaders: Arc::new(Mutex::new(shaders)),
            render_targets: Arc::new(Mutex::new(HashMap::new())),

            vertex_buffer,
            index_buffer,
            enable_z_buffer: true,

            sprite_shader_id,
            error_shader_id,

            textures: context.textures.clone(),
            texture_layout: context.texture_layout.clone(),

            context,

            depth_texture,
            msaa_texture,
            msaa_depth_texture,
        };

        renderer
    }

    pub(crate) fn resize(&mut self, mut new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            new_size.width = new_size.width.max(1);
            new_size.height = new_size.height.max(1);
            self.size = new_size;

            if let Some(main_camera) = &get_run_time_context().read().main_camera {
                main_camera.write().resize(new_size);
            }

            if let Some(surface) = &self.context.surface.as_mut() {
                let mut config = self.context.config.write();

                config.width = new_size.width;
                config.height = new_size.height;
                // config.present_mode = present_mode;

                surface.configure(&self.context.device, &config);
            }

            self.update_resources(get_run_time_context().read().sample_count.into());
        }
    }

    pub(crate) fn set_present_mode(&mut self, present_mode: PresentMode) {
        if let Some(surface) = &self.context.surface.as_mut() {
            let mut config = self.context.config.write();
            config.present_mode = present_mode;
            surface.configure(&self.context.device, &config);
        }
    }

    fn update_resources(&mut self, sample_count: u32) {
        self.msaa_texture = create_multisampled_framebuffer(
            &self.context.device,
            &self.context.config.read(),
            sample_count,
        );

        self.depth_texture = create_multisampled_depth(
            &self.context.device,
            &self.context.config.read(),
            1
        );

        self.msaa_depth_texture = create_multisampled_depth(
            &self.context.device,
            &self.context.config.read(),
            sample_count
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
        // 检查 surface 是否可用
        let output = {
            if let Some(surface) = &self.context.surface.as_mut() {
                match surface.get_current_texture() {
                    Ok(texture) => texture,
                    Err(_) => return,
                }
            } else {
                return;
            }
        };

        let surface_view = output
            .texture
            .create_view(&TextureViewDescriptor::default());

        let (sample_count, clear_color) = {
            let bind = get_run_time_context();
            let read = bind.read();

            (read.sample_count, read.clear_color)
        };

        run_batched_render_passes(
            self,
            sample_count,
            clear_color,
            self.sprite_shader_id,
            self.error_shader_id,
            &surface_view
        );

        // 解析MSAA纹理到非MSAA的高精度纹理
        if sample_count != Msaa::Off {
            let mut encoder =
                self.context
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Msaa Encoder"),
                    });

            encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("MSAA Resolve Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &self.msaa_texture,
                    resolve_target: Some(&surface_view),
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.msaa_depth_texture, // MSAA 深度附件
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            self.context.queue.submit(std::iter::once(encoder.finish()));
            // 注意：我们不需要在这个通道中执行任何绘制操作，因为解析是自动进行的
        }

        output.present();
    }

    pub(crate) fn end_frame(&mut self) {
        self.clear_buffer();
    }

    pub(crate) fn clear_buffer(&mut self) {
        let mut encoder =
            self.context
                .device
                .create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("Clear buffer encoder"),
                });

        encoder.clear_buffer(&self.vertex_buffer.buffer, 0, None);
        encoder.clear_buffer(&self.index_buffer.buffer, 0, None);

        self.context.queue.submit(std::iter::once(encoder.finish()));
    }

    fn projection_matrix(&self) -> Mat4 {
        if let Some(camera) = &get_run_time_context().read().main_camera {
            camera.read().matrix()
        } else {
            self.pixel_perfect_projection_matrix()
        }
    }

    fn pixel_perfect_projection_matrix(&self) -> Mat4 {
        let (width, height) = (self.size.width as f32, self.size.height as f32);

        Mat4::orthographic_rh(0., width, height, 0., -1., 1.)
    }
}
