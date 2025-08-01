use std::default;

use crate::*;

use anyhow::Result;
use tokio::sync::watch::error;
use wgpu::{
    AddressMode, BindingResource, BlendState, ColorTargetState, ColorWrites, CommandEncoderDescriptor, FragmentState, IndexFormat, LoadOp, MultisampleState, Operations, PrimitiveState, RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor, ShaderModuleDescriptor, ShaderSource, StoreOp, TextureView, TextureViewDescriptor, VertexState
};

pub type PipelineMap = HashMap<String, wgpu::RenderPipeline>;
pub type UserPipelineMap = HashMap<String, UserRenderPipeline>;
pub type TextureMap = HashMap<TextureHandle, BindableTexture>;
pub type RenderTargetMap = HashMap<RenderTargetId, Arc<RwLock<UserRenderTarget>>>;

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

// pub fn depth_stencil_attachment(
//     enabled: bool,
//     view: &TextureView,
//     is_first: bool,
// ) -> Option<RenderPassDepthStencilAttachment> {
//     let clear_depth = if is_first {
//         LoadOp::Clear(1.0)
//     } else {
//         LoadOp::Load
//     };

//     if enabled {
//         Some(RenderPassDepthStencilAttachment {
//             view,
//             depth_ops: Some(Operations {
//                 load: clear_depth,
//                 store: StoreOp::Store,
//             }),
//             stencil_ops: None,
//         })
//     } else {
//         None
//     }
// }

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

pub fn create_default_rt() {
    let size = get_window_size();

    UserRenderTarget::new(&RenderTargetParams {
        label: "Default RT".to_owned(),
        size: uvec2(size.width.max(1), size.height.max(1)),
    });
}

pub struct WgpuRenderer {
    pub context: GraphicsContext,

    pub pipelines: PipelineMap,
    pub user_pipelines: UserPipelineMap,
    pub shaders: Arc<Mutex<ShaderMap>>,

    pub vertex_buffer: SizedBuffer,
    pub index_buffer: SizedBuffer,

    pub enable_z_buffer: bool,

    pub textures: Arc<Mutex<TextureMap>>,
    pub texture_layout: Arc<BindGroupLayout>,

    pub sprite_shader_id: ShaderId,
    pub error_shader_id: ShaderId,

    pub size: UVec2,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: Buffer,
    pub camera_bind_group: Arc<BindGroup>,
    pub camera_bind_group_layout: BindGroupLayout,

    pub blit_pipeline: Option<wgpu::RenderPipeline>,
}

impl WgpuRenderer {
    pub async fn new(window: Arc<Window>) {
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

        let sprite_shader_id =
            create_shader1(&mut shaders, "sprite", &include_str!("shaders/sprite.wgsl")).unwrap();

        let error_shader_id =
            create_shader1(&mut shaders, "error", &include_str!("shaders/error.wgsl")).unwrap();

        let size = uvec2(size.width.max(1), size.height.max(1));

        let wr = Arc::new(RwLock::new(Self {
            size,

            camera_buffer,
            camera_uniform,
            camera_bind_group,
            camera_bind_group_layout,

            pipelines: HashMap::new(),
            user_pipelines: HashMap::new(),

            shaders: Arc::new(Mutex::new(shaders)),

            vertex_buffer,
            index_buffer,
            enable_z_buffer: true,

            sprite_shader_id,
            error_shader_id,

            textures: context.textures.clone(),
            texture_layout: context.texture_layout.clone(),

            context,

            blit_pipeline: None,
        }));

        let _ = WGPU_RENDERER.set(wr.clone());

        create_default_rt();

        wr.write().resize(size, true);
    }

    pub(crate) fn resize(&mut self, size: UVec2, is_first: bool) {
        if !is_first && self.size == size { return; }

        self.size = size;

        // 相机固定尺寸不参与缩放
        // if let Some(main_camera) = &get_run_time_context().read().main_camera {
        //     main_camera.lock().resize(size);
        // }

        if let Some(surface) = &self.context.surface.as_mut() {
            let mut config = self.context.config.write();

            config.width = size.x;
            config.height = size.y;
            // config.present_mode = present_mode;

            surface.configure(&self.context.device, &config);
        }

        self.update_resources();
        
    }

