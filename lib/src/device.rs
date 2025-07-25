use wgpu::{
    Features, Limits, RequestAdapterOptions, SamplerBindingType, TextureSampleType, TextureUsages,
    TextureViewDimension,
};

use crate::*;

pub async fn create_graphics_context(window: Arc<Window>) -> GraphicsContext {
    let size = window.inner_size();

    let default_backends = Backends::VULKAN;

    let instance = Instance::new(&InstanceDescriptor {
        backends: default_backends,
        ..Default::default()
    });

    let surface = instance
        .create_surface(window)
        .expect("Failed to create surface");

    trace!("Requesting adapter");

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("adapter config must be valid");

    info!("Using adapter: {:?}", adapter.get_info().name);

    trace!("Requesting device");

    let limits = Limits {
        max_texture_dimension_2d: 4096,
        ..Limits::downlevel_defaults()
    };

    // TODO: adapter.features();

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: limits,

                ..Default::default()
            },
            None,
        )
        .await
        .expect("failed to create wgpu adapter");

    let caps = surface.get_capabilities(&adapter);

    info!("Supported formats: {:?}", caps.formats);

    let formats = caps.formats;
    // For future HDR output support, we'll need to request a format that supports HDR,
    // but as of wgpu 0.15 that is not yet supported.
    // Prefer sRGB formats for surfaces, but fall back to first available format if no sRGB formats are available.
    let mut format = *formats.first().expect("No supported formats for surface");
    for available_format in formats {
        // Rgba8UnormSrgb and Bgra8UnormSrgb and the only sRGB formats wgpu exposes that we can use for surfaces.
        if available_format == TextureFormat::Rgba8UnormSrgb
            || available_format == TextureFormat::Bgra8UnormSrgb
        {
            format = available_format;
            break;
        }
    }

    info!("Supported format: {:?}", format);

    let _ = DEFAULT_TEXTURE_FORMAT.set(format);

    #[cfg(any(target_os = "android", target_os = "ios"))]
    let present_mode = PresentMode::Mailbox;

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let present_mode = PresentMode::Immediate;

    let config = SurfaceConfiguration {
        format,
        present_mode,
        width: size.width.max(1),
        height: size.height.max(1),
        alpha_mode: caps.alpha_modes[0],
        desired_maximum_frame_latency: 2,
        usage: TextureUsages::RENDER_ATTACHMENT,
        view_formats: if !format.is_srgb() {
            vec![format.add_srgb_suffix()]
        } else {
            vec![]
        },
    };

    trace!("Configuring surface");

    surface.configure(&device, &config);

    let texture_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    multisampled: false,
                    view_dimension: TextureViewDimension::D2,
                    sample_type: TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
        ],
        label: Some("texture_bind_group_layout"),
    });

    let textures = Arc::new(Mutex::new(HashMap::new()));

    let device = Arc::new(device);
    let queue = Arc::new(queue);
    let texture_layout = Arc::new(texture_bind_group_layout);

    GraphicsContext {
        queue,
        device,
        texture_layout,
        adapter: Arc::new(adapter),
        surface: Some(Arc::new(surface)),
        instance: Arc::new(instance),
        config: Arc::new(RwLock::new(config)),
        textures,
    }
}
