use image::DynamicImage;
use image::GenericImageView;
use image::ImageResult;
use wgpu::AddressMode;
use wgpu::BindGroup;
use wgpu::BindGroupLayout;
use wgpu::CompareFunction;
use wgpu::Device;
use wgpu::Extent3d;
use wgpu::FilterMode;
use wgpu::ImageCopyTexture;
use wgpu::ImageDataLayout;
use wgpu::Origin3d;
use wgpu::Queue;
use wgpu::Sampler;
use wgpu::SamplerDescriptor;
use wgpu::SurfaceConfiguration;
use wgpu::TextureAspect;
use wgpu::TextureDescriptor;
use wgpu::TextureDimension;
use wgpu::TextureFormat;
use wgpu::TextureUsages;
use wgpu::TextureView;
use wgpu::TextureViewDescriptor;

use crate::DEFAULT_TEXTURE_FORMAT;
use crate::utils::DeviceExtensions;

#[derive(Debug)]
pub struct TextureCreationParams<'a> {
    pub label: Option<&'a str>,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
    pub mip_level_count: u32,
    pub filter_mode: FilterMode,
    pub render_scale: f32,
    pub view_formats: &'a [TextureFormat],
}

impl Default for TextureCreationParams<'_> {
    fn default() -> Self {
        Self {
            label: None,
            width: 0,
            height: 0,
            format: *(DEFAULT_TEXTURE_FORMAT.get().unwrap()),
            mip_level_count: 1,
            filter_mode: FilterMode::Linear,
            render_scale: 1.0,
            view_formats: &[],
        }
    }
}

#[derive(Debug)]
pub struct BindableTexture {
    pub texture: Texture,
    pub bind_group: BindGroup,
}

impl BindableTexture {
    pub fn new(
        device: &Device,
        layout: &BindGroupLayout,
        params: &TextureCreationParams,
    ) -> Self {
        let texture = Texture::create_with_params(device, params);

        let label = params.label.map(|x| format!("{} Bind Group", x));

        let bind_group = device.simple_bind_group(label.as_deref(), &texture, layout);

        Self {
            texture,
            bind_group,
        }
    }
}

#[derive(Debug)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: TextureView,
    pub sampler: Sampler,
}

impl Texture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &Device,
        config: &SurfaceConfiguration,
        label: &str,
        sample_count: u32
    ) -> Self {
        let size = Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count,
            dimension: TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
        }
    }

    pub fn create_with_params(device: &Device, params: &TextureCreationParams) -> Self {
        let size = Extent3d {
            width: ((params.width as f32) * params.render_scale.sqrt()).round() as u32,
            height: ((params.height as f32) * params.render_scale.sqrt()).round() as u32,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: params.label,
            size,
            mip_level_count: params.mip_level_count,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: params.format,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: params.view_formats,
        });

        let view_label = params.label.map(|x| format!("{} View", x));

        let view = texture.create_view(&TextureViewDescriptor {
            label: view_label.as_deref(),
            // TODO: fix this and move it to the pp layer instead
            mip_level_count: if params.mip_level_count > 0 {
                Some(1)
            } else {
                None
            },
            ..Default::default()
        });

        let sampler_label = params.label.map(|x| format!("{} Sampler", x));

        let sampler = device.create_sampler(&SamplerDescriptor {
            label: sampler_label.as_deref(),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: params.filter_mode,
            min_filter: params.filter_mode,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            texture,
            view,
            sampler,
            // size,
        }
    }

    pub fn create_scaled_mip_filter_surface_texture(
        device: &Device,
        config: &SurfaceConfiguration,
        format: TextureFormat,
        render_scale: f32,
        mip_level_count: u32,
        filter_mode: FilterMode,
        label: &str,
    ) -> Self {
        Self::create_with_params(
            device,
            &TextureCreationParams {
                label: Some(label),
                width: config.width,
                height: config.height,
                format,
                mip_level_count,
                filter_mode,
                render_scale,
                view_formats: &[],
            },
        )
    }

    pub fn from_bytes(
        device: &Device,
        queue: &Queue,
        bytes: &[u8],
        label: &str,
        is_normal_map: bool,
    ) -> ImageResult<(DynamicImage, Self)> {
        let img = image::load_from_memory(bytes)?;
        let tex = Self::from_image(device, queue, &img, Some(label), is_normal_map)?;

        Ok((img, tex))
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
    ) -> ImageResult<Self> {
        Self::from_image_ex(
            device,
            queue,
            img,
            label,
            is_normal_map,
            AddressMode::Repeat,
        )
    }

    pub fn from_image_ex(
        device: &Device,
        queue: &Queue,
        img: &DynamicImage,
        label: Option<&str>,
        is_normal_map: bool,
        address_mode: AddressMode,
    ) -> ImageResult<Self> {
        let format: TextureFormat = if is_normal_map {
            // 法线贴图避免伽马矫正
            TextureFormat::Rgba8Unorm
        } else {
            TextureFormat::Rgba8UnormSrgb
        };
        Self::from_image_with_format(device, queue, img, label, address_mode, format)
    }

    pub fn from_image_with_format(
        device: &Device,
        queue: &Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
        address_mode: AddressMode,
        format: TextureFormat,
    ) -> ImageResult<Self> {
        let img = img.flipv();
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        Self::from_image_data_with_format(
            device,
            queue,
            &rgba,
            label,
            address_mode,
            format,
            dimensions,
            4,
        )
    }

    pub fn from_image_data_with_format(
        device: &Device,
        queue: &Queue,
        img_data: &[u8],
        label: Option<&str>,
        address_mode: AddressMode,
        format: TextureFormat,
        dimensions: (u32, u32),
        bytes_per_pixel: u32,
    ) -> ImageResult<Self> {
        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                aspect: TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            img_data,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_pixel * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            // TODO: configure this
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub fn from_image_uninit(
        device: &Device,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> ImageResult<Self> {
        let dimensions = img.dimensions();
        assert!(dimensions.0 > 0 && dimensions.1 > 0);
        Self::create_uninit(device, dimensions.0, dimensions.1, label)
    }

    pub fn create_uninit(
        device: &Device,
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> ImageResult<Self> {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: *DEFAULT_TEXTURE_FORMAT.get().unwrap(),
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }
}
