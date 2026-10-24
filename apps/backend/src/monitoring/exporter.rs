use serde::{Deserialize, Serialize};

use crate::hardware::GpuSample;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSample {
    pub t: i64,
    pub temp: Option<f64>,
    pub core: Option<f64>,
    pub mem: Option<f64>,
    pub fan: Option<f64>,
    pub power: Option<f64>,
    pub core_util: Option<f64>,
    pub mem_util: Option<f64>,
    pub mem_used_mb: Option<u64>,
    pub mem_total_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedGpuData {
    pub gpu_id: String,
    pub samples: Vec<ExportedSample>,
}

pub struct Exporter;

impl Exporter {
    pub fn export_samples(samples: &[&GpuSample]) -> Vec<ExportedSample> {
        samples
            .iter()
            .map(|s| ExportedSample {
                t: s.timestamp,
                temp: s.temperature_celsius.map(|v| (v * 10.0).round() / 10.0),
                core: s.core_clock_mhz.map(f64::round),
                mem: s.memory_clock_mhz.map(f64::round),
                fan: s.fan_speed_percent.map(|v| (v * 10.0).round() / 10.0),
                power: s.power_watts.map(|v| (v * 10.0).round() / 10.0),
                core_util: s.core_utilization_percent.map(|v| (v * 10.0).round() / 10.0),
                mem_util: s.memory_utilization_percent.map(|v| (v * 10.0).round() / 10.0),
                mem_used_mb: s.memory_used_mb,
                mem_total_mb: s.memory_total_mb,
            })
            .collect()
    }

    pub fn export_to_csv(samples: &[&GpuSample]) -> String {
        let mut csv = String::from("timestamp,temperature,core_clock,memory_clock,fan_speed,power,core_util,memory_util,memory_used_mb,memory_total_mb\n");
        for s in samples {
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{}\n",
                s.timestamp,
                s.temperature_celsius.map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default(),
                s.core_clock_mhz.map(|v| v.round().to_string()).unwrap_or_default(),
                s.memory_clock_mhz.map(|v| v.round().to_string()).unwrap_or_default(),
                s.fan_speed_percent.map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default(),
                s.power_watts.map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default(),
                s.core_utilization_percent.map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default(),
                s.memory_utilization_percent.map(|v| ((v * 10.0).round() / 10.0).to_string()).unwrap_or_default(),
                s.memory_used_mb.map(|v| v.to_string()).unwrap_or_default(),
                s.memory_total_mb.map(|v| v.to_string()).unwrap_or_default(),
            ));
        }
        csv
    }
}
