// 内部模块的导入
mod assets;
mod batching;
mod camera;
mod color;
mod config;
mod device;
mod ex;
mod gameloop;
mod gg;
mod pipelines;
mod post_processing;
mod quad;
mod rect;
mod render_pass;
mod render_queues;
mod shaders;
mod texture;
mod time;
mod utils;
mod y_sort;

use pollster::FutureExt;
// 其他可能导入的模块
use assets::*;
use batching::*;
use camera::*;
use color::*;
use colors::*;
use config::*;
use device::*;
use ex::*;
use gameloop::*;
use gg::*;
use pipelines::*;
use post_processing::*;
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
use anyhow::*;
use glam::*;
use itertools::*;
use log::*;
use once_cell::sync::*;
use ordered_float::*;
use parking_lot::*;
use pollster::*;
use smallvec::*;

// std 相关的导入
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap},
    hash::{DefaultHasher, Hasher},
    sync::{Arc, atomic::*},
    time::{Duration, Instant},
};

// WGPU 相关的导入
use wgpu::{
    Adapter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType,
    BufferUsages, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits,
    PipelineCompilationOptions, PowerPreference, PresentMode, Queue, ShaderStages, Surface,
    SurfaceConfiguration,
    util::{self, DeviceExt},
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
    sync::{
        Barrier,
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    task,
};

// 线程间通信消息
#[allow(dead_code)]
#[derive(Debug)]
enum RenderMessage {
    RenderFrame(oneshot::Sender<()>), // 请求渲染帧
    Exit,                             // 退出信号
}

#[cfg(target_os = "android")]
pub static ANDROID_APP: OnceLock = OnceLock::new();

#[cfg(target_os = "android")]
fn android_main(android_app: winit::platform::android::activity::AndroidApp) {
    let _ = ANDROID_APP.set(android_app);
    main();
}

// 主函数
fn main() {
    // 创建高精度Tokio运行时
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .expect("Failed to create Tokio runtime");

    // 创建通信通道
    let (tx, mut rx) = mpsc::unbounded_channel::<RenderMessage>();

    let mut event_loop_builder = EventLoop::<RenderMessage>::with_user_event();

    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;

        env_logger::builder()
            .filter_level(LevelFilter::Off) // 默认日志级别
            .parse_default_env()
            .init();

        event_loop_builder.with_any_thread(false);
    }

    #[cfg(target_os = "android")]
    {
        use android_logger::Config;
        use winit::platform::android::EventLoopBuilderExtAndroid;

        android_logger::init_once(Config::default().with_max_level(LevelFilter::Info));

        let msg = "?error";
        event_loop_builder.with_android_app(ANDROID_APP.get().expect(msg).clone());
    }

    let event_loop = event_loop_builder
        .build()
        .expect("Failed to build event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let event_loop_proxy = event_loop.create_proxy();

    init_window_config(
        "Full Game Loop Example".to_string(),
        "v0.0.1",
        _init_default_config,
    );

    let mut game = MyGame::default();

    // 启动游戏循环任务
    rt.spawn(async move {
        println!("Game loop started");
        // 等待窗口初始化完成
        while !window_config().init_end {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        // 执行游戏初始化
        game.start().await;

        // 主游戏循环
        loop {
            // 执行游戏逻辑（物理、AI、状态更新等）
            game.update().await;

            // 创建帧完成通知通道
            let (frame_done_tx, frame_done_rx) = oneshot::channel::<()>();

            // 发送渲染请求并附带完成通知
            if let Err(e) = event_loop_proxy.send_event(RenderMessage::RenderFrame(frame_done_tx)) {
                error!("Failed to send render request: {}", e);
                break;
            }

            // 等待渲染完成通知
            if frame_done_rx.await.is_err() {
                warn!("Frame completion notification failed");
            }

            get_timer().lock().update();

            if window_config().vsync_mode == PresentMode::Immediate {
                framerate_limiter(100.);
            }

            // print_time_data();
        }
    });

    // 启动事件循环
    let _ = event_loop.run_app(&mut App::default());

    // 关闭 Tokio 运行时
    rt.shutdown_background();
}

#[allow(unused_variables)]
fn framerate_limiter(fps: f64) {
    let binding: Arc<lock_api::Mutex<RawMutex, Time>> = get_timer();
    let mut timer = binding.lock();

    let limit = Duration::from_secs_f64(1.0 / fps);
    let frame_time = timer.sleep_end.elapsed();
    let oversleep = timer
        .sleep_timer
        .oversleep
        .try_lock()
        .as_deref()
        .cloned()
        .unwrap_or_default();
    let sleep_time = limit.saturating_sub(frame_time + oversleep);
    spin_sleep::sleep(sleep_time);

    let frame_time_total = timer.sleep_end.elapsed();
    timer.sleep_end = Instant::now();

    let sd = timer.sleep_timer.frametime.try_lock();
    if let Some(mut frametime) = timer.sleep_timer.frametime.try_lock() {
        *frametime = frame_time;
    }
    if let Some(mut oversleep) = timer.sleep_timer.oversleep.try_lock() {
        *oversleep = frame_time_total.saturating_sub(limit);
    }
}

pub fn _init_default_config(mut config: WindowConfig) -> WindowConfig {
    config.resolution = ResolutionConfig::Physical(1280, 720);
    config.power_preference = PowerPreference::HighPerformance;
    config.vsync_mode = PresentMode::Fifo;
    config.sample_count = Msaa::Sample4;
    config
}

#[derive(Default)]
struct App {
    interaction_timer: Option<Instant>,
    pub window: Option<Arc<Window>>,
    pub wr: Option<WgpuRenderer>,
}

impl App {
    pub fn init_window(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_config = window_config();

        let resolution = Some(match window_config.resolution {
            ResolutionConfig::Physical(w, h) => Size::Physical(PhysicalSize::new(w, h)),
            ResolutionConfig::Logical(w, h) => Size::Logical(LogicalSize::new(w as f64, h as f64)),
        });
        let min_resolution = Some(match window_config.min_resolution {
            ResolutionConfig::Physical(w, h) => Size::Physical(PhysicalSize::new(w, h)),
            ResolutionConfig::Logical(w, h) => Size::Logical(LogicalSize::new(w as f64, h as f64)),
        });

        let fullscreen = if window_config.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };

        let mut window_attributes = WindowAttributes::default();

        window_attributes.title = window_config.title_name.clone();
        window_attributes.inner_size = resolution;
        window_attributes.min_inner_size = min_resolution;
        window_attributes.fullscreen = fullscreen;

        self.window = Some(Arc::new(
            event_loop.create_window(window_attributes).unwrap(),
        ));
    }

    pub fn set_window(&mut self, window_config: WindowConfig) {
        let window = self
            .window
            .as_mut()
            .expect("The window has not been initialized");

        let resolution = match window_config.resolution {
            ResolutionConfig::Physical(w, h) => Size::Physical(PhysicalSize::new(w, h)),
            ResolutionConfig::Logical(w, h) => Size::Logical(LogicalSize::new(w as f64, h as f64)),
        };
        let min_resolution = Some(match window_config.min_resolution {
            ResolutionConfig::Physical(w, h) => Size::Physical(PhysicalSize::new(w, h)),
            ResolutionConfig::Logical(w, h) => Size::Logical(LogicalSize::new(w as f64, h as f64)),
        });
        let _ = window.request_inner_size(resolution);

        window.set_title(&window_config.title_name);
        window.set_min_inner_size(min_resolution);

        if window_config.fullscreen {
            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
        }

        set_window_config(window_config);
    }

    pub fn renderer_update(&mut self) {
        // 交互结束500ms后恢复垂直同步
        if let Some(wr) = &mut self.wr {
            wr.update();
            wr.draw();
            wr.end_frame();

            clear_shader_uniform_table();

            if let Some(timer) = self.interaction_timer {
                if timer.elapsed() > Duration::from_millis(500) {
                    wr.set_present_mode(PresentMode::Fifo);
                    window_config_mut().vsync_mode = PresentMode::Fifo;
                    self.interaction_timer = None;
                }
            }
        }
    }
}

impl ApplicationHandler<RenderMessage> for App {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RenderMessage) {
        match event {
            RenderMessage::RenderFrame(completion_tx) => {
                self.renderer_update();

                // 通知游戏循环渲染完成
                if let Err(_) = completion_tx.send(()) {
                    warn!("Failed to send frame completion");
                }
            }
            RenderMessage::Exit => event_loop.exit(),
        }
    }

    // 当应用程序从挂起状态恢复时调用此方法
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(window) = &self.window {
            if let Some(wr) = &mut self.wr {
                wr.context.resume(window.clone());

                info!("Resumed");
            }
        } else {
            // 从桌面回来不会执行
            self.init_window(event_loop);

            self.wr = Some(WgpuRenderer::new(self.window.clone().unwrap()).block_on());

            window_config_mut().init_end = true;
        }
    }

    // 处理窗口相关的事件
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                if let Some(wr) = &mut self.wr {
                    // 交互时切换为Immediate模式
                    wr.resize(new_size, PresentMode::Immediate);
                    window_config_mut().vsync_mode = PresentMode::Immediate;
                    self.interaction_timer = Some(Instant::now());
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(wr) = &mut self.wr {
                    // 交互时切换为Immediate模式
                    wr.set_present_mode(PresentMode::Immediate);
                    window_config_mut().vsync_mode = PresentMode::Immediate;
                    self.interaction_timer = Some(Instant::now());
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => (),
        }
    }

    // region: 看起来没什么用的内容

    // 当应用程序被挂起时调用
    fn suspended(&mut self, _: &ActiveEventLoop) {
        if let Some(wr) = self.wr.as_mut() {
            wr.context.surface = None;
        }

        info!("Suspended");
    }

    // 在应用程序准备退出时调用
    fn exiting(&mut self, _: &ActiveEventLoop) {
        info!("Exiting");
    }

    // endregion: 看起来没什么用的内容
}
