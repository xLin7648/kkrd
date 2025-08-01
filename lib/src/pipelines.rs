use parking_lot::lock_api::Mutex;
use wgpu::{
    AddressMode, BindingResource, BufferDescriptor, Extent3d, FilterMode, Sampler,
    SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use crate::*;

static GENERATED_RENDER_TARGET_IDS: AtomicU32 = AtomicU32::new(0);

#[derive(Clone, Debug)]
pub struct RenderTargetParams {
    pub label: String,
    pub size: UVec2,
}

#[derive(Debug)]
pub(crate) struct UserRenderTarget {
    // RT 尺寸
    pub size: UVec2,

    // MSAA 专用
    pub msaa_texture: wgpu::Texture,
    pub msaa_view: wgpu::TextureView,
    pub msaa_depth_texture: wgpu::Texture,
    pub msaa_depth_view: wgpu::TextureView,

    // 真正拿来采样 / blit 的纹理
    pub resolve_texture: wgpu::Texture,
    pub resolve_view: wgpu::TextureView,

    // 采样 resolve_texture 用的 bind_group
    pub blit_bind_group: wgpu::BindGroup,
}

impl UserRenderTarget {
    pub fn new(params: &RenderTargetParams) -> RenderTargetId {
        let id = gen_render_target();

        let wr = get_global_wgpu().read();

        // 5. 插入全局表
        get_global_render_targets().write().insert(
            id,
            Arc::new(RwLock::new(Self::create_resources(
                &wr.context,
                &wr.texture_layout,
                params,
            ))),
        );

        id
    }

    pub fn update(
        &mut self,
        c: &GraphicsContext,
        texture_layout: &BindGroupLayout,
        params: &RenderTargetParams,
    ) {
        let resources = Self::create_resources(c, texture_layout, params);

        self.size = resources.size;

        self.msaa_texture = resources.msaa_texture;
        self.msaa_view = resources.msaa_view;
        self.msaa_depth_texture = resources.msaa_depth_texture;
        self.msaa_depth_view = resources.msaa_depth_view;

        self.resolve_texture = resources.resolve_texture;
        self.resolve_view = resources.resolve_view;

        self.blit_bind_group = resources.blit_bind_group;
    }

    fn create_resources(
        c: &GraphicsContext,
        texture_layout: &BindGroupLayout,
        params: &RenderTargetParams,
    ) -> UserRenderTarget {
        let size = Extent3d {
            width: params.size.x,
            height: params.size.y,
            depth_or_array_layers: 1,
        };

        // 由外部决定 1 或 4/8
        let sample_count = get_run_time_context().read().sample_count.into();
        let format = *DEFAULT_TEXTURE_FORMAT.get().unwrap();
        let label = params.label.as_str();

        // 1) MSAA 颜色纹理
        let msaa_texture = c.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}_msaa_color")),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[format],
        });
        let msaa_view = msaa_texture.create_view(&Default::default());

        // 2) MSAA 深度纹理
        let msaa_depth_texture = c.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}_msaa_depth")),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let msaa_depth_view = msaa_depth_texture.create_view(&Default::default());

        // 3) 1-sample resolve 纹理（真正拿来采样 / blit）
        let resolve_texture = c.device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("{label}_resolve")),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[format],
        });
        let resolve_view = resolve_texture.create_view(&Default::default());

        // 4) 采样器 + blit bind_group
        let sampler = c.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(&format!("{label}_sampler")),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });
        let blit_bind_group = c.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(&format!("{label}_blit_bind_group")),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&resolve_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        UserRenderTarget {
            size: params.size,
            msaa_texture,
            msaa_view,
            msaa_depth_texture,
            msaa_depth_view,
            resolve_texture,
            resolve_view,
            blit_bind_group,
        }
    }
}

/// Allocates a new render target id
fn gen_render_target() -> RenderTargetId {
    let id = GENERATED_RENDER_TARGET_IDS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    RenderTargetId(id)
}

