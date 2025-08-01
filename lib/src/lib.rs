// 内部模块的导入
mod app_events;
mod assets;
mod batching;
mod camera;
mod color;
mod config;
mod device;
mod fpslimiter;
mod gameloop;
mod graphic;
mod pipelines;
mod quad;
mod rect;
mod render_pass;
mod render_queues;
mod shaders;
mod texture;
mod time;
mod utils;
mod y_sort;

// 其他可能导入的模块
use app_events::*;
use assets::*;
use batching::*;
use camera::*;
use color::*;
use colors::*;
use config::*;
use device::*;
use fpslimiter::*;
use gameloop::*;
use graphic::*;
use pipelines::*;
use quad::*;
use rect::*;
use render_pass::*;
use render_queues::*;
use shaders::*;
use texture::*;
use time::*;
use utils::*;
use y_sort::*;

// 外部依赖库的导入
use glam::*;
use itertools::*;
use log::*;
use once_cell::sync::*;
use ordered_float::*;
use parking_lot::*;
use smallvec::*;

use async_trait::*;

// std 相关的导入
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hasher},
    sync::{Arc, OnceLock, atomic::*},
    time::{Duration, Instant},
};

// WGPU 相关的导入
use wgpu::{
    Adapter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, Device, DeviceDescriptor, Instance, InstanceDescriptor,
    PipelineCompilationOptions, PowerPreference, PresentMode, Queue, ShaderStages, Surface,
    SurfaceConfiguration,
    util::{self, DeviceExt},
    TextureFormat
};

// Winit 相关的导入
use winit::{
    application::ApplicationHandler,
    dpi::*,
    event::*,
    event_loop::{EventLoop, *},
    window::*,
};

use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
};

// 线程间通信消息
#[allow(dead_code)]
#[derive(Debug)]
enum WinitMessage {
    CheckInit(oneshot::Sender<bool>),
    RenderFrame(oneshot::Sender<()>),             // 请求渲染帧
    CheckFrameState(oneshot::Sender<FrameState>), // 检查帧状态
    ResetFrameState(oneshot::Sender<()>),         // 重置帧状态
    Exit,                                         // 退出信号
}

#[allow(dead_code)]
#[derive(Debug, Default, Clone, Copy)]
struct FrameState {
    resize: UVec2
}

// 新增全局窗口引用
static GLOBAL_WINDOW: OnceLock<Arc<Window>> = OnceLock::new();

// 新增全局窗口访问函数
pub fn get_global_window() -> Option<&'static Arc<Window>> {
    GLOBAL_WINDOW.get()
}

pub fn get_window_size() -> PhysicalSize<u32> {
    if let Some(window) = get_global_window() {
        window.inner_size()
    } else {
        PhysicalSize::new(1, 1)
    }
}

static WGPU_RENDERER: OnceLock<Arc<RwLock<WgpuRenderer>>> = OnceLock::new();

pub fn check_wgpu_init() -> bool {
    WGPU_RENDERER.get().is_some()
}

pub fn get_global_wgpu() -> &'static Arc<RwLock<WgpuRenderer>> {
    WGPU_RENDERER.get().unwrap_or_else(|| panic!("Wgpu Render Not Init"))
}

pub static DEFAULT_TEXTURE_FORMAT: OnceLock<TextureFormat> = OnceLock::new();

pub fn clear_background(color: Color) {
    let wr = get_global_wgpu();
    wr.write().clear(color);
}

static RENDER_TARGETS: OnceLock<Arc<RwLock<RenderTargetMap>>> = OnceLock::new();

pub fn get_global_render_targets() -> &'static Arc<RwLock<RenderTargetMap>> {
    RENDER_TARGETS.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

#[cfg(target_os = "android")]
pub static ANDROID_APP: OnceLock<winit::platform::android::activity::AndroidApp> = OnceLock::new();

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(android_app);
    main();
}

// 主函数
fn main() {
    let init_game_config = InitGameConfig {
        version: "v0.0.1",
        window_config: WindowConfig {
            title_name: "New Game!!!".to_owned(),
            fullscreen: false,
            resolution: Size::Physical(PhysicalSize::new(1280, 720)),
            min_resolution: None,
        },
    };

    let run_time_context = RunTimeContext {
        target_frame_rate: Some(120),
        sample_count: Msaa::Sample4,
        clear_color: BLACK,
        main_camera: None,
    };

    init_game(init_game_config, run_time_context, MyGame::default());
}