use crate::*;

pub async fn create_graphics_context(window: Arc<Window>) -> GraphicsContext {
    let size = window.inner_size();
    let window_config = game_config();

    let instance = Instance::new(&InstanceDescriptor {
        backends: Backends::VULKAN,
        ..Default::default()
    });

    let surface = instance
        .create_surface(window)
        .expect("Failed to create surface");

    trace!("Requesting adapter");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: window_config.power_preference,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("adapter config must be valid");

    info!("Using adapter: {:?}", adapter.get_info().name);

    trace!("Requesting device");

    let limits = wgpu::Limits {
        max_texture_dimension_2d: 4096,
        ..wgpu::Limits::downlevel_defaults()
    };

    // TODO: adapter.features();

    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                required_limits: limits,

                ..Default::default()
            },
            None,
        )
        .await
        .expect("failed to create wgpu adapter");

    let caps = surface.get_capabilities(&adapter);

    error!("Supported formats: {:?}", caps.formats);

    // 1. 动态选择支持的格式
    let format = if caps.formats.contains(&wgpu::TextureFormat::Rgba8Unorm) {
        wgpu::TextureFormat::Rgba8Unorm
    } else if caps.formats.contains(&wgpu::TextureFormat::Bgra8Unorm) {
        wgpu::TextureFormat::Bgra8Unorm
    } else if caps.formats.contains(&wgpu::TextureFormat::Rgba16Float) {
        // 使用高动态范围格式
        wgpu::TextureFormat::Rgba16Float
    } else if let Some(&fallback) = caps.formats.first() {
        // 使用设备支持的第一个格式
        log::warn!("Using fallback format: {:?}", fallback);
        fallback
    } else {
        // 没有支持的格式 - 严重错误
        panic!("No supported surface formats available!");
    };

    error!("Supported format: {:?}", format);

    let _ = DEFAULT_TEXTURE_FORMAT.set(format);

    let surface_usage = wgpu::TextureUsages::RENDER_ATTACHMENT;

    let config = wgpu::SurfaceConfiguration {
        usage: surface_usage,
        format: format,
        width: size.width.max(1),
        height: size.height.max(1),
        present_mode: PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        desired_maximum_frame_latency: 2,
        view_formats: vec![],
    };

    trace!("Configuring surface");

    surface.configure(&device, &config);

    let texture_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
