use crate::*;
use pollster::FutureExt;

#[derive(Default)]
struct App {
    pub runtime: Option<Box<Runtime>>,
    pub window: Option<Arc<Window>>,
    pub wr: Option<WgpuRenderer>,
}

pub fn start_game(mut game: impl GameLoop + 'static) {
    // 创建高精度Tokio运行时
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .expect("Failed to create Tokio runtime");

    // 创建通信通道
    let (tx, mut rx) = mpsc::unbounded_channel::<WinitMessage>();

    let mut event_loop_builder = EventLoop::<WinitMessage>::with_user_event();

    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;

        env_logger::builder()
            .filter_level(LevelFilter::Info) // 默认日志级别
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

    // 启动游戏循环任务
    rt.spawn(async move {
        // 等待窗口初始化完成
        while !game_config().init_end {
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
            if let Err(e) = event_loop_proxy.send_event(WinitMessage::RenderFrame(frame_done_tx)) {
                error!("Failed to send render request: {}", e);
                let _ = event_loop_proxy.send_event(WinitMessage::Exit);
                break;
            }

            // 等待渲染完成通知
            if frame_done_rx.await.is_err() {
                warn!("Frame completion notification failed");
            }

            // 更新timer
            get_timer().lock().update();

            framerate_limiter();
        }
    });

    let mut app = App {
        runtime: Some(Box::new(rt)),
        ..Default::default()
    };

    // 启动事件循环
    let _ = event_loop.run_app(&mut app);
}

impl App {
    pub fn init_window(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_config = game_config();

        let fullscreen = if window_config.fullscreen {
            Some(Fullscreen::Borderless(None))
        } else {
            None
        };

        let mut window_attributes = WindowAttributes::default();
        window_attributes.title = window_config.title_name.clone();
        window_attributes.inner_size = window_config.resolution;
        window_attributes.min_inner_size = window_config.min_resolution;
        window_attributes.fullscreen = fullscreen;

        match event_loop.create_window(window_attributes) {
            Ok(window) => {
                self.window = Some(Arc::new(window));
                let _ = GLOBAL_WINDOW.set(self.window.clone().unwrap());
            }
            Err(_) => event_loop.exit(),
        }
    }

    pub fn renderer_update(&mut self) {
        if let Some(wr) = &mut self.wr {
            wr.update();
            wr.draw();
            wr.end_frame();

            clear_shader_uniform_table();
        }
    }
}

impl ApplicationHandler<WinitMessage> for App {
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: WinitMessage) {
        match event {
            WinitMessage::RenderFrame(completion_tx) => {
                self.renderer_update();

                // 通知游戏循环渲染完成
                if let Err(_) = completion_tx.send(()) {
                    warn!("Failed to send frame completion");
                }
            }
            WinitMessage::Exit => event_loop.exit(),
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

            game_config_mut().init_end = true;
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
        // 关闭 Tokio 运行时
        if let Some(runtime) = self.runtime.take() {
            // 用 take() 获取所有权
            runtime.shutdown_background();
        }
        info!("Exiting");
    }

    // endregion: 看起来没什么用的内容
}
