use crate::*;

#[cfg(not(target_arch = "wasm32"))]
pub fn detect_frametime() -> Duration {
    let refresh_rate = get_global_window()
        .and_then(|w| w.current_monitor())
        .and_then(|m| m.refresh_rate_millihertz())
        .unwrap_or(30_000) as f64 / 1000.0;
    
    let best_framerate = refresh_rate - 0.5;
    Duration::from_secs_f64(1.0 / best_framerate)
}

#[allow(unused_variables)]
pub fn framerate_limiter() {

    let limit = if let Some(frame_rate) = game_config().target_frame_rate {
        Duration::from_secs_f64(1.0 / frame_rate as f64)
    } else {
        detect_frametime()
    };
    
    let binding = get_timer();
    let mut timer = binding.write();

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