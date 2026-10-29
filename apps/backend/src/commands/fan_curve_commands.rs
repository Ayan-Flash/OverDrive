use std::sync::Arc;

use tauri::State;
use tracing::info;

use super::state::SharedState;
use crate::hardware::fan_curve::{FanCurvePoint as RawFanCurvePoint, FanCurveState};
use crate::utils::error::AppResult;

/// Wire format for a fan curve keyframe.
#[derive(serde::Deserialize, serde::Serialize)]
pub struct FanCurvePoint {
    pub temperature_celsius: f64,
    pub fan_percent: f64,
}

impl From<RawFanCurvePoint> for FanCurvePoint {
    fn from(point: RawFanCurvePoint) -> Self {
        Self {
            temperature_celsius: point.temperature_celsius,
            fan_percent: point.fan_percent,
        }
    }
}

impl From<FanCurvePoint> for RawFanCurvePoint {
    fn from(point: FanCurvePoint) -> Self {
        Self {
            temperature_celsius: point.temperature_celsius,
            fan_percent: point.fan_percent,
        }
    }
}

/// Web-layer projection of the engine state for a GPU.
#[derive(serde::Serialize)]
pub struct FanCurveView {
    pub gpu_id: String,
    pub enabled: bool,
    pub points: Vec<FanCurvePoint>,
}

impl From<FanCurveState> for FanCurveView {
    fn from(state: FanCurveState) -> Self {
        Self {
            gpu_id: state.gpu_id,
            enabled: state.enabled,
            points: state.points.into_iter().map(Into::into).collect(),
        }
    }
}

#[tauri::command]
pub fn get_fan_curve(state: State<'_, SharedState>, gpu_id: String) -> AppResult<FanCurveView> {
    let view: FanCurveView = state.fan_curve.get_state(&gpu_id).into();
    Ok(view)
}

#[tauri::command]
pub fn set_fan_curve(
    state: State<'_, SharedState>,
    gpu_id: String,
    points: Vec<FanCurvePoint>,
) -> AppResult<()> {
    let raw: Vec<RawFanCurvePoint> = points.into_iter().map(Into::into).collect();
    state.fan_curve.set_curve(&gpu_id, raw)?;
    info!(gpu_id, "Custom fan curve updated");
    Ok(())
}

#[tauri::command]
pub fn set_fan_curve_enabled(
    state: State<'_, SharedState>,
    gpu_id: String,
    enabled: bool,
) -> AppResult<FanCurveView> {
    let monitor = &state.monitoring;
    let engine = &state.fan_curve;
    if enabled {
        engine.enable(&gpu_id);
        info!(gpu_id, "Custom fan curve engaged");
    } else {
        // Give the fans back to the driver's automatic policy before dropping
        // the manual target; a failed restore does not block going idle, but
        // the error must reach the UI instead of failing silently.
        if let Some(previous_gpu) = engine.disable() {
            if let Err(error) = monitor.set_fan_control_automatic(&previous_gpu) {
                tracing::warn!(gpu_id = %previous_gpu, error = %error, "Fan restore to automatic failed");
                return Err(error.into());
            }
        }
        info!(gpu_id, "Custom fan curve disabled");
    }
    let view: FanCurveView = engine.get_state(&gpu_id).into();
    Ok(view)
}

#[tauri::command]
pub fn reset_gpu_controls(state: State<'_, SharedState>, gpu_id: String) -> AppResult<()> {
    // Turning off any active custom curve is part of "restore defaults".
    if let Some(previous_gpu) = state.fan_curve.disable() {
        if let Err(error) = state.monitoring.set_fan_control_automatic(&previous_gpu) {
            tracing::warn!(gpu_id = %previous_gpu, error = %error, "Fan restore to automatic failed");
        }
    }
    info!(gpu_id, "Restoring GPU controls to driver defaults");
    state.monitoring.reset_controls(&gpu_id)?;
    Ok(())
}

/// Named export for the spawn helper used in `lib.rs` setup; keeping the loop
/// here makes the engine tick testable and keeps `lib.rs` declarative.
pub fn spawn_fan_curve_task(engine: Arc<crate::hardware::FanCurveEngine>, monitor: Arc<crate::monitoring::MonitoringEngine>) {
    tauri::async_runtime::spawn(async move {
        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(crate::hardware::fan_curve::APPLY_INTERVAL_SECS));
        loop {
            interval.tick().await;
            let Some(gpu_id) = engine.curve_gpu_id() else {
                continue;
            };
            let Some(temperature) = monitor.probe_temperature(&gpu_id) else {
                continue;
            };
            if let Some(duty) = engine.target_duty(&gpu_id, temperature) {
                match monitor.set_fan_speed(&gpu_id, duty) {
                    Ok(()) => {
                        engine.note_applied(&gpu_id, duty);
                    }
                    Err(error) => {
                        tracing::warn!(gpu_id = %gpu_id, error = %error, "Fan curve duty write failed");
                    }
                }
            }
        }
    });
}
