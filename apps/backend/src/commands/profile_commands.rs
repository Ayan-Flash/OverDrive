use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::info;

use super::state::SharedState;
use crate::hardware::fan_curve::FanCurvePoint as RawFanCurvePoint;
use crate::utils::error::{AppError, AppResult};

fn profiles_dir() -> AppResult<std::path::PathBuf> {
    let dir = dirs_next::config_dir()
        .ok_or_else(|| AppError::ConfigError("Failed to get config directory".into()))?
        .join("overdrive")
        .join("profiles");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn slots_dir() -> AppResult<std::path::PathBuf> {
    let dir = profiles_dir()?.join("slots");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

const QUICK_SLOT_COUNT: i32 = 5;

/// Fan mode carried inside a control snapshot. Mirrors the Tuning page's
/// three modes (driver automatic, fixed duty, custom temperature curve).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotFanMode {
    Automatic,
    Fixed,
    Curve,
}

/// Fan-curve keyframe as stored inside a snapshot. Same wire shape as the
/// fan curve commands use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotFanPoint {
    pub temperature_celsius: f64,
    pub fan_percent: f64,
}

/// Full portable control state: everything the Tuning page can set, captured
/// or applied in one unit (the MSI Afterburner profile model).
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ControlSnapshot {
    #[serde(default)]
    pub core_offset_mhz: i32,
    #[serde(default)]
    pub memory_offset_mhz: i32,
    #[serde(default)]
    pub voltage_offset_mv: i32,
    /// Power limit in percent of the driver default; absent when unsupported.
    #[serde(default)]
    pub power_limit_percent: Option<f64>,
    #[serde(default)]
    pub fan_mode: Option<SnapshotFanMode>,
    /// Fan duty target used when `fan_mode` is `fixed`.
    #[serde(default)]
    pub fixed_fan_percent: Option<f64>,
    /// Temperature → duty keyframes used when `fan_mode` is `curve`.
    #[serde(default)]
    pub fan_curve: Option<Vec<SnapshotFanPoint>>,
}

/// Summary of one occupied quick slot for the Tuning page strip.
#[derive(Debug, Clone, Serialize)]
pub struct QuickSlotSummary {
    pub slot: i32,
    pub name: String,
    pub gpu_id: String,
    pub saved_at: String,
    pub snapshot: ControlSnapshot,
}

fn slot_path(slot: i32) -> AppResult<std::path::PathBuf> {
    if slot < 1 || slot > QUICK_SLOT_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "Slot must be between 1 and {QUICK_SLOT_COUNT}"
        )));
    }
    Ok(slots_dir()?.join(format!("slot{slot}.json")))
}

#[derive(Serialize, Deserialize)]
struct QuickSlotFile {
    name: String,
    gpu_id: String,
    saved_at: String,
    snapshot: ControlSnapshot,
}

