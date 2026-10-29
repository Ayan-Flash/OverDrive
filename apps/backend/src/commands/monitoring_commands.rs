use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tauri::State;
use tracing::info;

use super::state::SharedState;
use crate::utils::error::AppResult;

static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Background collection cadence: 250 ms (4 Hz) keeps the ring buffers and
/// cached recent readings fresh enough for "live" charts while the per-call
/// NVML probe stays far below 1 ms. Persistence and alert rules are throttled
/// separately (see `RECORD_EVERY_TICKS`) so what reaches the database/alerts
/// is unchanged from the former 1 Hz loop.
pub const SAMPLE_INTERVAL_MS: u64 = 250;

/// One alert evaluation + DB-persisted sample per second.
pub const RECORD_EVERY_TICKS: u32 = 1000 / SAMPLE_INTERVAL_MS as u32;

#[tauri::command]
pub fn start_monitoring(state: State<'_, SharedState>) -> AppResult<()> {
    if MONITORING_ACTIVE.load(Ordering::Relaxed) {
        return Ok(()); // already running
    }
    start_background_loop(state.inner());
    Ok(())
}

/// Spawns the background collection thread. Called by `start_monitoring` and
/// automatically from app setup so telemetry, alerts, and the recorded history
/// work without the user having to enable anything first (like a classic
/// hardware monitor, which is always on duty).
pub fn start_background_loop(state: &SharedState) {
    info!("Starting background monitoring loop");
    state.monitoring.start();
    MONITORING_ACTIVE.store(true, Ordering::Relaxed);

    let app_state = state.clone();
    thread::spawn(move || {
        let mut tick = 0u32;
        while MONITORING_ACTIVE.load(Ordering::Relaxed) {
            let samples = app_state.monitoring.collect_all_once();
            tick += 1;
            if tick % RECORD_EVERY_TICKS == 0 {
                // Alert rules fire on every breaching sample (no edge trigger),
                // so they are evaluated at the fixed 1 Hz cadence regardless of
                // how fast samples are collected or the UI polls.
                for sample in &samples {
                    app_state.alerts.evaluate_sample(sample);
                }
            }
            thread::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS));
        }
        app_state.monitoring.stop();
        info!("Background monitoring loop exited");
    });
}

#[tauri::command]
pub fn stop_monitoring(_state: State<'_, SharedState>) -> AppResult<()> {
    if !MONITORING_ACTIVE.load(Ordering::Relaxed) {
        return Ok(());
    }
    info!("Stopping background monitoring loop");
    MONITORING_ACTIVE.store(false, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub fn is_monitoring_running(_state: State<'_, SharedState>) -> AppResult<bool> {
    Ok(MONITORING_ACTIVE.load(Ordering::Relaxed))
}

#[tauri::command]
pub fn export_csv(
    state: State<'_, SharedState>,
    gpu_id: String,
    count: usize,
) -> AppResult<String> {
    info!(gpu_id, count, "Exporting CSV");
    Ok(state.monitoring.export_csv(&gpu_id, count))
}