pub fn ensure_pipeline_exists(
    context: &mut WgpuRenderer,
    pass_data: &MeshDrawData,
    sprite_shader_id: ShaderId,
    sample_count: u32,
) -> String {
    let shaders = context.shaders.lock();

    let maybe_shader_instance_id = pass_data.shader;

    let maybe_shader = {
        if maybe_shader_instance_id.0 > 0 {
            let instance = get_shader_instance(maybe_shader_instance_id);
            shaders.get(instance.id)
        } else {
            None
        }
    };

    let name = format!(
        "{} {:?} {:?} {:?}",
        if maybe_shader_instance_id.0 > 0 {
            "USER(Mesh)"
        } else {
            "BUILTIN(Mesh)"
        },
        pass_data.blend_mode,
        maybe_shader,
        context.enable_z_buffer
    );

    let mesh_pipeline = if let Some(shader) = maybe_shader {
        RenderPipeline::User(
            context
                .user_pipelines
                .entry(name.clone())
                .or_insert_with(|| {
                    create_user_pipeline(
                        &name,
                        pass_data,
                        shader,
                        &context.context,
                        &context.texture_layout,
                        &context.camera_bind_group_layout,
                        context.enable_z_buffer,
                        sample_count,
                    )
                }),
        )
    } else {
        RenderPipeline::Wgpu(context.pipelines.entry(name.clone()).or_insert_with(|| {
            create_render_pipeline_with_layout(
                &name,
                &context.context.device,
                *DEFAULT_TEXTURE_FORMAT.get().unwrap(),
                &[&context.texture_layout, &context.camera_bind_group_layout],
                &[SpriteVertex::desc()],
                shaders.get(sprite_shader_id).unwrap(),
                pass_data.blend_mode,
                context.enable_z_buffer,
                sample_count,
            )
            .unwrap()
        }))
    };

    if let RenderPipeline::User(user_pipeline) = mesh_pipeline {
        if maybe_shader_instance_id.0 > 0 {
            let shader_instance = get_shader_instance(maybe_shader_instance_id);
            let shader = shaders.get(shader_instance.id).unwrap();

            for (buffer_name, buffer) in user_pipeline.buffers.iter().sorted_by_key(|x| x.0) {
                // 尝试从实例中获取自定义uniform值
                if let Some(uniform) = shader_instance.uniforms.get(buffer_name) {
                    match uniform {
                        Uniform::F32(value) => {
                            let data = [value.0];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        Uniform::Vec2(values) => {
                            let data = [values[0].0, values[1].0];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        Uniform::Vec3(values) => {
                            let data = [values[0].0, values[1].0, values[2].0];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        Uniform::Vec4(values) => {
                            let data = [values[0].0, values[1].0, values[2].0, values[3].0];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                    }
                }
                // 使用shader定义的默认值
                else if let Some(uniform_def) = shader.uniform_defs.get(buffer_name) {
                    match uniform_def {
                        UniformDef::F32(Some(default_value)) => {
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&[*default_value]),
                            );
                        }
                        UniformDef::Vec2(Some((x, y))) => {
                            let data = [*x, *y];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        UniformDef::Vec3(Some((x, y, z))) => {
                            let data = [*x, *y, *z];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        UniformDef::Vec4(Some((x, y, z, w))) => {
                            let data = [*x, *y, *z, *w];
                            context.context.queue.write_buffer(
                                buffer,
                                0,
                                bytemuck::cast_slice(&data),
                            );
                        }
                        // 没有默认值的情况
                        _ => panic!("No uniform value or default for {buffer_name}"),
                    }
                } else {
                    panic!("Uniform definition not found for {buffer_name}");
                }
            }
        }
    }

    name
}

pub fn create_user_pipeline(
    name: &str,
    pass_data: &MeshDrawData,
    shader: &Shader,
    context: &GraphicsContext,
    texture_layout: &Arc<BindGroupLayout>,
    camera_bind_group_layout: &BindGroupLayout,
    enable_z_buffer: bool,
    sample_count: u32,
) -> UserRenderPipeline {
    info!("Creating pipeline for shader: {:?}", shader.id);

    let mut layout_entries = Vec::new();
    let mut bind_group_entries = Vec::new();
    let mut buffers = HashMap::new();

    for (uniform_name, binding) in shader.bindings.iter() {
        let uniform_def = shader.uniform_defs.get(uniform_name).unwrap();

        layout_entries.push(BindGroupLayoutEntry {
            binding: *binding,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        });

        let uniform_buffer_usage = BufferUsages::UNIFORM | BufferUsages::COPY_DST;

        match uniform_def {
            UniformDef::F32(maybe_default) => {
                let size = std::mem::size_of::<f32>() as u64;
                if let Some(value) = maybe_default {
                    let buffer = context
                        .device
                        .create_buffer_init(&util::BufferInitDescriptor {
                            label: Some(&format!("User UB: {} (default={})", uniform_name, value)),
                            contents: bytemuck::cast_slice(&[*value]),
                            usage: uniform_buffer_usage,
                        });
                    buffers.insert(uniform_name.to_string(), buffer);
                } else {
                    let buffer = context.device.create_buffer(&BufferDescriptor {
                        label: Some(&format!("User UB: {} (no-default)", uniform_name)),
                        size,
                        usage: uniform_buffer_usage,
                        mapped_at_creation: false,
                    });
                    buffers.insert(uniform_name.to_string(), buffer);
                }
            }
            UniformDef::Vec2(maybe_default) => {
                let size = std::mem::size_of::<[f32; 2]>() as u64;
                if let Some((x, y)) = maybe_default {
                    let data = [*x, *y];
                    let buffer = context
                        .device
                        .create_buffer_init(&util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}))",
                                uniform_name, x, y
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        });
                    buffers.insert(uniform_name.to_string(), buffer);
                } else {
                    let buffer = context.device.create_buffer(&BufferDescriptor {
                        label: Some(&format!("User UB: {} (no-default)", uniform_name)),
                        size,
                        usage: uniform_buffer_usage,
                        mapped_at_creation: false,
                    });
                    buffers.insert(uniform_name.to_string(), buffer);
                }
            }
            UniformDef::Vec3(maybe_default) => {
                let size = std::mem::size_of::<[f32; 3]>() as u64;
                if let Some((x, y, z)) = maybe_default {
                    let data = [*x, *y, *z];
                    let buffer = context
                        .device
                        .create_buffer_init(&util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}, {}))",
                                uniform_name, x, y, z
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        });
                    buffers.insert(uniform_name.to_string(), buffer);
                } else {
                    let buffer = context.device.create_buffer(&BufferDescriptor {
                        label: Some(&format!("User UB: {} (no-default)", uniform_name)),
                        size,
                        usage: uniform_buffer_usage,
                        mapped_at_creation: false,
                    });
                    buffers.insert(uniform_name.to_string(), buffer);
                }
            }
            UniformDef::Vec4(maybe_default) => {
                let size = std::mem::size_of::<[f32; 4]>() as u64;
                if let Some((x, y, z, w)) = maybe_default {
                    let data = [*x, *y, *z, *w];
                    let buffer = context
                        .device
                        .create_buffer_init(&util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}, {}, {}))",
                                uniform_name, x, y, z, w
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        });
                    buffers.insert(uniform_name.to_string(), buffer);
                } else {
                    let buffer = context.device.create_buffer(&BufferDescriptor {
                        label: Some(&format!("User UB: {} (no-default)", uniform_name)),
                        size,
                        usage: uniform_buffer_usage,
                        mapped_at_creation: false,
                    });
                    buffers.insert(uniform_name.to_string(), buffer);
                }
            }
        }
    }

    for (name, binding) in shader.bindings.iter() {
        bind_group_entries.push(BindGroupEntry {
            binding: *binding,
            resource: buffers.get(name).unwrap().as_entire_binding(),
        });
    }

    let user_layout = context
        .device
        .create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(&format!("User Layout: {}", name)),
            entries: &layout_entries,
        });

    let pipeline = create_render_pipeline_with_layout(
        name,
        &context.device,
        *DEFAULT_TEXTURE_FORMAT.get().unwrap(),
        &[&texture_layout, &camera_bind_group_layout, &user_layout],
        &[SpriteVertex::desc()],
        shader,
        pass_data.blend_mode,
        enable_z_buffer,
        sample_count,
    )
    .unwrap();

    let bind_group = context.device.create_bind_group(&BindGroupDescriptor {
        label: Some("User Bind Group"),
        layout: &user_layout,
        entries: &bind_group_entries,
    });

    UserRenderPipeline {
        pipeline,
        layout: user_layout,
        bind_group,
        buffers,
    }
}
