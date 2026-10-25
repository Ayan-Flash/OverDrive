//! Starts the bundled LibreHardwareMonitor sensor feed for live CPU telemetry.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

const RUNTIME_FOLDER: &str = "LibreHardwareMonitor";
const PROVIDER_EXECUTABLE: &str = "CpuSensorHost.exe";
const SENSOR_URL: &str = "http://127.0.0.1:8085/data.json";

#[derive(Clone, Default)]
pub struct SensorRuntime {
    child: Arc<Mutex<Option<Child>>>,
}

impl SensorRuntime {
    /// Start the private sensor provider without delaying window creation.
    pub fn start_background(&self, app: AppHandle) {
        let runtime = self.clone();
        thread::spawn(move || runtime.ensure_running(&app));
    }

    /// Stop only the provider process started by this app.
    pub fn stop(&self) {
        let mut child = self.child.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(mut process) = child.take() {
            if process.try_wait().ok().flatten().is_none() {
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }

    fn ensure_running(&self, app: &AppHandle) {
        if sensor_endpoint_ready() {
            tracing::info!("Using an existing local LibreHardwareMonitor sensor feed");
            return;
        }

        let runtime_dir = match prepare_runtime(app) {
            Ok(path) => path,
            Err(error) => {
                tracing::warn!(error = %error, "Could not prepare the bundled CPU sensor provider");
                return;
            }
        };
        let executable = runtime_dir.join(PROVIDER_EXECUTABLE);
        let mut child = match Command::new(&executable)
            .current_dir(&runtime_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                tracing::warn!(path = %executable.display(), error = %error, "Could not start the bundled CPU sensor provider");
                return;
            }
        };

        // Give the hardware monitor a short window to initialize its sensors
        // and bind the loopback-only JSON endpoint.
        for _ in 0..30 {
            if sensor_endpoint_ready() {
                tracing::info!("Bundled headless CPU sensor feed is ready");
                *self.child.lock().unwrap_or_else(|error| error.into_inner()) = Some(child);
                return;
            }
            if child.try_wait().ok().flatten().is_some() {
                tracing::warn!("Bundled headless CPU sensor provider exited before its feed became ready");
                return;
            }
            thread::sleep(Duration::from_millis(500));
        }

        tracing::warn!("Bundled headless CPU sensor provider did not start its feed within 15 seconds");
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn prepare_runtime(app: &AppHandle) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    // On Windows Tauri's resource_dir() is the directory containing the main
    // executable. Bundled resources retain their configured relative path, so
    // `resources/LibreHardwareMonitor/` is nested below that directory.
    // Keep the direct child candidate for development layouts that place
    // resources beside the executable.
    let resource_dir = app.path().resource_dir()?;
    let bundled_dir = [
        resource_dir.join("resources").join(RUNTIME_FOLDER),
        resource_dir.join(RUNTIME_FOLDER),
    ]
    .into_iter()
    .find(|candidate| candidate.join(PROVIDER_EXECUTABLE).is_file())
    .ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!(
                "{PROVIDER_EXECUTABLE} was not found under {} (checked resources/{RUNTIME_FOLDER} and {RUNTIME_FOLDER})",
                resource_dir.display()
            ),
        )
    })?;
    let runtime_dir = app
        .path()
        .app_data_dir()?
        .join("sensor-runtime")
        .join(RUNTIME_FOLDER);
    fs::create_dir_all(&runtime_dir)?;

    copy_runtime_files(&bundled_dir, &runtime_dir)?;
    Ok(runtime_dir)
}

fn copy_runtime_files(source: &Path, destination: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        if source_path.is_file() {
            let file_name = entry.file_name();
            // Older app versions launched this GUI binary directly. It may
            // still be running during an in-place app update, so leave it in
            // place and copy only the headless provider and shared libraries.
            if file_name == "LibreHardwareMonitor.exe"
                || file_name == "LibreHardwareMonitor.exe.config"
            {
                continue;
            }
            fs::copy(source_path, destination.join(file_name))?;
        }
    }
    Ok(())
}

fn sensor_endpoint_ready() -> bool {
    let Ok(client) = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(350))
        .build()
    else {
        return false;
    };
    let Ok(response) = client.get(SENSOR_URL).send() else {
        return false;
    };
    response
        .error_for_status()
        .ok()
        .and_then(|response| response.json::<serde_json::Value>().ok())
        .is_some_and(|value| value.get("Children").is_some())
}
