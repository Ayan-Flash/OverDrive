use std::fs;
use std::path::PathBuf;

use tracing::error;

use super::Report;

pub struct ReportStore {
    path: PathBuf,
}

impl Default for ReportStore {
    fn default() -> Self {
        Self::new()
    }
}

impl ReportStore {
    pub fn new() -> Self {
        let path = dirs_next::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("overdrive")
            .join("reports");
        if !path.exists() {
            let _ = fs::create_dir_all(&path);
        }
        Self { path }
    }

    pub fn list_reports(&self) -> Vec<String> {
        let mut reports = vec![];
        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".json") {
                        reports.push(name.trim_end_matches(".json").to_string());
                    }
                }
            }
        }
        reports.sort();
        reports.reverse();
        reports
    }

    pub fn load_report(&self, id: &str) -> Option<Report> {
        let file_path = self.path.join(format!("{id}.json"));
        match fs::read_to_string(&file_path) {
            Ok(content) => serde_json::from_str(&content).ok(),
            Err(e) => {
                error!("Failed to read report {id}: {e}");
                None
            }
        }
    }

    pub fn save_report(&self, report: &Report) {
        let file_path = self.path.join(format!("{}.json", report.id));
        match serde_json::to_string_pretty(report) {
            Ok(content) => {
                if let Err(e) = fs::write(&file_path, &content) {
                    error!("Failed to save report {}: {e}", report.id);
                }
            }
            Err(e) => {
                error!("Failed to serialize report {}: {e}", report.id);
            }
        }
    }

    pub fn delete_report(&self, id: &str) {
        let file_path = self.path.join(format!("{id}.json"));
        let _ = fs::remove_file(&file_path);
    }

    pub fn get_report_path(&self, id: &str) -> PathBuf {
        self.path.join(format!("{id}.json"))
    }
}
