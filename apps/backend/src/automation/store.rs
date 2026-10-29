use std::fs;
use std::path::PathBuf;

use tracing::error;

use super::rule::Rule;

pub struct RuleStore {
    path: PathBuf,
}

impl Default for RuleStore {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("automation_rules.json");
        Self { path }
    }

    pub fn load(&self) -> Vec<Rule> {
        if !self.path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => {
                serde_json::from_str(&content).unwrap_or_else(|e| {
                    error!("Failed to parse automation rules: {e}");
                    Vec::new()
                })
            }
            Err(e) => {
                error!("Failed to read automation rules: {e}");
                Vec::new()
            }
        }
    }

    pub fn save(&self, rules: &[Rule]) {
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        match serde_json::to_string_pretty(rules) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write automation rules: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize automation rules: {e}");
            }
        }
    }
}
