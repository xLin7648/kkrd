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

#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub title_name: String,
    pub version: &'static str,
    pub fullscreen: bool,
    pub init_end: bool,

    pub resolution: ResolutionConfig,
    pub min_resolution: ResolutionConfig,
    
    pub sample_count: Msaa,
    pub vsync_mode: wgpu::PresentMode,
    pub power_preference: wgpu::PowerPreference,

    pub clear_color: Color
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self { 
            title_name: "New Game".to_owned(),
            version: "New Version",
            fullscreen: false,
            init_end: false,

            resolution: ResolutionConfig::Physical(1280, 720), 
            min_resolution: ResolutionConfig::Physical(100, 100), 

            sample_count: Msaa::default(),
            vsync_mode: wgpu::PresentMode::default(),
            power_preference: wgpu::PowerPreference::default(),

            clear_color: BLUE
        }
    }
}

use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use once_cell::sync::OnceCell;

static WINDOW_CONFIG: OnceCell<RwLock<WindowConfig>> = OnceCell::new();

pub fn init_window_config(
    title_name: String,
    version: &'static str,
    config_fn: fn(WindowConfig) -> WindowConfig,
) {
    WINDOW_CONFIG
        .set(RwLock::new(config_fn(WindowConfig {
            title_name,
            version,
            ..Default::default()
        })))
        .expect("init_window_config() should only be called once");
}

pub fn window_config() -> RwLockReadGuard<'static, WindowConfig> {
    WINDOW_CONFIG
        .get()
        .expect("window_config() must be called after comfy main runs")
        .read()
        .expect("Failed to acquire read lock")
}

pub fn window_config_mut() -> RwLockWriteGuard<'static, WindowConfig> {
    WINDOW_CONFIG
        .get()
        .expect("game_config() must be called after comfy main runs")
        .write()
        .expect("Failed to acquire write lock")
}

pub(crate) fn set_window_config(window_config: WindowConfig) {
    let mut config = window_config_mut();
    *config = window_config;
}

pub fn clear_background(color: Color) {
    let mut config = window_config_mut();
    config.clear_color = color;
}
