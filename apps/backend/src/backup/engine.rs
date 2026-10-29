use std::collections::HashMap;
use std::fs;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupScope {
    pub profiles: bool,
    pub alert_rules: bool,
    pub automation_rules: bool,
    pub integrations: bool,
    pub enterprise: bool,
    pub overlay: bool,
    pub remote: bool,
    pub tuning_profiles: bool,
    pub reports: bool,
}

impl Default for BackupScope {
    fn default() -> Self {
        Self {
            profiles: true,
            alert_rules: true,
            automation_rules: true,
            integrations: true,
            enterprise: true,
            overlay: true,
            remote: true,
            tuning_profiles: true,
            reports: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub file_size_bytes: u64,
    pub scope: BackupScope,
    pub gpu_count: usize,
    pub profile_count: usize,
    pub rule_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupResult {
    pub id: String,
    pub path: String,
    pub metadata: BackupMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub restored_profiles: usize,
    pub restored_rules: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupBundle {
    pub id: String,
    pub name: String,
    pub created_at: i64,
    pub version: String,
    pub scope: BackupScope,
    pub profiles: Option<Value>,
    pub alert_rules: Option<Value>,
    pub automation_rules: Option<Value>,
    pub integrations: Option<Value>,
    pub enterprise: Option<Value>,
    pub overlay: Option<Value>,
    pub remote: Option<Value>,
    pub tuning_profiles: Option<Value>,
    pub reports_data: Option<Value>,
}

pub struct BackupEngine;

impl BackupEngine {
    pub fn create_backup(name: String, scope: BackupScope) -> Result<BackupResult, String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        let mut bundle = BackupBundle {
            id: id.clone(),
            name,
            created_at: now,
            version: env!("CARGO_PKG_VERSION").into(),
            scope: scope.clone(),
            profiles: None,
            alert_rules: None,
            automation_rules: None,
            integrations: None,
            enterprise: None,
            overlay: None,
            remote: None,
            tuning_profiles: None,
            reports_data: None,
        };

        let mut profile_count = 0;
        let mut rule_count = 0;

        if scope.profiles {
            let dir = dirs_next::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("overdrive")
                .join("profiles");
            let mut profiles_map = HashMap::new();
            if dir.exists() {
                if let Ok(entries) = fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            if name.ends_with(".json") {
                                if let Ok(content) = fs::read_to_string(entry.path()) {
                                    let key = name.trim_end_matches(".json").to_string();
                                    let val = serde_json::from_str::<Value>(&content).ok();
                                    profiles_map.insert(key, val);
                                    profile_count += 1;
                                }
                            }
                        }
                    }
                }
            }
            bundle.profiles = Some(serde_json::to_value(profiles_map).unwrap_or_default());
        }

        if scope.automation_rules {
            let store = crate::automation::store::RuleStore::new();
            let rules = store.load();
            bundle.automation_rules = Some(serde_json::to_value(&rules).unwrap_or_default());
            rule_count += rules.len();
        }

        if scope.alert_rules {
            let store_path = dirs_next::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("overdrive")
                .join("alert_rules.json");
            if let Ok(content) = fs::read_to_string(&store_path) {
                bundle.alert_rules = serde_json::from_str::<Value>(&content).ok();
            }
        }

        if scope.integrations {
            let store = crate::integrations::IntegrationStore::new();
            let config = store.load();
            bundle.integrations = Some(serde_json::to_value(&config).unwrap_or_default());
        }

        if scope.enterprise {
            let store = crate::enterprise::store::EnterpriseStore::new();
            let config = store.load_config();
            bundle.enterprise = Some(serde_json::to_value(&config).unwrap_or_default());
        }

        if scope.tuning_profiles {
            let _tuner_store = crate::ai::tuner::TuningStore::new();
            let path = dirs_next::config_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join("overdrive")
                .join("tuning_profiles.json");
            if let Ok(content) = fs::read_to_string(&path) {
                bundle.tuning_profiles = serde_json::from_str::<Value>(&content).ok();
            }
        }

        let json = serde_json::to_string_pretty(&bundle)
            .map_err(|e| format!("Failed to serialize backup: {e}"))?;
        let file_size = json.len() as u64;

        let store = super::store::BackupStore::new();
        store.write_backup(&id, &json);

        let gpu_count = 1;
        let metadata = BackupMetadata {
            id: id.clone(),
            name: bundle.name.clone(),
            created_at: now,
            file_size_bytes: file_size,
            scope,
            gpu_count,
            profile_count,
            rule_count,
        };

        let mut all_meta = store.list_metadata();
        all_meta.push(metadata.clone());
        store.save_metadata(&all_meta);

        info!("Backup created: {} ({})", bundle.name, id);

        let path_str = store.backup_path(&id).to_string_lossy().into();
        Ok(BackupResult {
            id,
            path: path_str,
            metadata,
        })
    }