/// Apply a full control snapshot to one GPU. Fan handling follows the
/// captured mode; unsupported controls degrade to a logged warning instead
/// of failing the whole apply (voltage offsetting is not exposed by NVML).
fn apply_control_snapshot_inner(state: &SharedState, gpu_id: &str, snapshot: &ControlSnapshot) -> AppResult<()> {
    let engine = &state.fan_curve;
    let monitor = &state.monitoring;

    // Fan mode first so later fan writes are done with the engine's target
    // mode active (fixed writes happen after the engine is off).
    match snapshot.fan_mode {
        None => {}
        Some(SnapshotFanMode::Automatic) => {
            if let Some(previous_gpu) = engine.disable() {
                if let Err(error) = monitor.set_fan_control_automatic(&previous_gpu) {
                    tracing::warn!(
                        gpu_id = %previous_gpu,
                        error = %error,
                        "Fan restore to automatic failed while applying snapshot"
                    );
                }
            }
        }
        Some(SnapshotFanMode::Fixed) => {
            if let Some(previous_gpu) = engine.disable() {
                if let Err(error) = monitor.set_fan_control_automatic(&previous_gpu) {
                    tracing::warn!(
                        gpu_id = %previous_gpu,
                        error = %error,
                        "Fan restore to automatic failed while applying snapshot"
                    );
                }
            }
            if let Some(percent) = snapshot.fixed_fan_percent {
                monitor.set_fan_speed(gpu_id, percent)?;
            }
        }
        Some(SnapshotFanMode::Curve) => {
            if let Some(curve) = &snapshot.fan_curve {
                let raw: Vec<RawFanCurvePoint> = curve
                    .iter()
                    .map(|point| RawFanCurvePoint {
                        temperature_celsius: point.temperature_celsius,
                        fan_percent: point.fan_percent,
                    })
                    .collect();
                engine.set_curve(gpu_id, raw)?;
                engine.enable(gpu_id);
                info!(gpu_id, "Snapshot fan curve engaged");
            } else {
                // Curve mode without keyframes falls back to the driver policy.
                if let Some(previous_gpu) = engine.disable() {
                    if let Err(error) = monitor.set_fan_control_automatic(&previous_gpu) {
                        tracing::warn!(
                            gpu_id = %previous_gpu,
                            error = %error,
                            "Fan restore to automatic failed while applying snapshot"
                        );
                    }
                }
            }
        }
    }

    monitor.set_core_clock_offset(gpu_id, snapshot.core_offset_mhz)?;
    monitor.set_memory_clock_offset(gpu_id, snapshot.memory_offset_mhz)?;
    if snapshot.voltage_offset_mv != 0 {
        // NVML exposes no voltage control; degrade instead of failing.
        if let Err(error) = monitor.set_voltage_offset(gpu_id, snapshot.voltage_offset_mv) {
            tracing::warn!(
                gpu_id,
                error = %error,
                "Voltage offset not applied (driver does not expose it)"
            );
        }
    }
    if let Some(percent) = snapshot.power_limit_percent {
        monitor.set_power_limit(gpu_id, percent)?;
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplySnapshotArgs {
    gpu_id: String,
    snapshot: ControlSnapshot,
}

#[tauri::command]
pub fn apply_control_snapshot(state: State<'_, SharedState>, gpu_id: String, snapshot: ControlSnapshot) -> AppResult<()> {
    info!(gpu_id, "Applying control snapshot");
    apply_control_snapshot_inner(&state, &gpu_id, &snapshot)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QuickSaveArgs {
    gpu_id: String,
    slot: i32,
    #[serde(default)]
    name: Option<String>,
    snapshot: ControlSnapshot,
}

#[tauri::command]
pub fn quick_save_slot(
    state: State<'_, SharedState>,
    gpu_id: String,
    slot: i32,
    name: Option<String>,
    snapshot: ControlSnapshot,
) -> AppResult<()> {
    let _ = &state;
    let file = QuickSlotFile {
        name: name.unwrap_or_else(|| format!("Profile {slot}")),
        gpu_id: gpu_id.clone(),
        saved_at: chrono::Utc::now().to_rfc3339(),
        snapshot,
    };
    let path = slot_path(slot)?;
    std::fs::write(&path, serde_json::to_string_pretty(&file)?)?;
    info!(gpu_id, slot, "Saved control snapshot to quick slot");
    Ok(())
}

#[tauri::command]
pub fn quick_delete_slot(slot: i32) -> AppResult<()> {
    let path = slot_path(slot)?;
    if path.exists() {
        std::fs::remove_file(&path)?;
        info!(slot, "Quick slot cleared");
    }
    Ok(())
}

#[tauri::command]
pub fn get_quick_slots(_state: State<'_, SharedState>) -> AppResult<Vec<Option<QuickSlotSummary>>> {
    let dir = slots_dir()?;
    let mut slots: Vec<Option<QuickSlotSummary>> = Vec::new();
    for slot in 1..=QUICK_SLOT_COUNT {
        let path = dir.join(format!("slot{slot}.json"));
        let summary = if path.exists() {
            let text = std::fs::read_to_string(&path)?;
            match serde_json::from_str::<QuickSlotFile>(&text) {
                Ok(file) => Some(QuickSlotSummary {
                    slot,
                    name: file.name,
                    gpu_id: file.gpu_id,
                    saved_at: file.saved_at,
                    snapshot: file.snapshot,
                }),
                Err(error) => {
                    tracing::warn!(slot, error = %error, "Quick slot file unreadable");
                    None
                }
            }
        } else {
            None
        };
        slots.push(summary);
    }
    Ok(slots)
}

#[tauri::command]
pub fn quick_load_slot(state: State<'_, SharedState>, gpu_id: String, slot: i32) -> AppResult<()> {
    let path = slot_path(slot)?;
    if !path.exists() {
        return Err(AppError::InvalidArgument("Slot is empty".into()));
    }
    let file: QuickSlotFile = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    info!(gpu_id, slot, "Loading quick slot");
    apply_control_snapshot_inner(&state, &gpu_id, &file.snapshot)
}

#[tauri::command]
pub fn save_profile(
    _state: State<'_, SharedState>,
    name: String,
    gpu_id: String,
    core_offset: i32,
    mem_offset: i32,
    voltage_offset: i32,
    fan_speed: f64,
    power_limit: f64,
) -> AppResult<String> {
    if name.trim().is_empty() {
        return Err(AppError::InvalidArgument("Profile name cannot be empty".into()));
    }
    if name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err(AppError::InvalidProfileName(
            "Profile name contains invalid characters".into(),
        ));
    }

    info!(name, gpu_id, "Saving profile");
    let profile = serde_json::json!({
        "name": name,
        "gpu_id": gpu_id,
        "core_offset": core_offset,
        "mem_offset": mem_offset,
        "voltage_offset": voltage_offset,
        "fan_speed": fan_speed,
        "power_limit": power_limit,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let dir = profiles_dir()?;
    let filename = format!("{}_{}.json", name.replace(' ', "_"), gpu_id);
    let path = dir.join(&filename);
    let content = serde_json::to_string_pretty(&profile)?;
    std::fs::write(&path, content)?;

    Ok(filename)
}

#[tauri::command]
pub fn load_profiles(_state: State<'_, SharedState>) -> AppResult<Vec<serde_json::Value>> {
    let dir = profiles_dir()?;

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut profiles = Vec::new();
    let entries = std::fs::read_dir(&dir)?;
    for entry in entries {
        let entry = entry?;
        let content = std::fs::read_to_string(entry.path())?;
        if let Ok(profile) = serde_json::from_str(&content) {
            profiles.push(profile);
        }
    }

    Ok(profiles)
}

#[tauri::command]
pub fn delete_profile(_state: State<'_, SharedState>, filename: String) -> AppResult<()> {
    info!(filename, "Deleting profile");
    let dir = profiles_dir()?;
    let path = dir.join(&filename);
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

#[tauri::command]
pub fn apply_profile(
    state: State<'_, SharedState>,
    gpu_id: String,
    core_offset: i32,
    mem_offset: i32,
    voltage_offset: i32,
    fan_speed: f64,
    power_limit: f64,
) -> AppResult<()> {
    info!(gpu_id, "Applying profile");
    state.monitoring.set_core_clock_offset(&gpu_id, core_offset)?;
    state.monitoring.set_memory_clock_offset(&gpu_id, mem_offset)?;
    state.monitoring.set_voltage_offset(&gpu_id, voltage_offset)?;
    state.monitoring.set_fan_speed(&gpu_id, fan_speed)?;
    state.monitoring.set_power_limit(&gpu_id, power_limit)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trips_through_json() {
        let snapshot = ControlSnapshot {
            core_offset_mhz: 150,
            memory_offset_mhz: -500,
            voltage_offset_mv: 0,
            power_limit_percent: Some(110.0),
            fan_mode: Some(SnapshotFanMode::Curve),
            fixed_fan_percent: None,
            fan_curve: Some(vec![
                SnapshotFanPoint { temperature_celsius: 40.0, fan_percent: 30.0 },
                SnapshotFanPoint { temperature_celsius: 80.0, fan_percent: 100.0 },
            ]),
        };
        let value = serde_json::to_value(&snapshot).unwrap();
        assert_eq!(value["core_offset_mhz"], 150);
        assert_eq!(value["fan_mode"], "curve");
        let back: ControlSnapshot = serde_json::from_value(value).unwrap();
        assert_eq!(back, snapshot);
    }

    #[test]
    fn slot_path_rejects_out_of_range() {
        assert!(slot_path(0).is_err());
        assert!(slot_path(6).is_err());
        assert!(slot_path(5).is_ok());
    }
}
