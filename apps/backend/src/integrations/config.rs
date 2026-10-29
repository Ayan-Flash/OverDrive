use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub discord_webhook_url: String,
    pub discord_username: String,
    pub discord_avatar_url: String,
    pub discord_notify_on_alert: bool,
    pub discord_notify_on_high_temp: bool,
    pub discord_high_temp_threshold: f64,
    pub obs_enabled: bool,
    pub obs_port: u16,
    pub obs_refresh_rate_ms: u64,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            discord_webhook_url: String::new(),
            discord_username: "OverDrive".into(),
            discord_avatar_url: String::new(),
            discord_notify_on_alert: true,
            discord_notify_on_high_temp: true,
            discord_high_temp_threshold: 80.0,
            obs_enabled: false,
            obs_port: 9877,
            obs_refresh_rate_ms: 1000,
        }
    }
}

pub struct IntegrationStore {
    path: PathBuf,
}

impl Default for IntegrationStore {
    fn default() -> Self {
        Self::new()
    }
}

impl IntegrationStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("integrations.json");
        Self { path }
    }

    pub fn load(&self) -> IntegrationConfig {
        if !self.path.exists() {
            return IntegrationConfig::default();
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                error!("Failed to read integration config: {e}");
                IntegrationConfig::default()
            }
        }
    }

    pub fn save(&self, config: &IntegrationConfig) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(config) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write integration config: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize integration config: {e}");
            }
        }
    }
}
