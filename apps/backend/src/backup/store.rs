use std::fs;
use std::path::PathBuf;

use tracing::error;

use super::engine::BackupMetadata;

pub struct BackupStore {
    path: PathBuf,
}

impl Default for BackupStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BackupStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("backups");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        Self { path }
    }

    pub fn backups_dir(&self) -> &PathBuf {
        &self.path
    }

    pub fn backup_path(&self, id: &str) -> PathBuf {
        self.path.join(format!("{id}.gpubackup"))
    }

    pub fn list_metadata(&self) -> Vec<BackupMetadata> {
        let index_path = self.path.join("index.json");
        if !index_path.exists() {
            return vec![];
        }
        match fs::read_to_string(&index_path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
            Err(e) => {
                error!("Failed to read backup index: {e}");
                vec![]
            }
        }
    }

    pub fn save_metadata(&self, metadata: &[BackupMetadata]) {
        let index_path = self.path.join("index.json");
        match serde_json::to_string_pretty(metadata) {
            Ok(content) => {
                if let Err(e) = fs::write(&index_path, &content) {
                    error!("Failed to write backup index: {e}");
                }
            }
            Err(e) => {
                error!("Failed to serialize backup index: {e}");
            }
        }
    }

    pub fn write_backup(&self, id: &str, content: &str) {
        let path = self.backup_path(id);
        if let Err(e) = fs::write(&path, content) {
            error!("Failed to write backup file {id}: {e}");
        }
    }

    pub fn read_backup(&self, id: &str) -> Option<String> {
        let path = self.backup_path(id);
        fs::read_to_string(&path).ok()
    }

    pub fn delete_backup(&self, id: &str) {
        let path = self.backup_path(id);
        let _ = fs::remove_file(&path);
    }

    pub fn import_backup(&self, source_path: &str) -> Result<String, String> {
        let content = fs::read_to_string(source_path)
            .map_err(|e| format!("Cannot read file: {e}"))?;
        let bundle: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid backup file: {e}"))?;
        let id = bundle.get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("imported")
            .to_string();
        self.write_backup(&id, &content);
        Ok(id)
    }
}
