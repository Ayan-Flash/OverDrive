use std::fs;
use std::path::PathBuf;

use tracing::error;

use super::config::EnterpriseConfig;
use super::policy::GroupPolicy;

pub struct EnterpriseStore {
    config_path: PathBuf,
    policies_dir: PathBuf,
}

impl Default for EnterpriseStore {
    fn default() -> Self {
        Self::new()
    }
}

impl EnterpriseStore {
    pub fn new() -> Self {
        let base = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("enterprise");
        let config_path = base.join("config.json");
        let policies_dir = base.join("policies");
        if !policies_dir.exists() {
            let _ = fs::create_dir_all(&policies_dir);
        }
        Self {
            config_path,
            policies_dir,
        }
    }

    pub fn load_config(&self) -> EnterpriseConfig {
        if !self.config_path.exists() {
            return EnterpriseConfig::default();
        }
        match fs::read_to_string(&self.config_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                error!("Failed to read enterprise config: {e}");
                EnterpriseConfig::default()
            }
        }
    }

    pub fn save_config(&self, config: &EnterpriseConfig) {
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(config) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.config_path, &content) {
                    error!("Failed to write enterprise config: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize enterprise config: {e}");
            }
        }
    }

    pub fn list_policies(&self) -> Vec<String> {
        let mut policies = vec![];
        if let Ok(entries) = fs::read_dir(&self.policies_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        policies.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        policies.sort();
        policies
    }

    pub fn load_policy(&self, id: &str) -> Option<GroupPolicy> {
        let path = self.policies_dir.join(format!("{id}.json"));
        match fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(e) => {
                error!("Failed to read policy {id}: {e}");
                None
            }
        }
    }

    pub fn save_policy(&self, policy: &GroupPolicy) {
        let path = self.policies_dir.join(format!("{}.json", policy.id));
        match serde_json::to_string_pretty(policy) {
            Ok(content) => {
                if let Err(e) = fs::write(&path, &content) {
                    error!("Failed to save policy {}: {e}", policy.id);
                }
            }
            Err(e) => {
                error!("Failed to serialize policy {}: {e}", policy.id);
            }
        }
    }

    pub fn delete_policy(&self, id: &str) {
        let path = self.policies_dir.join(format!("{id}.json"));
        let _ = fs::remove_file(&path);
    }
}
