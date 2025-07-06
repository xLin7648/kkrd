// 内部模块的导入
mod rect;
mod quad;
mod utils;
mod color;
mod y_sort;
mod assets;
mod camera;
mod config;
mod device;
mod shaders;
mod texture;
mod batching;
mod gg;
mod pipelines;
mod render_pass;
mod render_queues;
mod post_processing;
mod time;
mod gameloop;
mod ex;

use pollster::FutureExt;
// 其他可能导入的模块
use rect::*;
use quad::*;
use utils::*;
use color::*;
use colors::*;
use y_sort::*;
use assets::*;
use camera::*;
use config::*;
use device::*;
use shaders::*;
use texture::*;
use batching::*;
use gg::*;
use pipelines::*;
use render_pass::*;
use render_queues::*;
use post_processing::*;
use time::*;
use gameloop::*;
use ex::*;

// 外部依赖库的导入
use anyhow::*;
use atomic_refcell::*;
use glam::*;
use log::*;
use ordered_float::*;
use pollster::*;
use smallvec::*;
use itertools::*;
use parking_lot::*;
use once_cell::sync::*;

// std 相关的导入
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{DefaultHasher, Hasher},
    sync::{atomic::*, Arc},
};

// WGPU 相关的导入
use wgpu::{util::{self, DeviceExt}, Adapter, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, BufferUsages, Device, DeviceDescriptor, Features, Instance, InstanceDescriptor, Limits, PipelineCompilationOptions, PowerPreference, PresentMode, Queue, ShaderStages, Surface, SurfaceConfiguration};

// Winit 相关的导入
use winit::{
    application::ApplicationHandler, dpi::*, event::*, event_loop::{EventLoop, *}, window::*
};

// 线程间通信消息
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
enum RenderMessage {
    RenderFrame, // 请求渲染帧
    Exit,        // 退出信号
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

    let _event_loop_proxy = event_loop.create_proxy();

    init_window_config(
        "Full Game Loop Example".to_string(),
        "v0.0.1",
        _init_default_config,
    );

    let mut game = MyGame::default();

    std::thread::spawn(move || {
        loop {
            if window_config().init_end {
                // 执行游戏逻辑（物理、AI、状态更新等）
                game.start();
                break;
            }
        }

        loop {
            // 执行游戏逻辑（物理、AI、状态更新等）
            game.update();

            // 通知渲染线程渲染当前帧
            let _ = _event_loop_proxy.send_event(RenderMessage::RenderFrame);

            get_timer().lock().update();

            // 控制帧率（示例：60FPS）
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    let _ = event_loop.run_app(&mut App::default());
}

pub fn _init_default_config(mut config: WindowConfig) -> WindowConfig {
    config.resolution = ResolutionConfig::Physical(1280, 720);
    config.power_preference = PowerPreference::HighPerformance;
    config.vsync_mode = PresentMode::Immediate;
    config.sample_count = Msaa::Sample4;
    config
}

#[derive(Default)]
struct App
{
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
        let min_resolution = Some( match window_config.min_resolution {
            ResolutionConfig::Physical(w, h) => Size::Physical(PhysicalSize::new(w, h)),
            ResolutionConfig::Logical(w, h) => Size::Logical(LogicalSize::new(w as f64, h as f64)),
        });
    
        let fullscreen = if window_config.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };
    
        let mut window_attributes = WindowAttributes::default();
    
        window_attributes.title          = window_config.title_name.clone();
        window_attributes.inner_size     = resolution;
        window_attributes.min_inner_size = min_resolution;
        window_attributes.fullscreen     = fullscreen;
    
        self.window = Some(Arc::new(event_loop.create_window(window_attributes).unwrap()));
    }

    pub fn set_window(&mut self, window_config: WindowConfig) {
        let window = self.window.as_mut().expect("The window has not been initialized");
    
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
        if let Some(w) = &mut self.wr {
            w.update();
            w.draw();
            w.end_frame();

            clear_shader_uniform_table();
        }
    }
}

impl ApplicationHandler<RenderMessage> for App
{
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RenderMessage) {
        match event {
            RenderMessage::RenderFrame => self.renderer_update(),
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

            self.wr = Some(WgpuRenderer::new(
                self.window.clone().unwrap()
            ).block_on());

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
                    wr.resize(new_size);
                }
            },
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
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