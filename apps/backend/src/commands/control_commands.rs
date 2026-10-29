use tauri::State;
use tracing::info;

use super::state::SharedState;
use crate::utils::error::AppResult;

#[tauri::command]
pub fn set_fan_speed(
    state: State<'_, SharedState>,
    gpu_id: String,
    percent: f64,
) -> AppResult<()> {
    info!(gpu_id, percent, "Setting fan speed");
    Ok(state.monitoring.set_fan_speed(&gpu_id, percent)?)
}

#[tauri::command]
pub fn set_core_clock_offset(
    state: State<'_, SharedState>,
    gpu_id: String,
    offset_mhz: i32,
) -> AppResult<()> {
    info!(gpu_id, offset_mhz, "Setting core clock offset");
    Ok(state.monitoring.set_core_clock_offset(&gpu_id, offset_mhz)?)
}

#[tauri::command]
pub fn set_memory_clock_offset(
    state: State<'_, SharedState>,
    gpu_id: String,
    offset_mhz: i32,
) -> AppResult<()> {
    info!(gpu_id, offset_mhz, "Setting memory clock offset");
    Ok(state.monitoring.set_memory_clock_offset(&gpu_id, offset_mhz)?)
}

#[tauri::command]
pub fn set_power_limit(
    state: State<'_, SharedState>,
    gpu_id: String,
    percent: f64,
) -> AppResult<()> {
    info!(gpu_id, percent, "Setting power limit");
    Ok(state.monitoring.set_power_limit(&gpu_id, percent)?)
}

#[tauri::command]
pub fn set_voltage_offset(
    state: State<'_, SharedState>,
    gpu_id: String,
    offset_mv: i32,
) -> AppResult<()> {
    info!(gpu_id, offset_mv, "Setting voltage offset");
    Ok(state.monitoring.set_voltage_offset(&gpu_id, offset_mv)?)
}

#[tauri::command]
pub fn set_fan_control_automatic(state: State<'_, SharedState>, gpu_id: String) -> AppResult<()> {
    info!(gpu_id, "Returning fan control to the driver's automatic policy");
    Ok(state.monitoring.set_fan_control_automatic(&gpu_id)?)
}
