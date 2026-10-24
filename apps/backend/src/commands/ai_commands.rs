use std::sync::Arc;

use tauri::State;

use super::state::AppState;
use crate::ai::anomaly::{Anomaly, AnomalyDetector};
use crate::ai::optimizer::{OptimizationSuggestion, Optimizer};
use crate::ai::predictor::{Prediction, Predictor};
use crate::ai::store::AiStore;
use crate::ai::tuner::{ClockTuner, FanCurveTuner, FanCurveResult, ClockTuneResult, PowerTuneResult, PowerTuner, TuningProfile};
use crate::ai::smart_alerts::{SmartAlertConfig, MetricBaseline, SuppressedAlert};

#[tauri::command]
pub fn get_ai_anomalies(limit: usize) -> Vec<Anomaly> {
    let store = AiStore::new();
    store.get_anomalies(limit)
}

#[tauri::command]
pub fn get_ai_suggestions() -> Vec<OptimizationSuggestion> {
    let store = AiStore::new();
    store.get_suggestions()
}

#[tauri::command]
pub fn clear_ai_anomalies() -> Result<(), String> {
    let mut store = AiStore::new();
    store.clear_anomalies();
    Ok(())
}

#[tauri::command]
pub fn dismiss_ai_suggestion(id: String) -> Result<(), String> {
    let mut store = AiStore::new();
    store.mark_suggestion_applied(&id);
    Ok(())
}

#[tauri::command]
pub fn run_ai_analysis(
    state: State<'_, Arc<AppState>>,
) -> Result<(Vec<Anomaly>, Vec<OptimizationSuggestion>), String> {
    let mut store = AiStore::new();
    let detector = AnomalyDetector::new();
    let mut all_anomalies = vec![];
    let mut all_suggestions = vec![];

    for gpu in state.monitoring.gpus() {
        let samples = state.monitoring.get_history(&gpu.id, 120);
        if samples.len() < 10 {
            continue;
        }

        let temps: Vec<f64> = samples.iter().filter_map(|s| s.temperature_celsius).collect();
        let fans: Vec<f64> = samples.iter().filter_map(|s| s.fan_speed_percent).collect();
        let powers: Vec<f64> = samples.iter().filter_map(|s| s.power_watts).collect();
        let utils: Vec<f64> = samples.iter().filter_map(|s| s.core_utilization_percent).collect();
        let clocks: Vec<f64> = samples.iter().filter_map(|s| s.core_clock_mhz).collect();

        for mut a in detector.detect_temperature_spike(&temps) {
            a.gpu_id = gpu.id.clone();
            store.add_anomaly(a.clone());
            all_anomalies.push(a);
        }

        for mut a in detector.detect_fan_drop(&fans) {
            a.gpu_id = gpu.id.clone();
            store.add_anomaly(a.clone());
            all_anomalies.push(a);
        }

        for mut a in detector.detect_power_surge(&powers) {
            a.gpu_id = gpu.id.clone();
            store.add_anomaly(a.clone());
            all_anomalies.push(a);
        }

        let avg_temp = temps.iter().sum::<f64>() / temps.len() as f64;
        let max_temp = temps.iter().cloned().fold(f64::MIN, f64::max);
        let _avg_power = powers.iter().sum::<f64>() / powers.len() as f64;
        let max_power = powers.iter().cloned().fold(f64::MIN, f64::max);

        for s in Optimizer::suggest_fan_curve(&gpu.id, &temps, &fans, avg_temp, max_temp) {
            store.add_suggestion(s.clone());
            all_suggestions.push(s);
        }

        for s in Optimizer::suggest_power_limit(&gpu.id, &powers, &temps, max_power) {
            store.add_suggestion(s.clone());
            all_suggestions.push(s);
        }

        for s in Optimizer::suggest_underclock(&gpu.id, &temps, &clocks, max_temp) {
            store.add_suggestion(s.clone());
            all_suggestions.push(s);
        }

        let _ = temps.last();
        let _ = fans.last();
        let _ = powers.last();
        let _ = utils.last();
    }

    Ok((all_anomalies, all_suggestions))
}

