use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::error;

use super::anomaly::Anomaly;
use super::optimizer::OptimizationSuggestion;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct AiState {
    pub anomalies: Vec<Anomaly>,
    pub suggestions: Vec<OptimizationSuggestion>,
    pub baseline_stats: HashMap<String, BaselineMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineMetrics {
    pub avg_temp: f64,
    pub max_temp: f64,
    pub avg_fan: f64,
    pub avg_power: f64,
    pub avg_utilization: f64,
    pub sample_count: u64,
}

pub struct AiStore {
    path: PathBuf,
    state: AiState,
}

impl Default for AiStore {
    fn default() -> Self {
        Self::new()
    }
}

impl AiStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("ai_state.json");
        let state = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(e) => {
                    error!("Failed to read AI state: {e}");
                    AiState::default()
                }
            }
        } else {
            AiState::default()
        };
        Self { path, state }
    }

    pub fn save(&self) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(&self.state) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write AI state: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize AI state: {e}");
            }
        }
    }

    pub fn add_anomaly(&mut self, anomaly: Anomaly) {
        self.state.anomalies.push(anomaly);
        if self.state.anomalies.len() > 1000 {
            self.state.anomalies.remove(0);
        }
        self.save();
    }

    pub fn add_suggestion(&mut self, suggestion: OptimizationSuggestion) {
        self.state.suggestions.push(suggestion);
        if self.state.suggestions.len() > 100 {
            self.state.suggestions.remove(0);
        }
        self.save();
    }

    pub fn get_anomalies(&self, limit: usize) -> Vec<Anomaly> {
        let len = self.state.anomalies.len();
        let start = len.saturating_sub(limit);
        self.state.anomalies[start..].to_vec()
    }

    pub fn get_suggestions(&self) -> Vec<OptimizationSuggestion> {
        self.state.suggestions.clone()
    }

    pub fn mark_suggestion_applied(&mut self, id: &str) {
        if let Some(s) = self.state.suggestions.iter_mut().find(|s| s.id == id) {
            s.applied = true;
        }
        self.save();
    }

    pub fn clear_anomalies(&mut self) {
        self.state.anomalies.clear();
        self.save();
    }

    pub fn update_baseline(&mut self, gpu_id: &str, temp: f64, fan: f64, power: f64, util: f64) {
        let entry = self.state.baseline_stats.entry(gpu_id.into())
            .or_insert(BaselineMetrics {
                avg_temp: 0.0, max_temp: 0.0, avg_fan: 0.0,
                avg_power: 0.0, avg_utilization: 0.0, sample_count: 0,
            });
        let n = entry.sample_count as f64;
        entry.avg_temp = (entry.avg_temp * n + temp) / (n + 1.0);
        entry.avg_fan = (entry.avg_fan * n + fan) / (n + 1.0);
        entry.avg_power = (entry.avg_power * n + power) / (n + 1.0);
        entry.avg_utilization = (entry.avg_utilization * n + util) / (n + 1.0);
        entry.max_temp = entry.max_temp.max(temp);
        entry.sample_count += 1;
        self.save();
    }
}

