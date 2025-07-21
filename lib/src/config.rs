use crate::*;

#[derive(Copy, Clone, Debug)]
pub enum ResolutionConfig {
    Physical(u32, u32),
    Logical(u32, u32),
}

impl ResolutionConfig {
    pub fn width(&self) -> u32 {
        match self {
            Self::Physical(w, _) => *w,
            Self::Logical(w, _) => *w,
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            Self::Physical(_, h) => *h,
            Self::Logical(_, h) => *h,
        }
    }

    pub fn ensure_non_zero(&mut self) -> ResolutionConfig {
        const MIN_WINDOW_SIZE: u32 = 1;
        match self {
            ResolutionConfig::Physical(w, h) | ResolutionConfig::Logical(w, h)
                if *w == 0 || *h == 0 =>
            {
                *w = MIN_WINDOW_SIZE;
                *h = MIN_WINDOW_SIZE;
            }
            _ => (),
        }

        *self
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum Msaa {
    Off = 1,
    Sample2 = 2,
    #[default]
    Sample4 = 4,
    Sample8 = 8,
}

// 实现 From Trait，使其返回对应的 u32 值
impl From<Msaa> for u32 {
    fn from(msaa: Msaa) -> Self {
        msaa as u32
    }
}

#[derive(Debug)]
pub struct InitGameConfig {
    pub version: &'static str,
    pub window_config: WindowConfig,
}

impl Default for InitGameConfig {
    fn default() -> Self {
        Self {
            version: "New Version",
            window_config: WindowConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title_name: String,
    pub fullscreen: bool,
    pub resolution: Size,
    pub min_resolution: Option<Size>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title_name: "New Game".to_owned(),
            fullscreen: false,
            resolution: Size::Physical(PhysicalSize::new(1280, 720)),
            min_resolution: Some(Size::Physical(PhysicalSize::new(1280, 720))),
        }
    }
}

pub struct RunTimeContext {
    pub target_frame_rate: Option<u32>,
    pub sample_count: Msaa,

    pub clear_color: Color,
    pub main_camera: Option<Arc<RwLock<dyn camera::Camera>>>,
}

impl Default for RunTimeContext {
    fn default() -> Self {
        Self { 
            target_frame_rate: Some(120),
            sample_count: Msaa::default(),
            clear_color: BLACK,
            main_camera: None,
        }
    }
}

pub static RUN_TIME_CONTEXT: OnceLock<Arc<RwLock<RunTimeContext>>> = OnceLock::new();

pub fn get_run_time_context() -> Arc<RwLock<RunTimeContext>> {
    RUN_TIME_CONTEXT
        .get()
        .unwrap()
        .clone()
}

pub fn set_clear_background_color(color: Color) {
    get_run_time_context().write().clear_color = color;
}

pub fn set_camera<T: Camera + Send + Sync + 'static>(mut camera: T) {
    camera.resize(get_window_size());
    get_run_time_context().write().main_camera = Some(Arc::new(RwLock::new(camera)));
}

pub fn get_camera() -> Option<Arc<RwLock<dyn Camera>>> {
    if let Some(cam) = &get_run_time_context().read().main_camera {
        Some(Arc::clone(&cam))
    } else {
        None
    }
}

pub fn set_default_camera() {
    get_run_time_context().write().main_camera = None;
}

pub fn set_target_frame_rate(target_frame_rate: u32) {
    get_run_time_context().write().target_frame_rate = Some(target_frame_rate);
}