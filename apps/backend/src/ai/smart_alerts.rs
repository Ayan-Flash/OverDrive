use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::hardware::GpuSample;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SystemContext {
    Idle,
    Normal,
    Gaming,
    Benchmarking,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBaseline {
    pub metric: String,
    pub mean: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub sample_count: u64,
    pub last_updated: i64,
    pub hourly_patterns: Vec<f64>,
    pub load_based_means: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextInfo {
    pub context: SystemContext,
    pub confidence: f64,
    pub avg_utilization: Option<f64>,
    pub avg_temperature: Option<f64>,
    pub avg_power: Option<f64>,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveThreshold {
    pub metric: String,
    pub base_threshold: f64,
    pub dynamic_threshold: f64,
    pub sensitivity: f64,
    pub multiplier: f64,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuppressedAlert {
    pub rule_id: String,
    pub metric: String,
    pub message: String,
    pub original_timestamp: i64,
    pub suppress_count: u32,
    pub last_suppressed: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAlertConfig {
    pub enabled: bool,
    pub learning_window_minutes: u64,
    pub adaptive_sensitivity: f64,
    pub suppress_duplicates: bool,
    pub duplicate_window_secs: u64,
    pub context_aware: bool,
    pub gaming_relaxation_factor: f64,
    pub idle_strictness_factor: f64,
    pub auto_reset_baselines: bool,
    pub baseline_reset_hours: u64,
}

impl Default for SmartAlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            learning_window_minutes: 60,
            adaptive_sensitivity: 1.0,
            suppress_duplicates: true,
            duplicate_window_secs: 300,
            context_aware: true,
            gaming_relaxation_factor: 1.5,
            idle_strictness_factor: 0.7,
            auto_reset_baselines: false,
            baseline_reset_hours: 24,
        }
    }
}

pub struct BaselineLearner {
    metric_windows: HashMap<String, Vec<f64>>,
    max_window: usize,
    baselines: HashMap<String, MetricBaseline>,
}

impl BaselineLearner {
    pub fn new(window_minutes: u64) -> Self {
        let max_window = (window_minutes * 60) as usize;
        Self {
            metric_windows: HashMap::new(),
            max_window,
            baselines: HashMap::new(),
        }
    }

    pub fn feed(&mut self, metric: &str, value: f64, utilization: f64) {
        let window = self.metric_windows.entry(metric.to_string()).or_default();
        window.push(value);
        if window.len() > self.max_window {
            window.remove(0);
        }
        self.recalculate(metric, utilization);
    }

    fn recalculate(&mut self, metric: &str, current_util: f64) {
        let window = match self.metric_windows.get(metric) {
            Some(w) if w.len() >= 10 => w,
            _ => return,
        };
        let value = *window.last().unwrap_or(&0.0);

        let n = window.len() as f64;
        let sum: f64 = window.iter().sum();
        let mean = sum / n;
        let variance = window.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();
        let min = window.iter().cloned().fold(f64::MAX, f64::min);
        let max = window.iter().cloned().fold(f64::MIN, f64::max);

        let hour = Local::now().hour();
        let mut hourly = vec![0.0; 24];
        if let Some(existing) = self.baselines.get(metric) {
            hourly = existing.hourly_patterns.clone();
        }
        let hour_idx = hour as usize;
        if hour_idx < 24 {
            let alpha = 0.1;
            hourly[hour_idx] = hourly[hour_idx] * (1.0 - alpha) + value * alpha;
        }

        let load_bucket = format!("{:.0}", (current_util / 20.0).floor() * 20.0);
        let mut load_based = HashMap::new();
        if let Some(existing) = self.baselines.get(metric) {
            load_based = existing.load_based_means.clone();
        }
        let entry = load_based.entry(load_bucket).or_insert(0.0);
        *entry = *entry * 0.9 + value * 0.1;

        self.baselines.insert(metric.to_string(), MetricBaseline {
            metric: metric.to_string(),
            mean,
            std_dev,
            min,
            max,
            sample_count: window.len() as u64,
            last_updated: chrono::Utc::now().timestamp(),
            hourly_patterns: hourly,
            load_based_means: load_based,
        });
    }

    pub fn get_baseline(&self, metric: &str) -> Option<&MetricBaseline> {
        self.baselines.get(metric)
    }

    pub fn get_all_baselines(&self) -> Vec<MetricBaseline> {
        self.baselines.values().cloned().collect()
    }

    pub fn adaptive_threshold(&self, metric: &str, config: &SmartAlertConfig, context: &SystemContext) -> Option<AdaptiveThreshold> {
        let baseline = self.baselines.get(metric)?;
        let base_mult = match context {
            SystemContext::Gaming | SystemContext::Benchmarking => config.gaming_relaxation_factor,
            SystemContext::Idle => config.idle_strictness_factor,
            _ => 1.0,
        };
        let multiplier = base_mult * config.adaptive_sensitivity;
        let dynamic_threshold = baseline.mean + multiplier * baseline.std_dev;

        Some(AdaptiveThreshold {
            metric: metric.to_string(),
            base_threshold: baseline.mean + baseline.std_dev,
            dynamic_threshold,
            sensitivity: config.adaptive_sensitivity,
            multiplier,
            enabled: config.enabled,
        })
    }

    pub fn is_anomalous(&self, metric: &str, value: f64, config: &SmartAlertConfig, context: &SystemContext) -> Option<bool> {
        let threshold = self.adaptive_threshold(metric, config, context)?;
        if threshold.dynamic_threshold < 0.01 {
            return Some(false);
        }
        Some(value > threshold.dynamic_threshold)
    }

    pub fn reset(&mut self) {
        self.metric_windows.clear();
        self.baselines.clear();
    }
}

pub struct ContextDetector {
    samples: Vec<(i64, Option<f64>, Option<f64>, Option<f64>)>,
    current: ContextInfo,
}

impl Default for ContextDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextDetector {
    pub fn new() -> Self {
        Self {
            samples: vec![],
            current: ContextInfo {
                context: SystemContext::Unknown,
                confidence: 0.0,
                avg_utilization: None,
                avg_temperature: None,
                avg_power: None,
                duration_secs: 0,
            },
        }
    }

    pub fn feed(&mut self, sample: &GpuSample) {
        let now = chrono::Utc::now().timestamp();
        self.samples.push((now, sample.core_utilization_percent, sample.temperature_celsius, sample.power_watts));
        if self.samples.len() > 300 {
            self.samples.remove(0);
        }
        self.detect();
    }

    fn detect(&mut self) {
        if self.samples.len() < 10 {
            return;
        }

        let recent: Vec<_> = self.samples.iter().rev().take(60).collect();
        let avg = |iter: Vec<f64>| (!iter.is_empty()).then(|| iter.iter().sum::<f64>() / iter.len() as f64);
        let avg_util = avg(recent.iter().filter_map(|(_,u,_,_)| *u).collect());
        let avg_temp = avg(recent.iter().filter_map(|(_,_,t,_)| *t).collect());
        let avg_power = avg(recent.iter().filter_map(|(_,_,_,p)| *p).collect());

        let util_std_dev = avg_util.map(|mean| avg(recent.iter().filter_map(|(_,u,_,_)| u.map(|v| (v - mean).powi(2))).collect()).unwrap_or(0.0).sqrt());

        let (context, confidence) = match (avg_util, avg_temp, util_std_dev) {
            (Some(util), Some(temp), Some(_)) if util < 5.0 && temp < 50.0 => (SystemContext::Idle, 0.9),
            (Some(util), Some(temp), Some(std_dev)) if util > 80.0 && std_dev < 15.0 && temp > 65.0 => (SystemContext::Benchmarking, 0.8),
            (Some(util), _, Some(std_dev)) if util > 40.0 && std_dev > 20.0 => (SystemContext::Gaming, 0.7),
            (Some(util), _, _) if util > 5.0 => (SystemContext::Normal, 0.6),
            (Some(_), Some(_), _) => (SystemContext::Unknown, 0.3),
            _ => (SystemContext::Unknown, 0.0),
        };

        let context_changed = self.current.context != context;
        let duration = if context_changed {
            self.current.duration_secs = 0;
            0
        } else {
            self.current.duration_secs + 1
        };

        self.current = ContextInfo {
            context,
            confidence,
            avg_utilization: avg_util,
            avg_temperature: avg_temp,
            avg_power,
            duration_secs: duration,
        };
    }

    pub fn current_context(&self) -> &ContextInfo {
        &self.current
    }

    pub fn reset(&mut self) {
        self.samples.clear();
        self.current = ContextInfo {
            context: SystemContext::Unknown,
            confidence: 0.0,
            avg_utilization: None,
            avg_temperature: None,
            avg_power: None,
            duration_secs: 0,
        };
    }
}

pub struct AlertSuppressor {
    suppressed: Vec<SuppressedAlert>,
    window_secs: u64,
}

impl AlertSuppressor {
    pub fn new(window_secs: u64) -> Self {
        Self {
            suppressed: vec![],
            window_secs,
        }
    }

    pub fn should_suppress(&mut self, rule_id: &str, metric: &str, message: &str) -> bool {
        let now = chrono::Utc::now().timestamp();

        if let Some(entry) = self.suppressed.iter_mut()
            .find(|s| s.rule_id == rule_id && s.metric == metric && s.message == message)
        {
            if (now - entry.last_suppressed) < self.window_secs as i64 {
                entry.suppress_count += 1;
                entry.last_suppressed = now;
                return true;
            }
            entry.original_timestamp = now;
            entry.suppress_count = 0;
            entry.last_suppressed = now;
            return false;
        }

        self.suppressed.push(SuppressedAlert {
            rule_id: rule_id.to_string(),
            metric: metric.to_string(),
            message: message.to_string(),
            original_timestamp: now,
            suppress_count: 0,
            last_suppressed: now,
        });

        if self.suppressed.len() > 100 {
            self.suppressed.remove(0);
        }

        false
    }

    pub fn get_suppressed(&self) -> &[SuppressedAlert] {
        &self.suppressed
    }

    pub fn reset(&mut self) {
        self.suppressed.clear();
    }
}

pub struct SmartAlertEngine {
    pub config: SmartAlertConfig,
    pub learner: BaselineLearner,
    pub context_detector: ContextDetector,
    pub suppressor: AlertSuppressor,
}

impl Default for SmartAlertEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartAlertEngine {
    pub fn new() -> Self {
        let config = SmartAlertConfig::default();
        Self {
            learner: BaselineLearner::new(config.learning_window_minutes),
            context_detector: ContextDetector::new(),
            suppressor: AlertSuppressor::new(config.duplicate_window_secs),
            config,
        }
    }

    pub fn feed_sample(&mut self, sample: &GpuSample) {
        self.context_detector.feed(sample);

        let metrics = [
            ("temperature", sample.temperature_celsius),
            ("fan_speed", sample.fan_speed_percent),
            ("power", sample.power_watts),
            ("core_clock", sample.core_clock_mhz),
            ("core_utilization", sample.core_utilization_percent),
            ("memory_utilization", sample.memory_utilization_percent),
        ];

        let util = sample.core_utilization_percent;
        for (name, value) in &metrics {
            if let (Some(value), Some(util)) = (value, util) { self.learner.feed(name, *value, util); }
        }
    }

    pub fn evaluate_alert(&mut self, rule_id: &str, metric: &str, value: f64, message: &str) -> bool {
        if !self.config.enabled {
            return true;
        }

        let context = self.context_detector.current_context();

        if self.config.context_aware
            && (context.context == SystemContext::Gaming || context.context == SystemContext::Benchmarking) {
                if let Some(threshold) = self.learner.adaptive_threshold(metric, &self.config, &context.context) {
                    if value < threshold.dynamic_threshold {
                        info!("Smart alert suppressed: {message} (context: {:?}, threshold: {:.1})", context.context, threshold.dynamic_threshold);
                        return false;
                    }
                }
            }

        if self.config.suppress_duplicates
            && self.suppressor.should_suppress(rule_id, metric, message) {
                warn!("Smart alert suppressed (duplicate): {message}");
                return false;
            }

        if let Some(is_anom) = self.learner.is_anomalous(metric, value, &self.config, &context.context) {
            if !is_anom {
                info!("Smart alert suppressed (within baseline): {message}");
                return false;
            }
        }

        true
    }

    pub fn get_baselines(&self) -> Vec<MetricBaseline> {
        self.learner.get_all_baselines()
    }

    pub fn get_context(&self) -> &ContextInfo {
        self.context_detector.current_context()
    }

    pub fn get_suppressed_alerts(&self) -> &[SuppressedAlert] {
        self.suppressor.get_suppressed()
    }

    pub fn update_config(&mut self, config: SmartAlertConfig) {
        let window_changed = config.learning_window_minutes != self.config.learning_window_minutes;
        let suppress_window_changed = config.duplicate_window_secs != self.config.duplicate_window_secs;
        self.config = config;

        if window_changed {
            self.learner = BaselineLearner::new(self.config.learning_window_minutes);
        }
        if suppress_window_changed {
            self.suppressor = AlertSuppressor::new(self.config.duplicate_window_secs);
        }
    }

    pub fn reset_baselines(&mut self) {
        self.learner.reset();
        self.context_detector.reset();
        self.suppressor.reset();
        info!("Smart alert baselines reset");
    }
}

pub struct SmartAlertStore {
    path: PathBuf,
}

impl Default for SmartAlertStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartAlertStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("smart_alert_config.json");
        Self { path }
    }

    pub fn load_config(&self) -> SmartAlertConfig {
        if !self.path.exists() {
            return SmartAlertConfig::default();
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                warn!("Failed to read smart alert config: {e}");
                SmartAlertConfig::default()
            }
        }
    }

    pub fn save_config(&self, config: &SmartAlertConfig) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(config) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    warn!("Failed to write smart alert config: {e}");
                }
            }
            Err(e) => {
                warn!("Failed to serialize smart alert config: {e}");
            }
        }
    }
}

pub struct SmartAlertManager {
    pub engine: std::sync::Mutex<SmartAlertEngine>,
    pub store: SmartAlertStore,
}

impl Default for SmartAlertManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartAlertManager {
    pub fn new() -> Self {
        let store = SmartAlertStore::new();
        let config = store.load_config();
        let mut engine = SmartAlertEngine::new();
        engine.config = config;
        Self {
            engine: std::sync::Mutex::new(engine),
            store,
        }
    }

    pub fn feed_sample(&self, sample: &GpuSample) {
        if let Ok(mut engine) = self.engine.lock() {
            engine.feed_sample(sample);
        }
    }

    pub fn evaluate_alert(&self, rule_id: &str, metric: &str, value: f64, message: &str) -> bool {
        if let Ok(mut engine) = self.engine.lock() {
            engine.evaluate_alert(rule_id, metric, value, message)
        } else {
            true
        }
    }
}
