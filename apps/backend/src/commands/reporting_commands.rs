use std::sync::Arc;

use chrono::Utc;
use tauri::State;

use super::state::AppState;
use crate::reporting::report::{MetricRow, Report, ReportConfig, ReportSummary, TimeRange};
use crate::reporting::store::ReportStore;
use crate::reporting::HtmlReport;

#[tauri::command]
pub async fn generate_report(
    name: String,
    config: ReportConfig,
    state: State<'_, Arc<AppState>>,
) -> Result<ReportSummary, String> {
    let mut report = Report::new(name, config);
    let store = ReportStore::new();

    let gpu_ids = if report.config.gpu_ids.is_empty() {
        state
            .monitoring
            .gpus()
            .iter()
            .map(|g| g.id.clone())
            .collect()
    } else {
        report.config.gpu_ids.clone()
    };

    for gpu_id in &gpu_ids {
        let gpu_name = state
            .monitoring
            .gpus()
            .iter()
            .find(|g| &g.id == gpu_id)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| gpu_id.clone());

        let count = match &report.config.time_range {
            TimeRange::LastMinutes(m) => (*m as usize) * 60,
            TimeRange::LastHours(h) => (*h as usize) * 3600,
            TimeRange::LastDays(d) => (*d as usize) * 86400,
            TimeRange::Custom { start, end } => ((end - start) as usize).max(1),
        };

        let count = count.min(3600);

        let samples = state.monitoring.get_history(gpu_id, count);
        let first_ts = samples.first().map(|s| s.timestamp).unwrap_or(0);
        let last_ts = samples.last().map(|s| s.timestamp).unwrap_or(0);
        let duration_secs = (last_ts - first_ts).max(0) as u64;

        let mut metrics = vec![];

        if samples.is_empty() {
            continue;
        }

        if report.config.include_temperature {
            let vals: Vec<f64> = samples
                .iter()
                .filter_map(|s| s.temperature_celsius)
                .collect();
            if let Some(current) = samples.last().and_then(|s| s.temperature_celsius) {
                metrics.push(MetricRow {
                    metric: "Temperature".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "°C".into(),
                });
            }
        }

        if report.config.include_utilization {
            let vals: Vec<f64> = samples
                .iter()
                .filter_map(|s| s.core_utilization_percent)
                .collect();
            if let Some(current) = samples.last().and_then(|s| s.core_utilization_percent) {
                metrics.push(MetricRow {
                    metric: "GPU Utilization".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "%".into(),
                });
            }

            let vals: Vec<f64> = samples
                .iter()
                .filter_map(|s| s.memory_used_mb.map(|v| v as f64))
                .collect();
            if let Some(current) = samples
                .last()
                .and_then(|s| s.memory_used_mb.map(|v| v as f64))
            {
                metrics.push(MetricRow {
                    metric: "Memory Used".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "MB".into(),
                });
            }
        }

        if report.config.include_clocks {
            let vals: Vec<f64> = samples.iter().filter_map(|s| s.core_clock_mhz).collect();
            if let Some(current) = samples.last().and_then(|s| s.core_clock_mhz) {
                metrics.push(MetricRow {
                    metric: "Core Clock".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "MHz".into(),
                });
            }

            let vals: Vec<f64> = samples.iter().filter_map(|s| s.memory_clock_mhz).collect();
            if let Some(current) = samples.last().and_then(|s| s.memory_clock_mhz) {
                metrics.push(MetricRow {
                    metric: "Memory Clock".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "MHz".into(),
                });
            }
        }

        if report.config.include_fan {
            let vals: Vec<f64> = samples.iter().filter_map(|s| s.fan_speed_percent).collect();
            if let Some(current) = samples.last().and_then(|s| s.fan_speed_percent) {
                metrics.push(MetricRow {
                    metric: "Fan Speed".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "%".into(),
                });
            }
        }

        if report.config.include_power {
            let vals: Vec<f64> = samples.iter().filter_map(|s| s.power_watts).collect();
            if let Some(current) = samples.last().and_then(|s| s.power_watts) {
                metrics.push(MetricRow {
                    metric: "Power Draw".into(),
                    current,
                    min: vals.iter().cloned().fold(f64::MAX, f64::min),
                    max: vals.iter().cloned().fold(f64::MIN, f64::max),
                    avg: vals.iter().sum::<f64>() / vals.len() as f64,
                    unit: "W".into(),
                });
            }
        }

        report
            .sections
            .push(crate::reporting::report::GpuReportSection {
                gpu_id: gpu_id.clone(),
                gpu_name,
                sampling_duration_secs: duration_secs,
                sample_count: samples.len(),
                metrics,
            });
    }

    let now = Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let html = HtmlReport::generate(&report.name, &now, &report.sections);
    report.html_content = Some(html);

    let summary = report.summary();
    store.save_report(&report);
    Ok(summary)
}

#[tauri::command]
pub async fn list_reports() -> Vec<ReportSummary> {
    let store = ReportStore::new();
    let ids = store.list_reports();
    let mut summaries = vec![];
    for id in ids {
        if let Some(report) = store.load_report(&id) {
            summaries.push(report.summary());
        }
    }
    summaries
}

#[tauri::command]
pub async fn get_report(id: String) -> Result<Report, String> {
    let store = ReportStore::new();
    store
        .load_report(&id)
        .ok_or_else(|| format!("Report {id} not found"))
}

#[tauri::command]
pub async fn delete_report(id: String) -> Result<(), String> {
    let store = ReportStore::new();
    store.delete_report(&id);
    Ok(())
}

#[tauri::command]
pub async fn export_report_csv(id: String) -> Result<String, String> {
    let store = ReportStore::new();
    let report = store
        .load_report(&id)
        .ok_or_else(|| format!("Report {id} not found"))?;

    let mut csv = String::from("gpu_id,metric,current,min,max,avg,unit\n");
    for section in &report.sections {
        for m in &section.metrics {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                section.gpu_id, m.metric, m.current, m.min, m.max, m.avg, m.unit
            ));
        }
    }
    Ok(csv)
}
