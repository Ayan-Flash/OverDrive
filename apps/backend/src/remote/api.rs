use std::sync::Arc;

use serde_json::Value;

use crate::monitoring::MonitoringEngine;
use crate::alerts::AlertEngine;

pub struct RemoteApi {
    pub monitoring: Arc<MonitoringEngine>,
    pub alerts: Arc<AlertEngine>,
}

impl RemoteApi {
    pub fn new(monitoring: Arc<MonitoringEngine>, alerts: Arc<AlertEngine>) -> Self {
        Self { monitoring, alerts }
    }

    pub fn list_gpus(&self) -> Value {
        let gpus = self.monitoring.gpus();
        serde_json::to_value(gpus).unwrap_or(serde_json::json!([]))
    }

    pub fn get_gpu_data(&self, gpu_id: &str) -> Value {
        let sample = self.monitoring.collect_once(gpu_id);
        let control = self.monitoring.get_control_state(gpu_id).ok();
        serde_json::json!({
            "sample": sample,
            "control": control,
        })
    }

    pub fn get_gpu_history(&self, gpu_id: &str, count: usize) -> Value {
        let samples = self.monitoring.get_exported_samples(gpu_id, count);
        let aggregated = self.monitoring.get_aggregated(gpu_id, count);
        serde_json::json!({
            "samples": samples,
            "aggregated": aggregated,
        })
    }

    pub fn get_alerts(&self, limit: usize) -> Value {
        let history = self.alerts.get_history(limit);
        let rules = self.alerts.get_rules();
        serde_json::json!({
            "events": history,
            "rules": rules,
        })
    }

    pub fn get_status(&self) -> Value {
        let gpu_count = self.monitoring.gpus().len();
        let alert_count = self.alerts.get_rules().len();
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "name": "OverDrive Remote",
            "gpu_count": gpu_count,
            "alert_rules": alert_count,
            "monitoring_active": self.monitoring.is_running(),
        })
    }
}