    // 创建渲染管线
    pub fn create_blit_pipeline(&mut self) {
        let c = &self.context;

        // 1. 创建着色器模块
        let shader = c.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/blit.wgsl").into()),
        });

        // 2. 创建管线布局
        let pipeline_layout = c
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Blit Pipeline Layout"),
                bind_group_layouts: &[&self.texture_layout, &self.camera_bind_group_layout],
                push_constant_ranges: &[],
            });

        // 3. 配置渲染管线
        self.blit_pipeline = Some(c.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("Blit Render Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[SpriteVertex::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: *DEFAULT_TEXTURE_FORMAT.get().unwrap(),
                        blend: Some(BlendState::REPLACE),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Cw,   // 顺时针为正面
                    cull_mode: Some(wgpu::Face::Back), // 背面剔除
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None, // 无深度测试
                multisample: MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            },
        ));
    }

    fn update_resources(&mut self) {
        if self.blit_pipeline.is_none() {
            self.create_blit_pipeline();
        }

        // 更新零号rt
        let size = get_window_size();

        let rts = get_global_render_targets().read();
        let default_rt = rts.get(&RenderTargetId(0)).unwrap();

        default_rt.write().update(
            &self.context,
            &self.texture_layout,
            &RenderTargetParams {
                label: "Default RT".to_owned(),
                size: uvec2(size.width.max(1), size.height.max(1)),
            },
        );
    }

    pub(crate) fn update_camera_buffer(&mut self) {
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

    pub(crate) fn clear(&mut self, clear_color: Color) {
        let cur_rt_id = get_current_render_target();
        let rts = get_global_render_targets().read();
        let cur_rt = rts.get(&cur_rt_id).unwrap().read();

        let w_clear_color: wgpu::Color = clear_color.into();

        // 5. 创建 encoder & render pass
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some(&format!(
                    "RT: {} Clear Color: {:?} Encoder",
                    cur_rt_id.0, w_clear_color
                )),
            });

        let (color_view, depth_view) = if get_run_time_context().read().sample_count != Msaa::Off {
            (&cur_rt.msaa_view, &cur_rt.msaa_depth_view)
        } else {
            (&cur_rt.resolve_view, &cur_rt.msaa_depth_view)
        };

        encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Mesh Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color_view, // MSAA 视图
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(w_clear_color),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: if self.enable_z_buffer {
                Some(RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: StoreOp::Store,
                    }),
                    stencil_ops: None,
                })
            } else {
                None
            },
            ..Default::default()
        });

        self.context.queue.submit(std::iter::once(encoder.finish()));
    }

    pub(crate) fn draw(&mut self) {
        let output = match self.context.surface.as_ref().unwrap().get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let surface_view = output.texture.create_view(&Default::default());

        let sample_count = get_run_time_context().read().sample_count;

        // 1. 场景渲染
        run_batched_render_passes(
            self,
            sample_count,
            self.sprite_shader_id,
            self.error_shader_id,
        );

        let rts = get_global_render_targets().read();

        // 2. 将默认 RT绘制到Surface上
        let default_rt = rts
            .get(&RenderTargetId(0))
            .unwrap_or_else(|| panic!("No Default RendererTarget"))
            .read();

        const QUAD_INDICES_U32: &[u32] = &[0, 1, 2, 0, 2, 3];

        let (half_w, half_h) = (self.size.x as f32 / 2.0, self.size.y as f32 / 2.0);

        let all_vertices: [SpriteVertex; 4] = [
            SpriteVertex::new(vec3(-half_w, -half_h, 0.0), vec2(0.0, 1.0), WHITE),
            SpriteVertex::new(vec3(-half_w,  half_h, 0.0), vec2(0.0, 0.0), WHITE),
            SpriteVertex::new(vec3( half_w,  half_h, 0.0), vec2(1.0, 0.0), WHITE),
            SpriteVertex::new(vec3( half_w, -half_h, 0.0), vec2(1.0, 1.0), WHITE),
        ];

        // 3. 上传顶点 / 索引
        self.vertex_buffer.ensure_size_and_copy(
            &self.context.device,
            &self.context.queue,
            bytemuck::cast_slice(&all_vertices),
        );
        self.index_buffer.ensure_size_and_copy(
            &self.context.device,
            &self.context.queue,
            bytemuck::cast_slice(QUAD_INDICES_U32),
        );

        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Blit Encoder"),
            });

        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Mesh Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &surface_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            rp.set_pipeline(self.blit_pipeline.as_ref().unwrap());

            rp.set_vertex_buffer(0, self.vertex_buffer.buffer.slice(..));
            rp.set_index_buffer(self.index_buffer.buffer.slice(..), IndexFormat::Uint32);

            rp.set_bind_group(0, &default_rt.blit_bind_group, &[]); // 纹理+采样器
            rp.set_bind_group(1, self.camera_bind_group.as_ref(), &[]); // 相机uniform
            
            // 8. 绘制
            rp.draw_indexed(0..6, 0, 0..1);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));

        output.present();
    }

    pub(crate) fn end_frame(&mut self) {
        self.clear_buffer();
    }

    pub(crate) fn clear_buffer(&mut self) {
        let mut encoder = self
            .context
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Clear buffer encoder"),
            });

        encoder.clear_buffer(&self.vertex_buffer.buffer, 0, None);
        encoder.clear_buffer(&self.index_buffer.buffer, 0, None);
        encoder.clear_buffer(&self.camera_buffer, 0, None);

        self.context.queue.submit(std::iter::once(encoder.finish()));
    }

    fn projection_matrix(&self) -> Mat4 {
        if let Some(camera) = &get_run_time_context().read().main_camera {
            camera.lock().matrix()
        } else {
            self.pixel_perfect_projection_matrix()
        }
    }

    fn pixel_perfect_projection_matrix(&self) -> Mat4 {
        let (x, y) = (self.size.x as f32 / 2.0, self.size.y as f32 / 2.0);
        // 保持左手坐标系函数
        let view = Mat4::look_at_lh(Vec3::ZERO, Vec3::Z, Vec3::Y);
        let proj = Mat4::orthographic_lh(
            -x,
            x,
            -y,
            y,
            -1.,
            1.,
        );
        proj * view
    }
}