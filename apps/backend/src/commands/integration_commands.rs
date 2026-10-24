use std::sync::Arc;

use tauri::State;

use super::state::AppState;
use crate::integrations::{DiscordWebhook, IntegrationConfig, IntegrationStore};

#[tauri::command]
pub async fn get_integration_config() -> Result<IntegrationConfig, String> {
    let store = IntegrationStore::new();
    Ok(store.load())
}

#[tauri::command]
pub async fn save_integration_config(config: IntegrationConfig) -> Result<(), String> {
    let store = IntegrationStore::new();
    store.save(&config);
    Ok(())
}

#[tauri::command]
pub async fn test_discord_webhook(webhook_url: String) -> Result<String, String> {
    DiscordWebhook::test_connection(&webhook_url).await
}

#[tauri::command]
pub async fn send_discord_alert(
    webhook_url: String,
    gpu_name: String,
    metric: String,
    value: f64,
    threshold: f64,
    severity: String,
) -> Result<(), String> {
    DiscordWebhook::send_alert(
        &webhook_url,
        "GPUControl Pro",
        "",
        &gpu_name,
        &metric,
        value,
        threshold,
        &severity,
    )
    .await
}

#[tauri::command]
pub async fn send_discord_report(
    webhook_url: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let gpus = state.monitoring.gpus();
    let gpu_id = gpus.first().map(|g| g.id.as_str()).unwrap_or("0");
    let samples = state.monitoring.get_history(gpu_id, 1);
    let sample = samples.first().ok_or("No GPU data available")?;

    let metrics: Vec<(String, f64, String)> = [
        ("Temperature", sample.temperature_celsius, "°C"),
        ("GPU Utilization", sample.core_utilization_percent, "%"),
        ("Memory Used", sample.memory_used_mb.map(|v| v as f64), "MB"),
        ("Fan Speed", sample.fan_speed_percent, "%"),
        ("Power Draw", sample.power_watts, "W"),
        ("Core Clock", sample.core_clock_mhz, "MHz"),
        ("Memory Clock", sample.memory_clock_mhz, "MHz"),
    ].into_iter().filter_map(|(label, value, unit)| value.map(|v| (label.into(), v, unit.into()))).collect();

    DiscordWebhook::send_report(&webhook_url, "GPUControl Pro", "", "GPU 0", &metrics).await
}

#[tauri::command]
pub async fn start_obs_source(
    _port: u16,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let engine = state.monitoring.clone();
    state.obs_source.start(Box::new(move || {
        let gpus = engine.gpus();
        let gpu_id = gpus.first().map(|g| g.id.as_str()).unwrap_or("0");
        let samples = engine.get_history(gpu_id, 1);
        if let Some(s) = samples.first() {
            crate::integrations::obs::ObsPayload {
                gpu_name: "GPU 0".into(),
                metrics: vec![
                    crate::integrations::obs::ObsMetric {
                        label: "Temperature".into(),
                        value: s.temperature_celsius.map(|v| format!("{v:.1}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "°C".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "GPU".into(),
                        value: s.core_utilization_percent.map(|v| format!("{v:.1}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "%".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "VRAM".into(),
                        value: s.memory_used_mb.map(|v| format!("{v:.1}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "MB".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "Fan".into(),
                        value: s.fan_speed_percent.map(|v| format!("{v:.1}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "%".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "Power".into(),
                        value: s.power_watts.map(|v| format!("{v:.1}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "W".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "Core Clock".into(),
                        value: s.core_clock_mhz.map(|v| format!("{v:.0}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "MHz".into(),
                    },
                    crate::integrations::obs::ObsMetric {
                        label: "Mem Clock".into(),
                        value: s.memory_clock_mhz.map(|v| format!("{v:.0}")).unwrap_or_else(|| "Unavailable".into()),
                        unit: "MHz".into(),
                    },
                ],
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }
        } else {
            crate::integrations::obs::ObsPayload {
                gpu_name: "No GPU".into(),
                metrics: vec![],
                timestamp: 0,
            }
        }
    }))
}

#[tauri::command]
pub async fn stop_obs_source(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.obs_source.stop();
    Ok(())
}

#[tauri::command]
pub async fn is_obs_running(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.obs_source.is_running())
}
