use crate::*;

use anyhow::*;
use image::DynamicImage;
use wgpu::{vertex_attr_array, AddressMode, BindingResource, BlendComponent, BlendFactor, BlendOperation, BlendState, BufferAddress, BufferDescriptor, ColorTargetState, ColorWrites, CommandEncoder, CommandEncoderDescriptor, CompareFunction, DepthBiasState, DepthStencilState, Extent3d, Face, FragmentState, FrontFace, LoadOp, MultisampleState, PipelineLayout, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, RenderPipelineDescriptor, StencilState, TextureDescriptor, TextureDimension, TextureUsages, TextureView, TextureViewDescriptor, VertexAttribute, VertexBufferLayout, VertexState, VertexStepMode};
use TextureFormat;

pub const VERTEX_SHADER: &str = include_str!("shaders/vertex-shader.wgsl");

pub fn sprite_shader_from_fragment(source: &str) -> String {
    format!(
        "{}{}",
        VERTEX_SHADER, source
    )
}

pub fn load_texture_from_engine_bytes(
    context: &GraphicsContext,
    name: &str,
    bytes: &[u8],
    textures: &mut TextureMap,
    address_mode: AddressMode,
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
    fn simple_encoder(&self, label: &str) -> CommandEncoder;
    fn simple_bind_group(
        &self,
        label: Option<&str>,
        texture: &Texture,
        layout: &BindGroupLayout,
    ) -> BindGroup;
}

impl DeviceExtensions for Device {
    fn simple_encoder(&self, label: &str) -> CommandEncoder {
        self.create_command_encoder(&CommandEncoderDescriptor { label: Some(label) })
    }

    fn simple_bind_group(
        &self,
        label: Option<&str>,
        texture: &Texture,
        layout: &BindGroupLayout,
    ) -> BindGroup {
        self.create_bind_group(&BindGroupDescriptor {
            label,
            layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&texture.view),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&texture.sampler),
                },
            ],
        })
    }
}

pub struct SizedBuffer {
    pub buffer: Buffer,
    pub size: usize,
    pub buffer_type: BufferType,
    pub label: String,
}

impl SizedBuffer {
    pub fn new(label: &str, device: &Device, size: usize, buffer_type: BufferType) -> Self {
        let desc = BufferDescriptor {
            label: Some(label),
            usage: buffer_type.usage(),
            size: size as BufferAddress,
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
        device: &Device,
        queue: &Queue,
        data: &[u8],
    ) {
        if data.len() > self.size {
            self.buffer.destroy();
            self.size = data.len();
            self.buffer = device.create_buffer_init(&util::BufferInitDescriptor {
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
    pub fn usage(&self) -> BufferUsages {
        match self {
            BufferType::Vertex => BufferUsages::VERTEX | BufferUsages::COPY_DST,
            BufferType::Index => BufferUsages::INDEX | BufferUsages::COPY_DST,
            BufferType::Instance => BufferUsages::VERTEX | BufferUsages::COPY_DST,
            BufferType::Uniform => BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            BufferType::Read => BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            BufferType::Storage => {
                todo!()
            }
        }
    }
}

pub fn create_render_pipeline(
    label: &str,
    device: &Device,
    layout: &PipelineLayout,
    color_format: TextureFormat,
    depth_format: Option<TextureFormat>,
    vertex_layouts: &[VertexBufferLayout],
    shader: &Shader,
    blend_mode: BlendMode,
    sample_count: u32
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
        BlendMode::Alpha => Some(BlendState::ALPHA_BLENDING),
        // BlendMode::Additive => Some(BlendState::ALPHA_BLENDING),
        BlendMode::Additive => Some(BlendState {
            color: BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::DstAlpha,
                operation: BlendOperation::Add,
            },
            alpha: BlendComponent {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        }),
        // BlendMode::Additive => {
        //     Some(BlendState {
        //         color: BlendComponent {
        //             src_factor: BlendFactor::One,
        //             dst_factor: BlendFactor::One,
        //             operation: BlendOperation::Add,
        //         },
        //         // alpha: BlendComponent::REPLACE,
        //         alpha: BlendComponent {
        //             src_factor: BlendFactor::One,
        //             dst_factor: BlendFactor::One,
        //             operation: BlendOperation::Add,
        //         },
        //     })
        // }
        BlendMode::None => Some(BlendState::ALPHA_BLENDING),
    };

    // let blend_state = Some(BlendState {
    //     color: BlendComponent {
    //         src_factor: BlendFactor::One,
    //         dst_factor: BlendFactor::One,
    //         operation: BlendOperation::Add,
    //     },
    //     // alpha: BlendComponent::REPLACE,
    //     alpha: BlendComponent {
    //         src_factor: BlendFactor::One,
    //         dst_factor: BlendFactor::One,
    //         operation: BlendOperation::Add,
    //     },
    // });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: vertex_layouts,
            compilation_options: PipelineCompilationOptions::default(),
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(ColorTargetState {
                format: color_format,
                blend: blend_state,
                write_mask: ColorWrites::ALL,
            })],
            compilation_options: PipelineCompilationOptions::default(),
        }),

        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            front_face: FrontFace::Cw, // 顺时针三角形为正面
            cull_mode: Some(Face::Back),
            ..Default::default()
        },

        depth_stencil: depth_format.map(|format| DepthStencilState {
            format,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        }),

        multisample: MultisampleState {
            count: sample_count,
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
    device: &Device,
    color_format: TextureFormat,
    bind_group_layouts: &[&BindGroupLayout],
    vertex_layouts: &[VertexBufferLayout],
    shader: &Shader,
    blend_mode: BlendMode,
    enable_z_buffer: bool,
    sample_count: u32
) -> Result<wgpu::RenderPipeline> {
    let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
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
        sample_count
    )
}

pub trait Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a>;
}

const ATTRIBS: [VertexAttribute; 3] = vertex_attr_array![
    0 => Float32x3,
    1 => Float32x2,
    2 => Float32x4,
];

impl Vertex for SpriteVertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: size_of::<SpriteVertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

pub fn color_to_clear_op(color: Option<Color>) -> LoadOp<wgpu::Color> {
    match color {
        Some(clear_color) => LoadOp::Clear(clear_color.into()),
        None => LoadOp::Load,
    }
}

pub fn create_multisampled_framebuffer(
    device: &Device,
    config: &SurfaceConfiguration,
    sample_count: u32,
) -> (TextureView, Option<TextureView>) {
    // 1. 优先选择移动端兼容格式
    let format = config.format;

    // 2. 创建 MSAA 颜色纹理
    let color_texture = device.create_texture(&TextureDescriptor {
        label: Some("Multisampled Color Attachment"),
        size: Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format, // 使用调整后的格式
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[format], // 关键修复：声明视图格式
    });

    // 3. 创建 MSAA 深度纹理（移动端必需）
    let depth_texture = device.create_texture(&TextureDescriptor {
        label: Some("Multisampled Depth Attachment"),
        size: Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format: TextureFormat::Depth32Float, // 或 Depth24Plus
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    // 4. 返回视图
    (
        color_texture.create_view(&TextureViewDescriptor::default()),
        Some(depth_texture.create_view(&TextureViewDescriptor::default())),
    )
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
