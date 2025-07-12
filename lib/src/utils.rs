use crate::*;

use anyhow::*;
use image::DynamicImage;

pub const FRAG_SHADER_PREFIX: &str = include_str!("shaders/frag-shader-prefix.wgsl");

pub const CAMERA_BIND_GROUP_PREFIX: &str = include_str!("shaders/camera-bind-group.wgsl");

pub const SHADER_POST_PROCESSING_VERTEX: &str = include_str!("shaders/post_processing_vertex.wgsl");

pub const COPY_SHADER_SRC: &str = include_str!("shaders/copy.wgsl");

pub fn sprite_shader_from_fragment(source: &str) -> String {
    format!(
        "{}{}{}",
        CAMERA_BIND_GROUP_PREFIX, FRAG_SHADER_PREFIX, source
    )
}

pub fn post_process_shader_from_fragment(source: &str) -> String {
    format!(
        "{}{}{}",
        CAMERA_BIND_GROUP_PREFIX, SHADER_POST_PROCESSING_VERTEX, source
    )
}

pub fn create_engine_post_processing_shader(
    shaders: &mut ShaderMap,
    name: &str,
    shader_str: &str,
) -> Shader {
    let full_shader = post_process_shader_from_fragment(shader_str);

    let shader_id = create_shader(shaders, name, &full_shader, HashMap::new())
        .expect("Failed to create shader");

    shaders.get(shader_id).expect("Shader not found").clone()
}

pub fn load_texture_from_engine_bytes(
    context: &GraphicsContext,
    name: &str,
    bytes: &[u8],
    textures: &mut TextureMap,
    address_mode: wgpu::AddressMode,
) {
    let img = image::load_from_memory(bytes).expect("must be valid image");
    let texture = Texture::from_image_ex(
        &context.device,
        &context.queue,
        &img,
        Some(name),
        false,
        address_mode,
    )
    .unwrap();

    load_texture_with_image(context, name, img, texture, textures);
}

/// Loads a pre-created `Texture` with an associated `DynamicImage`
/// into the asset store.
///
/// Useful for when the user wants to create a Texture on their own,
/// e.g. by using a more exotic format and way of loading of the image.
pub fn load_texture_with_image(
    context: &GraphicsContext,
    name: &str,
    img: DynamicImage,
    texture: Texture,
    textures: &mut TextureMap,
) {
    let handle = texture_path(name);

    let bind_group = context.device.simple_bind_group(
        Some(&format!("{}_bind_group", name)),
        &texture,
        &context.texture_layout,
    );

    ASSETS.write().insert_handle(name, handle);
    ASSETS
        .write()
        .texture_image_map
        .lock()
        .insert(handle, Arc::new(img.to_rgba8()));
    textures.insert(
        handle,
        BindableTexture {
            bind_group,
            texture,
        },
    );
}

