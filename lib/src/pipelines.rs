use wgpu::{AddressMode, BindingResource, BufferDescriptor, Extent3d, FilterMode, Sampler, SamplerDescriptor, TextureAspect, TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor};

use crate::*;

static GENERATED_RENDER_TARGET_IDS: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug)]
pub struct RenderTargetParams {
    pub label: String,
    pub size: UVec2,
    pub filter_mode: FilterMode,
}

/// Creates a new render target with given dimensions. Among other parameters a label is
/// required so that graphic debuggers like RenderDoc can display its name properly.
pub fn create_render_target(
    params: &RenderTargetParams,
) -> RenderTargetId {
    let id = gen_render_target();

    if let Some(wr) = get_global_wgpu() {
        let wr = wr.lock();
        let c = &wr.context;

        let size = Extent3d {
            width: params.size.x,
            height: params.size.y,
            depth_or_array_layers: 1,
        };

        let format = *DEFAULT_TEXTURE_FORMAT.get().unwrap();

        let texture = c.device.create_texture(&TextureDescriptor {
            label: Some(&params.label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[format],
        });

        let view = texture.create_view(&TextureViewDescriptor {
            label: Some(&format!("{} View", params.label)),
            format: None,
            dimension: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            ..Default::default()
        });

        let sampler = c.device.create_sampler(&SamplerDescriptor {
            label: Some(&format!("{} Sampler", params.label)),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: params.filter_mode,
            min_filter: params.filter_mode,
            mipmap_filter: params.filter_mode,
            ..Default::default()
        });

        let bind_group = c.device.create_bind_group(&BindGroupDescriptor {
            label: Some(&format!("{} Bind Group", params.label)),
            layout: &c.texture_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&sampler),
                },
            ],
        });

        wr.render_targets.lock().insert(
            id,
            UserRenderTarget {
                creation_params: params.clone(),
                texture,
                view,
                sampler,
                bind_group,
            },
        );

        id
    } else {
        panic!("Wgpu Renderer Not Init");
    }

    
}

pub struct UserRenderTarget {
    pub creation_params: RenderTargetParams,
    pub texture: wgpu::Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub bind_group: BindGroup,
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
    sample_count: u32
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
        "{} {:?} {:?} {:?} {:?}",
        if maybe_shader_instance_id.0 > 0 {
            "USER(Mesh)"
        } else {
            "BUILTIN(Mesh)"
        },
        pass_data.blend_mode,
        maybe_shader,
        context.enable_z_buffer,
        sample_count
    );

    let mesh_pipeline = if let Some(shader) = maybe_shader {
        RenderPipeline::User(context.user_pipelines.entry(name.clone()).or_insert_with(|| {
            create_user_pipeline(
                &name,
                pass_data,
                shader,
                &context.context,
                &context.texture_layout,
                &context.camera_bind_group_layout,
                context.enable_z_buffer,
                sample_count
            )
        }))
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
                sample_count
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
    sample_count: u32
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
                    let buffer = context.device.create_buffer_init(
                        &util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default={})",
                                uniform_name, value
                            )),
                            contents: bytemuck::cast_slice(&[*value]),
                            usage: uniform_buffer_usage,
                        }
                    );
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
                    let buffer = context.device.create_buffer_init(
                        &util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}))",
                                uniform_name, x, y
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        }
                    );
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
                    let buffer = context.device.create_buffer_init(
                        &util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}, {}))",
                                uniform_name, x, y, z
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        }
                    );
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
                    let buffer = context.device.create_buffer_init(
                        &util::BufferInitDescriptor {
                            label: Some(&format!(
                                "User UB: {} (default=({}, {}, {}, {}))",
                                uniform_name, x, y, z, w
                            )),
                            contents: bytemuck::cast_slice(&data),
                            usage: uniform_buffer_usage,
                        }
                    );
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
        sample_count
    )
    .unwrap();

    let bind_group = context
        .device
        .create_bind_group(&BindGroupDescriptor {
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
