use tauri::State;
use tracing::debug;

use super::state::SharedState;
use crate::hardware::cpu::{CpuInfo, CpuSample};
use crate::utils::error::AppResult;

/// Static CPU identity: model, vendor, physical & logical core counts.
#[tauri::command]
pub fn get_cpu_info(state: State<'_, SharedState>) -> AppResult<CpuInfo> {
    let info = state.cpu.info();
    debug!("get_cpu_info: model={}", info.model);
    Ok(info)
}

/// Live CPU telemetry: package/per-core frequency, usage, temperature, voltage.
#[tauri::command]
pub fn get_cpu_sample(state: State<'_, SharedState>) -> AppResult<CpuSample> {
    let sample = state.cpu.sample();
    debug!(
        "get_cpu_sample: freq={}MHz, usage={:.1}%, temp={:?}",
        sample.frequency_mhz, sample.usage_percent, sample.temperature_celsius
    );
    Ok(sample)
}