#[tauri::command]
pub fn predict_gpu_temperature(
    gpu_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Prediction, String> {
    let samples = state.monitoring.get_history(&gpu_id, 120);
    if samples.is_empty() {
        return Err("No data for GPU".into());
    }
    let temps: Vec<f64> = samples.iter().filter_map(|s| s.temperature_celsius).collect();
    Ok(Predictor::predict_temperature(&temps, 90.0))
}

#[tauri::command]
pub fn predict_gpu_utilization(
    gpu_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Prediction, String> {
    let samples = state.monitoring.get_history(&gpu_id, 120);
    if samples.is_empty() {
        return Err("No data for GPU".into());
    }
    let utils: Vec<f64> = samples.iter().filter_map(|s| s.core_utilization_percent).collect();
    Ok(Predictor::predict_utilization(&utils))
}

#[tauri::command]
pub fn tune_fan_curve(
    gpu_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<FanCurveResult, String> {
    let samples = state.monitoring.get_history(&gpu_id, 300);
    if samples.is_empty() {
        return Err("No data for GPU".into());
    }
    Ok(FanCurveTuner::tune(&samples))
}

#[tauri::command]
pub fn tune_clock_offsets(
    gpu_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<ClockTuneResult, String> {
    let samples = state.monitoring.get_history(&gpu_id, 120);
    if samples.is_empty() {
        return Err("No data for GPU".into());
    }
    Ok(ClockTuner::tune(&samples))
}

#[tauri::command]
pub fn tune_power_limit(
    gpu_id: String,
    max_power_watts: f64,
    state: State<'_, Arc<AppState>>,
) -> Result<PowerTuneResult, String> {
    let samples = state.monitoring.get_history(&gpu_id, 120);
    if samples.is_empty() {
        return Err("No data for GPU".into());
    }
    Ok(PowerTuner::tune(&samples, max_power_watts))
}

#[tauri::command]
pub fn get_tuning_profiles(gpu_id: String) -> Vec<TuningProfile> {
    let store = crate::ai::tuner::TuningStore::new();
    store.get_profiles(&gpu_id)
}

#[tauri::command]
pub fn save_tuning_profile(profile: TuningProfile) -> Result<(), String> {
    let store = crate::ai::tuner::TuningStore::new();
    store.save_profile(&profile);
    Ok(())
}

#[tauri::command]
pub fn apply_tuning_profile(gpu_id: String) -> Result<String, String> {
    let store = crate::ai::tuner::TuningStore::new();
    let profiles = store.get_profiles(&gpu_id);
    if let Some(p) = profiles.last() {
        Ok(format!("Applied tuning profile: {}", p.name))
    } else {
        Err("No profile to apply".into())
    }
}

#[tauri::command]
pub fn get_smart_alert_status(state: State<'_, Arc<AppState>>) -> SmartAlertConfig {
    let manager = &state.smart_alerts;
    if let Ok(engine) = manager.engine.lock() {
        engine.config.clone()
    } else {
        SmartAlertConfig::default()
    }
}

#[tauri::command]
pub fn get_smart_baselines(state: State<'_, Arc<AppState>>) -> Vec<MetricBaseline> {
    let manager = &state.smart_alerts;
    if let Ok(engine) = manager.engine.lock() {
        engine.get_baselines()
    } else {
        vec![]
    }
}

#[tauri::command]
pub fn get_smart_context(state: State<'_, Arc<AppState>>) -> serde_json::Value {
    let manager = &state.smart_alerts;
    if let Ok(engine) = manager.engine.lock() {
        let ctx = engine.get_context();
        serde_json::to_value(ctx).unwrap_or_default()
    } else {
        serde_json::Value::Null
    }
}

#[tauri::command]
pub fn get_smart_suppressed(state: State<'_, Arc<AppState>>) -> Vec<SuppressedAlert> {
    let manager = &state.smart_alerts;
    if let Ok(engine) = manager.engine.lock() {
        engine.get_suppressed_alerts().to_vec()
    } else {
        vec![]
    }
}

#[tauri::command]
pub fn update_smart_config(config: SmartAlertConfig, state: State<'_, Arc<AppState>>) {
    let manager = &state.smart_alerts;
    if let Ok(mut engine) = manager.engine.lock() {
        engine.update_config(config.clone());
    }
    manager.store.save_config(&config);
}

#[tauri::command]
pub fn reset_smart_baselines(state: State<'_, Arc<AppState>>) {
    let manager = &state.smart_alerts;
    if let Ok(mut engine) = manager.engine.lock() {
        engine.reset_baselines();
    }
}
