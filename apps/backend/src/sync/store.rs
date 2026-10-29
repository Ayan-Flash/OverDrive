use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncState {
    pub device_id: String,
    pub server_url: String,
    pub api_key: String,
    pub registered: bool,
    pub last_sync_at: Option<i64>,
    pub enabled: bool,
    pub sync_interval_secs: u64,
    pub sync_profiles: bool,
    pub sync_reports: bool,
    pub sync_policies: bool,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            server_url: String::new(),
            api_key: String::new(),
            registered: false,
            last_sync_at: None,
            enabled: false,
            sync_interval_secs: 300,
            sync_profiles: true,
            sync_reports: false,
            sync_policies: true,
        }
    }
}

pub struct SyncStore {
    path: PathBuf,
}

impl Default for SyncStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("sync_state.json");
        Self { path }
    }

    pub fn load(&self) -> SyncState {
        if !self.path.exists() {
            return SyncState::default();
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                error!("Failed to read sync state: {e}");
                SyncState::default()
            }
        }
    }

    pub fn save(&self, state: &SyncState) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(state) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write sync state: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize sync state: {e}");
            }
        }
    }
}