    pub fn restore_backup(id: &str, scope: BackupScope) -> Result<RestoreResult, String> {
        let store = super::store::BackupStore::new();
        let content = store
            .read_backup(id)
            .ok_or_else(|| format!("Backup {id} not found"))?;

        let bundle: BackupBundle = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid backup file: {e}"))?;

        let mut restored_profiles = 0;
        let mut restored_rules = 0;
        let mut errors = vec![];

        if scope.profiles {
            if let Some(profiles_val) = &bundle.profiles {
                if let Some(map) = profiles_val.as_object() {
                    let profiles_dir = dirs_next::config_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("overdrive")
                        .join("profiles");
                    if !profiles_dir.exists() {
                        let _ = fs::create_dir_all(&profiles_dir);
                    }
                    for (name, val) in map {
                        let path = profiles_dir.join(format!("{name}.json"));
                        if let Ok(content) = serde_json::to_string_pretty(val) {
                            if fs::write(&path, &content).is_ok() {
                                restored_profiles += 1;
                            } else {
                                errors.push(format!("Failed to write profile {name}"));
                            }
                        }
                    }
                }
            }
        }

        if scope.automation_rules {
            if let Some(rules_val) = &bundle.automation_rules {
                let store = crate::automation::RuleStore::new();
                if let Ok(rules) = serde_json::from_value::<Vec<crate::automation::Rule>>(rules_val.clone()) {
                    store.save(&rules);
                    restored_rules = rules.len();
                }
            }
        }

        if scope.integrations {
            if let Some(val) = &bundle.integrations {
                if let Ok(config) = serde_json::from_value::<crate::integrations::IntegrationConfig>(val.clone()) {
                    let istore = crate::integrations::IntegrationStore::new();
                    istore.save(&config);
                }
            }
        }

        if scope.enterprise {
            if let Some(val) = &bundle.enterprise {
                if let Ok(config) = serde_json::from_value::<crate::enterprise::EnterpriseConfig>(val.clone()) {
                    let estore = crate::enterprise::store::EnterpriseStore::new();
                    estore.save_config(&config);
                }
            }
        }

        info!("Backup restored: {} ({} profiles, {} rules)", id, restored_profiles, restored_rules);

        Ok(RestoreResult {
            success: errors.is_empty(),
            restored_profiles,
            restored_rules,
            errors,
        })
    }

    pub fn list_backups() -> Vec<BackupMetadata> {
        let store = super::store::BackupStore::new();
        store.list_metadata()
    }

    pub fn delete_backup(id: &str) {
        let store = super::store::BackupStore::new();
        store.delete_backup(id);

        let mut all = store.list_metadata();
        all.retain(|m| m.id != id);
        store.save_metadata(&all);
    }

    pub fn export_backup_path(id: &str) -> Result<String, String> {
        let store = super::store::BackupStore::new();
        let path = store.backup_path(id);
        if path.exists() {
            Ok(path.to_string_lossy().to_string())
        } else {
            Err("Backup file not found".into())
        }
    }

    pub fn import_backup(file_path: &str) -> Result<BackupMetadata, String> {
        let store = super::store::BackupStore::new();
        let id = store.import_backup(file_path)?;
        let content = store.read_backup(&id).ok_or("Failed to read imported backup")?;
        let bundle: BackupBundle = serde_json::from_str(&content)
            .map_err(|e| format!("Invalid backup: {e}"))?;

        let metadata = BackupMetadata {
            id: id.clone(),
            name: bundle.name,
            created_at: bundle.created_at,
            file_size_bytes: content.len() as u64,
            scope: bundle.scope,
            gpu_count: 0,
            profile_count: 0,
            rule_count: 0,
        };

        let mut all = store.list_metadata();
        all.push(metadata.clone());
        store.save_metadata(&all);

        Ok(metadata)
    }
}
