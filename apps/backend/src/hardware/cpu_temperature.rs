//! CPU package temperature from CPU-specific LibreHardwareMonitor sensors.
//!
//! Windows ACPI thermal zones are intentionally not used here: their generic
//! labels do not establish that the reported value belongs to the CPU.

use serde::Deserialize;
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TemperatureSensor {
    identifier: String,
    name: String,
    value: Option<f64>,
}

#[derive(Debug)]
struct CachedTemperature {
    value: Option<f64>,
    queried_at: Option<Instant>,
}

/// Rate-limited CPU package temperature reader. Missing LHM sensors remain None.
#[derive(Debug)]
pub struct CpuTemperatureReader {
    cached: std::sync::Mutex<CachedTemperature>,
}

impl Default for CpuTemperatureReader {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuTemperatureReader {
    pub fn new() -> Self {
        Self {
            cached: std::sync::Mutex::new(CachedTemperature {
                value: None,
                queried_at: None,
            }),
        }
    }

    /// Read CPU package temperature in Celsius, refreshing at most every 3 seconds.
    pub fn read(&self) -> Option<f64> {
        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if cached.queried_at.is_some_and(|at| at.elapsed() < CACHE_TTL) {
            return cached.value;
        }

        let value = query_cpu_temperature();
        cached.value = value;
        cached.queried_at = Some(Instant::now());
        value
    }
}

#[cfg(windows)]
fn query_cpu_temperature() -> Option<f64> {
    let script = r#"
try {
    $sensors = @(Get-CimInstance -Namespace "root/LibreHardwareMonitor" -ClassName Sensor -ErrorAction Stop |
        Where-Object { $_.SensorType -eq 'Temperature' -and $_.Identifier -match '^/(intelcpu|amdcpu)/' } |
        Select-Object Identifier, Name, Value)
    ConvertTo-Json -InputObject $sensors -Compress -Depth 3
} catch { '[]' }
"#;
    let output = run_powershell(script)?;
    let sensors: Vec<TemperatureSensor> = serde_json::from_str(&output).ok()?;
    select_cpu_temperature(&sensors)
}

#[cfg(not(windows))]
fn query_cpu_temperature() -> Option<f64> {
    None
}

#[cfg(windows)]
fn run_powershell(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;
    use std::time::Instant;

    let mut child = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let timeout = Duration::from_secs(5);
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let output = child.wait_with_output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let stdout = stdout.trim();
    (!stdout.is_empty()).then(|| stdout.to_owned())
}

fn select_cpu_temperature(sensors: &[TemperatureSensor]) -> Option<f64> {
    let valid_cpu_sensors = sensors.iter().filter_map(|sensor| {
        let identifier = sensor.identifier.to_ascii_lowercase();
        if !(identifier.starts_with("/intelcpu/") || identifier.starts_with("/amdcpu/")) {
            return None;
        }
        let value = sensor
            .value
            .filter(|value| value.is_finite() && (0.0..=125.0).contains(value))?;
        Some((sensor.name.to_ascii_lowercase(), value))
    });

    let sensors: Vec<(String, f64)> = valid_cpu_sensors.collect();
    let rank = |name: &str| {
        if name.contains("package") {
            Some(0)
        } else if name.contains("tctl") || name.contains("tdie") {
            Some(1)
        } else if name.contains("die") && (name.contains("average") || name.contains("avg")) {
            Some(2)
        } else {
            None
        }
    };

    sensors
        .iter()
        .filter_map(|(name, value)| rank(name).map(|rank| (rank, *value)))
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, value)| value)
        .or_else(|| {
            sensors
                .iter()
                .filter(|(name, _)| name.contains("core"))
                .map(|(_, value)| *value)
                .max_by(f64::total_cmp)
        })
}

#[cfg(test)]
mod tests {
    use super::{select_cpu_temperature, TemperatureSensor};

    fn sensor(identifier: &str, name: &str, value: Option<f64>) -> TemperatureSensor {
        TemperatureSensor {
            identifier: identifier.into(),
            name: name.into(),
            value,
        }
    }

    #[test]
    fn prefers_cpu_package_over_core_temperature() {
        let sensors = [
            sensor("/intelcpu/0/temperature/2", "Core #1", Some(71.0)),
            sensor("/intelcpu/0/temperature/0", "CPU Package", Some(64.0)),
        ];
        assert_eq!(select_cpu_temperature(&sensors), Some(64.0));
    }

    #[test]
    fn accepts_amd_tctl_tdie_sensor() {
        let sensors = [sensor(
            "/amdcpu/0/temperature/0",
            "CPU (Tctl/Tdie)",
            Some(58.5),
        )];
        assert_eq!(select_cpu_temperature(&sensors), Some(58.5));
    }

    #[test]
    fn falls_back_to_hottest_cpu_core() {
        let sensors = [
            sensor("/amdcpu/0/temperature/1", "Core #1", Some(51.0)),
            sensor("/amdcpu/0/temperature/2", "Core #2", Some(57.0)),
        ];
        assert_eq!(select_cpu_temperature(&sensors), Some(57.0));
    }

    #[test]
    fn ignores_non_cpu_and_invalid_values() {
        let sensors = [
            sensor("/lpc/it8688e/temperature/0", "CPU", Some(80.0)),
            sensor("/intelcpu/0/temperature/1", "Core #1", Some(f64::NAN)),
            sensor("/intelcpu/0/temperature/2", "Core #2", Some(126.0)),
        ];
        assert_eq!(select_cpu_temperature(&sensors), None);
    }
}
