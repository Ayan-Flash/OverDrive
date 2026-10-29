use std::fs;
use std::path::PathBuf;

use tracing::error;

use super::engine::MarketplaceProfile;

pub struct MarketplaceStore {
    path: PathBuf,
}

impl Default for MarketplaceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MarketplaceStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("marketplace")
            .join("registry.json");
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                let _ = fs::create_dir_all(parent);
            }
        }
        Self { path }
    }

    pub fn load_all(&self) -> Vec<MarketplaceProfile> {
        if !self.path.exists() {
            return vec![];
        }
        match fs::read_to_string(&self.path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                error!("Failed to parse marketplace registry: {e}");
                vec![]
            }),
            Err(e) => {
                error!("Failed to read marketplace registry: {e}");
                vec![]
            }
        }
    }

    pub fn save_all(&self, profiles: &[MarketplaceProfile]) {
        match serde_json::to_string_pretty(profiles) {
            Ok(content) => {
                if let Err(e) = fs::write(&self.path, &content) {
                    error!("Failed to write marketplace registry: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize marketplace registry: {e}");
            }
        }
    }
}