// EX
use crate::*;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum BlendMode {
    #[default]
    None,
    // TODO: Rename to Add
    Additive,
    Alpha,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextureHandle {
    Path(u64),
    Raw(u64),
    RenderTarget(RenderTargetId),
}

pub fn default_hash(value: &impl std::hash::Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

pub fn simple_hash(value: impl std::hash::Hash) -> u64 {
    ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(value)
}

impl TextureHandle {
    // TODO: rename to something like "unchecked_id"
    pub fn from_path(path: &str) -> Self {
        TextureHandle::Path(simple_hash(path))
    }

    pub fn key_unchecked(key: &str) -> Self {
        TextureHandle::Path(simple_hash(key))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Mesh {
    pub origin: Vec3,
    pub vertices: SmallVec<[SpriteVertex; 4]>,
    pub indices: SmallVec<[u32; 6]>,
    pub z_index: i32,
    pub texture: Option<TextureHandle>,
    pub y_sort_offset: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    pub fn new(position: Vec3, tex_coords: Vec2, color: Color) -> Self {
        Self {
            position: [position.x, position.y, position.z],
            tex_coords: [tex_coords.x, tex_coords.y],
            color: [color.r, color.g, color.b, color.a],
        }
    }
}

pub trait DeviceExtensions {
    fn simple_encoder(&self, label: &str) -> wgpu::CommandEncoder;
    fn simple_bind_group(
        &self,
        label: Option<&str>,
        texture: &Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup;
}

impl DeviceExtensions for wgpu::Device {
    fn simple_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    fn simple_bind_group(
        &self,
        label: Option<&str>,
        texture: &Texture,
        layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        self.create_bind_group(&wgpu::BindGroupDescriptor {
            label,
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }
}

pub struct SizedBuffer {
    pub buffer: wgpu::Buffer,
    pub size: usize,
    pub buffer_type: BufferType,
    pub label: String,
}

impl SizedBuffer {
    pub fn new(label: &str, device: &wgpu::Device, size: usize, buffer_type: BufferType) -> Self {
        let desc = wgpu::BufferDescriptor {
            label: Some(label),
            usage: buffer_type.usage(),
            size: size as wgpu::BufferAddress,
            mapped_at_creation: false,
        };

        let buffer = device.create_buffer(&desc);

        Self {
            label: label.to_string(),
            size,
            buffer_type,
            buffer,
        }
    }

    pub fn ensure_size_and_copy(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        data: &[u8],
    ) {
        if data.len() > self.size {
            self.buffer.destroy();
            self.size = data.len();
            self.buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&self.label),
                usage: self.buffer_type.usage(),
                contents: data,
            });
        } else {
            queue.write_buffer(&self.buffer, 0, data);
        }
    }
}

pub enum BufferType {
    Vertex,
    Index,
    Instance,
    Uniform,
    Storage,
    Read,
}

impl BufferType {
    pub fn usage(&self) -> wgpu::BufferUsages {
        match self {
            BufferType::Vertex => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            BufferType::Index => wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            BufferType::Instance => wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            BufferType::Uniform => wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            BufferType::Read => wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            BufferType::Storage => {
                todo!()
            }
        }
    }
}

pub fn create_render_pipeline(
    label: &str,
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    color_format: wgpu::TextureFormat,
    depth_format: Option<wgpu::TextureFormat>,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: &Shader,
    blend_mode: BlendMode,
) -> Result<wgpu::RenderPipeline> {
    // let module = naga::front::wgsl::parse_str(&shader.source)?;
    //
    // let mut validator = naga::valid::Validator::new(
    //     naga::valid::ValidationFlags::all(),
    //     naga::valid::Capabilities::all(),
    // );
    //
    // validator.validate(&module)?;

    let wgpu_shader = shader_to_wgpu(shader);

    let shader = device.create_shader_module(wgpu_shader);

    info!("CREATED SHADER, GOT {:?}", shader);

    let blend_state = match blend_mode {
        BlendMode::Alpha => Some(wgpu::BlendState::ALPHA_BLENDING),
        // BlendMode::Additive => Some(wgpu::BlendState::ALPHA_BLENDING),
        BlendMode::Additive => Some(wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::SrcAlpha,
                dst_factor: wgpu::BlendFactor::DstAlpha,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
        }),
        // BlendMode::Additive => {
        //     Some(wgpu::BlendState {
        //         color: wgpu::BlendComponent {
        //             src_factor: wgpu::BlendFactor::One,
        //             dst_factor: wgpu::BlendFactor::One,
        //             operation: wgpu::BlendOperation::Add,
        //         },
        //         // alpha: wgpu::BlendComponent::REPLACE,
        //         alpha: wgpu::BlendComponent {
        //             src_factor: wgpu::BlendFactor::One,
        //             dst_factor: wgpu::BlendFactor::One,
        //             operation: wgpu::BlendOperation::Add,
        //         },
        //     })
        // }
        BlendMode::None => Some(wgpu::BlendState::ALPHA_BLENDING),
    };

    // let blend_state = Some(wgpu::BlendState {
    //     color: wgpu::BlendComponent {
    //         src_factor: wgpu::BlendFactor::One,
    //         dst_factor: wgpu::BlendFactor::One,
    //         operation: wgpu::BlendOperation::Add,
    //     },
    //     // alpha: wgpu::BlendComponent::REPLACE,
    //     alpha: wgpu::BlendComponent {
    //         src_factor: wgpu::BlendFactor::One,
    //         dst_factor: wgpu::BlendFactor::One,
    //         operation: wgpu::BlendOperation::Add,
    //     },
    // });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: color_format,
                blend: blend_state,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),

        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: Some(wgpu::Face::Back),
            ..Default::default()
        },

        depth_stencil: depth_format.map(|format| wgpu::DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),

        multisample: wgpu::MultisampleState {
            count: game_config().sample_count.clone().into(),
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    Ok(pipeline)
}

pub fn create_render_pipeline_with_layout(
    name: &str,
    device: &wgpu::Device,
    color_format: wgpu::TextureFormat,
    bind_group_layouts: &[&wgpu::BindGroupLayout],
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: &Shader,
    blend_mode: BlendMode,
    enable_z_buffer: bool,
) -> Result<wgpu::RenderPipeline> {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{} Pipeline Layout", name)),
        bind_group_layouts,
        push_constant_ranges: &[],
    });

    create_render_pipeline(
        &format!("{} Pipeline", name),
        device,
        &layout,
        color_format,
        if enable_z_buffer {
            Some(Texture::DEPTH_FORMAT)
        } else {
            None
        },
        vertex_layouts,
        shader,
        blend_mode,
    )
}

pub trait Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a>;
}

const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
];

impl Vertex for SpriteVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

pub fn color_to_clear_op(color: Option<Color>) -> wgpu::LoadOp<wgpu::Color> {
    match color {
        Some(clear_color) => wgpu::LoadOp::Clear(clear_color.into()),
        None => wgpu::LoadOp::Load,
    }
}

pub fn create_multisampled_framebuffer(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    sample_count: u32,
) -> (wgpu::TextureView, Option<wgpu::TextureView>) {
    // 1. 优先选择移动端兼容格式
    let format = config.format;

    // 2. 创建 MSAA 颜色纹理
    let color_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Multisampled Color Attachment"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format, // 使用调整后的格式
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[format], // 关键修复：声明视图格式
    });

    // 3. 创建 MSAA 深度纹理（移动端必需）
    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Multisampled Depth Attachment"),
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float, // 或 Depth24Plus
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    // 4. 返回视图
    (
        color_texture.create_view(&wgpu::TextureViewDescriptor::default()),
        Some(depth_texture.create_view(&wgpu::TextureViewDescriptor::default())),
    )
}

pub fn create_hdr_texture(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        label: Some("HDR Texture"),
        view_formats: &[],
    });

    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

pub fn create_tonemapping_pipeline(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    hdr_bind_group_layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Tonemapping Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/tonemapping.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Tonemapping Pipeline Layout"),
        bind_group_layouts: &[hdr_bind_group_layout],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Tonemapping Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

pub fn create_hdr_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("hdr_bind_group_layout"),
    })
}

pub fn create_hdr_bind_group(
    device: &wgpu::Device,
    hdr_bind_group_layout: &wgpu::BindGroupLayout,
    hdr_texture: &wgpu::TextureView,
) -> wgpu::BindGroup {
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: hdr_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(hdr_texture),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
        label: Some("hdr_bind_group"),
    })
}

pub fn is_mobile() -> bool {
    // 实际实现根据目标平台判断
    #[cfg(target_os = "android")]
    return true;
    #[cfg(target_os = "ios")]
    return true;
    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    return false;
}
