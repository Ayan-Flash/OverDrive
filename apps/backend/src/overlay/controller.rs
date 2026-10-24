use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tracing::info;

use super::config::OverlayConfig;
use super::game_detection::GameDetector;
use crate::monitoring::MonitoringEngine;

pub struct OverlayController {
    config: OverlayConfig,
    running: AtomicBool,
    game_detector: GameDetector,
    monitoring: Arc<MonitoringEngine>,
}

impl OverlayController {
    pub fn new(monitoring: Arc<MonitoringEngine>) -> Self {
        Self {
            config: OverlayConfig::default(),
            running: AtomicBool::new(false),
            game_detector: GameDetector::new(),
            monitoring,
        }
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::Relaxed) {
            return Err("Overlay is already running".to_string());
        }
        self.running.store(true, Ordering::Relaxed);
        info!("Overlay started");
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if !self.running.load(Ordering::Relaxed) {
            return Err("Overlay is not running".to_string());
        }
        self.running.store(false, Ordering::Relaxed);
        info!("Overlay stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn config(&self) -> &OverlayConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: OverlayConfig) {
        self.config = config;
        info!("Overlay config updated");
    }

    pub fn is_game_running(&self) -> bool {
        self.game_detector.is_any_game_running()
    }

    pub fn detected_games(&self) -> Vec<String> {
        self.game_detector.detect_running_games()
    }

    pub fn should_show_overlay(&self) -> bool {
        if !self.config.enabled || !self.running.load(Ordering::Relaxed) {
            return false;
        }
        if self.config.auto_hide_no_game && !self.game_detector.is_any_game_running() {
            return false;
        }
        true
    }

    pub fn get_overlay_data(&self) -> Vec<(String, String, f64)> {
        let gpus = self.monitoring.gpus();
        let mut data = Vec::new();

        for gpu in &gpus {
            if let Some(sample) = self.monitoring.get_latest(&gpu.id) {
                for metric in &self.config.metrics {
                    if !metric.enabled {
                        continue;
                    }
                    let value = match metric.metric.as_str() {
                        "temperature_celsius" => sample.temperature_celsius,
                        "core_clock_mhz" => sample.core_clock_mhz,
                        "memory_clock_mhz" => sample.memory_clock_mhz,
                        "fan_speed_percent" => sample.fan_speed_percent,
                        "power_watts" => sample.power_watts,
                        "core_voltage_mv" => sample.core_voltage_mv,
                        "core_utilization_percent" => sample.core_utilization_percent,
                        "memory_utilization_percent" => sample.memory_utilization_percent,
                        _ => continue,
                    };
                    if let Some(value) = value { data.push((gpu.name.clone(), metric.label.clone(), value)); }
                }
            }
        }

        data
    }
}
