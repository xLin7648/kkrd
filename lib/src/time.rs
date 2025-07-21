use crate::*;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct Time {
    start_time: Instant,
    current_time: Duration,
    delta_time: Duration,
    fps: f32,  // 改为f32保持类型一致
    frame_times: [f32; 60],  // 帧时间环形缓冲区
    frame_index: usize,
    last_update: Instant,
    
    pub sleep_end: Instant,
    pub sleep_timer: SleepTimer,
}

#[derive(Default, Clone)]
pub struct SleepTimer {
    pub oversleep: Arc<Mutex<Duration>>,
    pub frametime: Arc<Mutex<Duration>>,
}

static TIME: Lazy<Arc<RwLock<Time>>> = Lazy::new(|| Arc::new(RwLock::new(Time::new())));

pub(crate) fn get_timer() -> Arc<RwLock<Time>> {
    Arc::clone(&TIME)
}

impl Time {
    fn new() -> Self {
        let start_time = Instant::now();
        Self {
            start_time,
            current_time: Duration::ZERO,
            delta_time: Duration::ZERO,
            fps: 0.0,
            frame_times: [0.0; 60],
            frame_index: 0,
            last_update: start_time,
            sleep_end: Instant::now(),
            sleep_timer: SleepTimer::default(),
        }
    }

    fn update(&mut self) {
        let now = Instant::now();
        
        // 计算增量时间
        self.delta_time = now.duration_since(self.last_update);
        self.last_update = now;
        self.current_time = now.duration_since(self.start_time);
        
        // 更新帧时间缓冲区
        let delta_secs = self.delta_time.as_secs_f32();
        self.frame_times[self.frame_index] = delta_secs;
        self.frame_index = (self.frame_index + 1) % self.frame_times.len();
        
        // 计算平均FPS（基于最近N帧）
        let total_time: f32 = self.frame_times.iter().sum();
        self.fps = if total_time > 0.0 {
            self.frame_times.len() as f32 / total_time
        } else {
            0.0
        };
    }

    // 获取当前时间 (秒)
    pub fn get_time(&self) -> f32 {
        self.current_time.as_secs_f32()
    }

    // 获取增量时间 (秒)
    pub fn get_delta_time(&self) -> f32 {
        self.delta_time.as_secs_f32()
    }

    // 获取平均FPS
    pub fn get_fps(&self) -> u32 {
        self.fps.round() as u32
    }

    pub fn print_time_data(&self) {
        println!(
            "Time: {:.3}s | FPS: {}(avg)",
            self.current_time.as_secs_f32(),
            self.fps.round() as u32,
        );
    }
}

pub fn update() {
    TIME.write().update();
}

// 获取当前时间 (秒)
pub fn get_time() -> f32 {
    TIME.read().current_time.as_secs_f32()
}

// 获取增量时间 (秒)
pub fn get_delta_time() -> f32 {
    TIME.read().delta_time.as_secs_f32()
}

// 获取平均FPS
pub fn get_fps() -> u32 {
    TIME.read().fps.round() as u32
}

pub fn print_time_data() {
    let time = TIME.read();
    let avg_frame_time = 1000.0 / time.fps.max(0.001);
    
    println!(
        "Time: {:.3}s | FPS: {}(avg)",
        time.current_time.as_secs_f32(),
        time.fps.round() as u32,
    );
}