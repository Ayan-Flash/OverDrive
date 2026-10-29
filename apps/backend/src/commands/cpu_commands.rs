use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tauri::State;
use tracing::debug;

use super::state::SharedState;
use crate::hardware::cpu::{CpuInfo, CpuMonitor, CpuSample};
use crate::utils::error::AppResult;

/// Fixed-rate background sampler feeding `CpuMonitor::record_latest`.
///
/// CPU usage percentages are computed over the interval between live reads,
/// so they are measured on this fixed ~1 Hz cadence — independent of how
/// fast the UI polls `get_cpu_sample` (which serves the cached result).
pub fn spawn_cpu_sampler(cpu: Arc<CpuMonitor>) {
    let sensor_monitor = Arc::downgrade(&cpu);
    thread::spawn(move || while let Some(cpu) = sensor_monitor.upgrade() {
        cpu.refresh_sensors();
        drop(cpu);
        thread::sleep(Duration::from_secs(1));
    });
    let sample_monitor = Arc::downgrade(&cpu);
    thread::spawn(move || while let Some(cpu) = sample_monitor.upgrade() {
        let started = std::time::Instant::now();
        cpu.record_latest();
        drop(cpu);
        thread::sleep(Duration::from_secs(1).saturating_sub(started.elapsed()));
    });
}

/// Static CPU identity: model, vendor, physical & logical core counts.
#[tauri::command]
pub fn get_cpu_info(state: State<'_, SharedState>) -> AppResult<CpuInfo> {
    let info = state.cpu.info();
    debug!("get_cpu_info: model={}", info.model);
    Ok(info)
}

/// Live CPU telemetry: package/per-core frequency, usage, temperature, voltage.
///
/// Served from the ~1 Hz background sampler cache, so display fetches cost
/// microseconds instead of performing sensor traffic per call. Usage
/// percentages keep their ~1 s measurement window regardless of how fast the
/// UI polls.
#[tauri::command]
pub fn get_cpu_sample(state: State<'_, SharedState>) -> AppResult<CpuSample> {
    let sample = state.cpu.sample_cached();
    debug!(
        "get_cpu_sample: freq={}MHz, usage={:.1}%, temp={:?}",
        sample.frequency_mhz, sample.usage_percent, sample.temperature_celsius
    );
    Ok(sample)
}
