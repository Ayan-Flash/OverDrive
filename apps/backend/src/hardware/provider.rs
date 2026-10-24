use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl std::fmt::Display for GpuVendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuVendor::Nvidia => write!(f, "nvidia"),
            GpuVendor::Amd => write!(f, "amd"),
            GpuVendor::Intel => write!(f, "intel"),
            GpuVendor::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuIdentity {
    pub id: String,
    pub name: String,
    pub vendor: GpuVendor,
    pub index: u32,
    pub memory_total_mb: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuSample {
    pub gpu_id: String,
    pub timestamp: i64,
    pub temperature_celsius: Option<f64>,
    pub core_clock_mhz: Option<f64>,
    pub memory_clock_mhz: Option<f64>,
    pub memory_used_mb: Option<u64>,
    pub memory_total_mb: Option<u64>,
    pub fan_speed_percent: Option<f64>,
    pub fan_rpm: Option<u32>,
    pub power_watts: Option<f64>,
    /// Maximum graphics power limit exposed by the driver, when available.
    #[serde(default)]
    pub max_power_limit_watts: Option<f64>,
    pub core_voltage_mv: Option<f64>,
    pub core_utilization_percent: Option<f64>,
    pub memory_utilization_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuControlState {
    pub gpu_id: String,
    pub target_fan_speed: Option<f64>,
    pub core_clock_offset_mhz: i32,
    pub memory_clock_offset_mhz: i32,
    pub power_limit_percent: Option<f64>,
    pub voltage_offset_mv: i32,
}

pub trait GpuProvider: Send + Sync {
    fn name(&self) -> &str;
    fn enumerate(&self) -> Result<Vec<GpuIdentity>, GpuProviderError>;
    fn read_sample(&self, gpu_id: &str) -> Result<GpuSample, GpuProviderError>;
    fn read_control_state(&self, gpu_id: &str) -> Result<GpuControlState, GpuProviderError>;
    fn set_fan_speed(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError>;
    fn set_core_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError>;
    fn set_memory_clock_offset(
        &self,
        gpu_id: &str,
        offset_mhz: i32,
    ) -> Result<(), GpuProviderError>;
    fn set_power_limit(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError>;
    fn set_voltage_offset(&self, gpu_id: &str, offset_mv: i32) -> Result<(), GpuProviderError>;
}

#[derive(Debug, thiserror::Error)]
pub enum GpuProviderError {
    #[error("GPU not found: {0}")]
    GpuNotFound(String),
    #[error("Failed to read data: {0}")]
    ReadError(String),
    #[error("Failed to set control: {0}")]
    ControlError(String),
    #[error("Provider not available: {0}")]
    NotAvailable(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gpu_zero_is_distinct_from_unsupported_and_null_serializes() {
        let mut sample = GpuSample {
            gpu_id: "test".into(),
            timestamp: 0,
            temperature_celsius: Some(0.0),
            core_clock_mhz: None,
            memory_clock_mhz: None,
            memory_used_mb: None,
            memory_total_mb: None,
            fan_speed_percent: None,
            fan_rpm: None,
            power_watts: None,
            max_power_limit_watts: None,
            core_voltage_mv: None,
            core_utilization_percent: Some(0.0),
            memory_utilization_percent: None,
        };
        let value = serde_json::to_value(&sample).unwrap();
        assert_eq!(value["temperature_celsius"], 0.0);
        assert_eq!(value["core_clock_mhz"], serde_json::Value::Null);
        assert_eq!(value["power_watts"], serde_json::Value::Null);
        sample.power_watts = Some(0.0);
        assert_eq!(serde_json::to_value(&sample).unwrap()["power_watts"], 0.0);
    }
}
