//! Observes the installer-managed CPU sensor service. The UI never elevates
//! itself or starts an elevated helper; only installation needs administrator access.
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

#[derive(Clone, Default)]
pub struct SensorRuntime;

impl SensorRuntime {
    pub fn start_background(&self, _app: AppHandle) {
        thread::spawn(|| {
            let Ok(client) = reqwest::blocking::Client::builder()
                .timeout(Duration::from_millis(500)).build() else { return; };
            // Allow the automatic service a short startup window after boot.
            for _ in 0..20 {
                if let Ok(response) = client.get("http://127.0.0.1:8085/data.json").send() {
                    if let Ok(feed) = response.error_for_status().and_then(|r| r.json::<serde_json::Value>()) {
                        if feed.get("Children").is_some() {
                            tracing::info!("CPU sensor service is available; no elevation required for the UI");
                            if feed.get("DriverInstalled").and_then(|v| v.as_bool()) == Some(false) {
                                tracing::warn!("PawnIO driver missing; repair the OverDrive installation");
                            }
                            return;
                        }
                    }
                }
                thread::sleep(Duration::from_millis(500));
            }
            tracing::warn!("CPU sensor service unavailable. Repair the OverDrive installation. Native CPU usage remains available.");
        });
    }

    /// The system service outlives the UI and is removed by the uninstaller.
    pub fn stop(&self) {}
}
