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
            ResolutionConfig::Physical(w, h) |
            ResolutionConfig::Logical(w, h)
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

#[derive(Default, Debug, Clone, PartialEq)]
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
pub struct GameConfig {
    pub title_name: String,
    pub version: &'static str,
    pub fullscreen: bool,
    pub target_frame_rate: Option<u32>,
    pub init_end: bool,

    pub resolution: Option<Size>,
    pub min_resolution: Option<Size>,
    
    pub sample_count: Msaa,
    pub power_preference: wgpu::PowerPreference,

    pub clear_color: Color,

    pub main_camera: Option<Arc<RwLock<dyn camera::Camera>>>
}

impl Default for GameConfig {
    fn default() -> Self {
        Self { 
            title_name: "New Game".to_owned(),
            version: "New Version",
            fullscreen: false,
            init_end: false,
            target_frame_rate: Some(60),

            resolution: Some(Size::Physical(PhysicalSize::new(1280, 720))), 
            min_resolution: Some(Size::Physical(PhysicalSize::new(1280, 720))), 

            sample_count: Msaa::default(),
            power_preference: wgpu::PowerPreference::default(),

            clear_color: BLUE,

            main_camera: None
        }
    }
}

static GAME_CONFIG: OnceCell<RwLock<GameConfig>> = OnceCell::new();

pub(crate) fn init_game_config(
    title_name: String,
    version: &'static str,
    config_fn: fn(GameConfig) -> GameConfig,
) {
    GAME_CONFIG
        .set(RwLock::new(config_fn(GameConfig {
            title_name,
            version,
            ..Default::default()
        })))
        .expect("init_window_config() should only be called once");
}

pub(crate) fn game_config() -> RwLockReadGuard<'static, GameConfig> {
    GAME_CONFIG
        .get()
        .expect("window_config() must be called after comfy main runs")
        .read()
}

pub(crate) fn game_config_mut() -> RwLockWriteGuard<'static, GameConfig> {
    GAME_CONFIG
        .get()
        .expect("game_config() must be called after comfy main runs")
        .write()
}

pub fn clear_background(color: Color) {
    let mut config = game_config_mut();
    config.clear_color = color;
}

pub fn set_camera<T: Camera + Send + Sync + 'static>(mut camera: T) {
    let mut config = game_config_mut();

    camera.resize(get_window_size());

    config.main_camera = Some(Arc::new(RwLock::new(camera)));
}

pub fn get_camera() -> Option<Arc<RwLock<dyn Camera>>> {
    let config = game_config();

    if let Some(cam) = &config.main_camera {
        Some(Arc::clone(&cam))
    } else {
        None
    }
}

pub fn set_default_camera() {
    let mut config = game_config_mut();
    config.main_camera = None;
}

pub fn set_target_frame_rate(target_frame_rate: u32) {
    let mut config = game_config_mut();
    config.target_frame_rate = Some(target_frame_rate);
}