use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::hardware::GpuSample;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuningProfile {
    pub id: String,
    pub gpu_id: String,
    pub name: String,
    pub created_at: i64,
    pub fan_curve: Option<FanCurveResult>,
    pub clock_offsets: Option<ClockTuneResult>,
    pub power_limit: Option<PowerTuneResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurvePoint {
    pub temperature: f64,
    pub fan_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurveResult {
    pub points: Vec<FanCurvePoint>,
    pub estimated_max_temp: f64,
    pub estimated_noise_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockTuneResult {
    pub core_offset_mhz: i32,
    pub memory_offset_mhz: i32,
    pub peak_performance_score: f64,
    pub avg_temperature: f64,
    pub stability: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerTuneResult {
    pub limit_percent: f64,
    pub estimated_performance: f64,
    pub estimated_power_save: f64,
    pub efficiency_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuneProgress {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

pub struct FanCurveTuner;

impl FanCurveTuner {
    pub fn tune(samples: &[GpuSample]) -> FanCurveResult {
        let temps: Vec<f64> = samples.iter().filter_map(|s| s.temperature_celsius).collect();
        let fans: Vec<f64> = samples.iter().filter_map(|s| s.fan_speed_percent).collect();

        if temps.is_empty() {
            return Self::default_curve();
        }

        let _min_temp = temps.iter().cloned().fold(f64::MAX, f64::min);
        let max_temp = temps.iter().cloned().fold(f64::MIN, f64::max);

        let mut points = vec![
            FanCurvePoint { temperature: 30.0, fan_speed: 0.0 },
            FanCurvePoint { temperature: 40.0, fan_speed: 15.0 },
            FanCurvePoint { temperature: 50.0, fan_speed: 25.0 },
            FanCurvePoint { temperature: 60.0, fan_speed: 35.0 },
            FanCurvePoint { temperature: 65.0, fan_speed: 45.0 },
            FanCurvePoint { temperature: 70.0, fan_speed: 55.0 },
            FanCurvePoint { temperature: 75.0, fan_speed: 65.0 },
            FanCurvePoint { temperature: 80.0, fan_speed: 80.0 },
            FanCurvePoint { temperature: 85.0, fan_speed: 100.0 },
        ];

        if max_temp > 75.0 {
            let hot_samples: Vec<(&f64, &f64)> = temps.iter().zip(fans.iter())
                .filter(|(t, _)| **t > 70.0).collect();
            if !hot_samples.is_empty() {
                let avg_fan_above_70: f64 = hot_samples.iter().map(|(_, f)| **f).sum::<f64>()
                    / hot_samples.len() as f64;
                if avg_fan_above_70 < 60.0 {
                    for point in &mut points {
                        if point.temperature >= 70.0 {
                            point.fan_speed = (point.fan_speed + 15.0).min(100.0);
                        }
                    }
                }
            }
        }

        if max_temp < 65.0 {
            for point in &mut points {
                if point.temperature >= 60.0 {
                    point.fan_speed = (point.fan_speed - 10.0).max(0.0);
                }
            }
        }

        let estimated_max_temp = if max_temp > 80.0 { max_temp - 8.0 }
            else if max_temp > 70.0 { max_temp - 4.0 }
            else { max_temp };

        let noise = if points.iter().any(|p| p.fan_speed > 80.0) { "High" }
            else if points.iter().any(|p| p.fan_speed > 60.0) { "Moderate" }
            else { "Low" };

        FanCurveResult {
            points,
            estimated_max_temp,
            estimated_noise_level: noise.into(),
        }
    }

    fn default_curve() -> FanCurveResult {
        FanCurveResult {
            points: vec![
                FanCurvePoint { temperature: 30.0, fan_speed: 0.0 },
                FanCurvePoint { temperature: 50.0, fan_speed: 30.0 },
                FanCurvePoint { temperature: 65.0, fan_speed: 50.0 },
                FanCurvePoint { temperature: 75.0, fan_speed: 70.0 },
                FanCurvePoint { temperature: 85.0, fan_speed: 100.0 },
            ],
            estimated_max_temp: 82.0,
            estimated_noise_level: "Moderate".into(),
        }
    }
}

pub struct ClockTuner;

impl ClockTuner {
    pub fn tune(samples: &[GpuSample]) -> ClockTuneResult {
        let temps: Vec<f64> = samples.iter().filter_map(|s| s.temperature_celsius).collect();
        let utils: Vec<f64> = samples.iter().filter_map(|s| s.core_utilization_percent).collect();
        let clocks: Vec<f64> = samples.iter().filter_map(|s| s.core_clock_mhz).collect();

        if clocks.is_empty() {
            return ClockTuneResult {
                core_offset_mhz: 0,
                memory_offset_mhz: 0,
                peak_performance_score: 0.0,
                avg_temperature: 0.0,
                stability: "Unknown".into(),
            };
        }

        let avg_temp = temps.iter().sum::<f64>() / temps.len() as f64;
        let avg_clock = clocks.iter().sum::<f64>() / clocks.len() as f64;
        let avg_util = utils.iter().sum::<f64>() / utils.len() as f64;

        let max_safe_offset = if avg_temp < 60.0 { 150 }
            else if avg_temp < 70.0 { 100 }
            else if avg_temp < 80.0 { 50 }
            else { 0 };

        let mem_offset = max_safe_offset / 2;

        let peak_score = if avg_util > 50.0 {
            avg_clock * (1.0 - (avg_temp - 50.0).max(0.0) / 100.0)
        } else {
            avg_clock * 0.8
        };

        let stability = if avg_temp < 70.0 { "High" }
            else if avg_temp < 80.0 { "Medium" }
            else { "Low" };

        ClockTuneResult {
            core_offset_mhz: max_safe_offset,
            memory_offset_mhz: mem_offset,
            peak_performance_score: peak_score,
            avg_temperature: avg_temp,
            stability: stability.into(),
        }
    }
}

pub struct PowerTuner;

impl PowerTuner {
    pub fn tune(samples: &[GpuSample], max_power_watts: f64) -> PowerTuneResult {
        let temps: Vec<f64> = samples.iter().filter_map(|s| s.temperature_celsius).collect();
        let powers: Vec<f64> = samples.iter().filter_map(|s| s.power_watts).collect();
        let utils: Vec<f64> = samples.iter().filter_map(|s| s.core_utilization_percent).collect();

        if powers.is_empty() || max_power_watts <= 0.0 {
            return PowerTuneResult {
                limit_percent: 100.0,
                estimated_performance: 100.0,
                estimated_power_save: 0.0,
                efficiency_score: 1.0,
            };
        }

        let _avg_temp = temps.iter().sum::<f64>() / temps.len() as f64;
        let max_temp = temps.iter().cloned().fold(f64::MIN, f64::max);
        let avg_power = powers.iter().sum::<f64>() / powers.len() as f64;
        let avg_util = utils.iter().sum::<f64>() / utils.len() as f64;

        let power_ratio = avg_power / max_power_watts;

        let (limit_percent, perf, power_save, efficiency) = if max_temp > 85.0 {
            (80.0, 92.0, 20.0, 1.15)
        } else if max_temp > 80.0 {
            (85.0, 95.0, 15.0, 1.12)
        } else if max_temp > 75.0 && power_ratio > 0.8 {
            (90.0, 97.0, 10.0, 1.08)
        } else if avg_util < 50.0 && power_ratio > 0.7 {
            (70.0, 85.0, 30.0, 1.21)
        } else {
            (100.0, 100.0, 0.0, 1.0)
        };

        PowerTuneResult {
            limit_percent,
            estimated_performance: perf,
            estimated_power_save: power_save,
            efficiency_score: efficiency,
        }
    }
}

pub struct TuningStore {
    path: PathBuf,
}

impl Default for TuningStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TuningStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gpucontrol-pro")
            .join("tuning_profiles.json");
        Self { path }
    }

    fn load_all(&self) -> Vec<TuningProfile> {
        if !self.path.exists() {
            return vec![];
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                error!("Failed to read tuning profiles: {e}");
                vec![]
            }
        }
    }

    fn save_all(&self, profiles: &[TuningProfile]) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(profiles) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write tuning profiles: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize tuning profiles: {e}");
            }
        }
    }

    pub fn get_profiles(&self, gpu_id: &str) -> Vec<TuningProfile> {
        let all = self.load_all();
        all.into_iter().filter(|p| p.gpu_id == gpu_id).collect()
    }

    pub fn save_profile(&self, profile: &TuningProfile) {
        let mut all = self.load_all();
        all.push(profile.clone());
        if all.len() > 50 {
            all.remove(0);
        }
        self.save_all(&all);
    }
}
