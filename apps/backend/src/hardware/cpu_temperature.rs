//! CPU package temperature from CPU-specific LibreHardwareMonitor sensors.
//!
//! Windows ACPI thermal zones are intentionally not used here: their generic
//! labels do not establish that the reported value belongs to the CPU.

use serde::Deserialize;
use std::time::{Duration, Instant};

const CACHE_TTL: Duration = Duration::from_secs(1);
const CACHE_UNAVAILABLE_TTL: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct TemperatureSensor {
    identifier: String,
    name: String,
    value: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuCoreClock {
    pub name: String,
    pub frequency_mhz: f64,
}

#[derive(Debug, Clone, Default)]
pub struct CpuTemperatureReadings {
    pub package_celsius: Option<f64>,
    pub core_temperatures: Vec<CpuCoreTemperature>,
    pub fastest_core_clock_mhz: Option<f64>,
    pub core_clocks: Vec<CpuCoreClock>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CpuCoreTemperature {
    pub name: String,
    pub temperature_celsius: f64,
}

/// JSON node returned by LibreHardwareMonitor's local `/data.json` endpoint.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct LhmNode {
    text: Option<String>,
    sensor_id: Option<String>,
    // LibreHardwareMonitor's data.json uses "Type"; the bundled headless
    // host emits "SensorType". Accept both so a live feed is not discarded.
    #[serde(alias = "Type")]
    sensor_type: Option<String>,
    raw_value: Option<serde_json::Value>,
    value: Option<serde_json::Value>,
    #[serde(default)]
    children: Vec<LhmNode>,
}

#[derive(Debug)]
struct CachedTemperature {
    value: CpuTemperatureReadings,
    queried_at: Option<Instant>,
}

/// Rate-limited CPU package temperature reader. Missing provider sensors remain None.
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
                value: CpuTemperatureReadings::default(),
                queried_at: None,
            }),
        }
    }

    /// Read CPU sensors once per second when available (5s if unavailable).
    pub fn read(&self) -> CpuTemperatureReadings {
        let mut cached = self
            .cached
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let ttl = if cached.value.package_celsius.is_some()
            || !cached.value.core_temperatures.is_empty()
        {
            CACHE_TTL
        } else {
            CACHE_UNAVAILABLE_TTL
        };
        if cached.queried_at.is_some_and(|at| at.elapsed() < ttl) {
            return cached.value.clone();
        }

        let value = query_cpu_temperatures();
        cached.value = value.clone();
        cached.queried_at = Some(Instant::now());
        value
    }
}

#[cfg(windows)]
fn query_cpu_temperatures() -> CpuTemperatureReadings {
    // LibreHardwareMonitor 0.9.5+ removed its WMI provider. Its supported
    // local JSON endpoint remains available when Remote Web Server is enabled.
    if let Some(readings) = query_lhm_http_temperatures() {
        return readings;
    }

    let script = r#"
try {
    $namespaces = @('root/LibreHardwareMonitor', 'root/OpenHardwareMonitor')
    foreach ($ns in $namespaces) {
        $sensors = @(Get-CimInstance -Namespace $ns -ClassName Sensor -ErrorAction SilentlyContinue |
            Where-Object { $_.SensorType -eq 'Temperature' -and $_.Identifier -match '^/(intelcpu|amdcpu)/' } |
            Select-Object Identifier, Name, Value)
        if ($sensors.Count -gt 0) {
            ConvertTo-Json -InputObject $sensors -Compress -Depth 3
            return
        }
    }
    '[]'
} catch { '[]' }
"#;
    let Some(output) = run_powershell(script) else {
        return CpuTemperatureReadings::default();
    };
    let Ok(sensors) = serde_json::from_str::<Vec<TemperatureSensor>>(&output) else {
        return CpuTemperatureReadings::default();
    };
    select_cpu_readings(&sensors)
}

#[cfg(windows)]
fn query_lhm_http_temperatures() -> Option<CpuTemperatureReadings> {
    use std::time::Duration;

    // Never accept a configurable/remote host here: sensor access stays local.
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .ok()?;
    let response = client
        .get("http://127.0.0.1:8085/data.json")
        .send()
        .ok()?
        .error_for_status()
        .ok()?;
    let root: LhmNode = response.json().ok()?;
    let mut sensors = Vec::new();
    collect_lhm_temperature_sensors(&root, &mut sensors);
    Some(select_cpu_readings(&sensors))
}

#[cfg(not(windows))]
fn query_lhm_http_temperatures() -> Option<CpuTemperatureReadings> {
    None
}

#[cfg(not(windows))]
fn query_cpu_temperatures() -> CpuTemperatureReadings {
    CpuTemperatureReadings::default()
}

fn parse_lhm_number(val: Option<&serde_json::Value>) -> Option<f64> {
    let val = val?;
    match val {
        serde_json::Value::Number(num) => num.as_f64(),
        serde_json::Value::String(text) => text
            .split_whitespace()
            .next()
            .and_then(|num| num.replace(',', ".").parse().ok()),
        _ => None,
    }
}

fn collect_lhm_temperature_sensors(node: &LhmNode, sensors: &mut Vec<TemperatureSensor>) {
    if node.sensor_type.as_deref().is_some_and(|sensor_type| {
        sensor_type.eq_ignore_ascii_case("Temperature")
            || sensor_type.eq_ignore_ascii_case("Clock")
    }) {
        if let Some(identifier) = node.sensor_id.as_ref() {
            sensors.push(TemperatureSensor {
                identifier: identifier.clone(),
                name: node.text.clone().unwrap_or_default(),
                value: parse_lhm_number(node.raw_value.as_ref())
                    .or_else(|| parse_lhm_number(node.value.as_ref())),
            });
        }
    }

    for child in &node.children {
        collect_lhm_temperature_sensors(child, sensors);
    }
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
    let timeout = Duration::from_millis(1500);
    let started = Instant::now();
    loop {
        match child.try_wait().ok()? {
            Some(_) => break,
            None if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            None => std::thread::sleep(Duration::from_millis(25)),
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
        if !identifier.contains("/temperature/") {
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

fn select_cpu_readings(sensors: &[TemperatureSensor]) -> CpuTemperatureReadings {
    let mut core_temperatures = Vec::new();
    let mut core_clocks = Vec::new();

    for sensor in sensors {
        let identifier = sensor.identifier.to_ascii_lowercase();
        if !(identifier.starts_with("/intelcpu/") || identifier.starts_with("/amdcpu/")) {
            continue;
        }

        // Temperature sensors
        if identifier.contains("/temperature/") {
            let Some(value) = sensor
                .value
                .filter(|value| value.is_finite() && (0.0..=125.0).contains(value))
            else {
                continue;
            };
            if is_per_core_sensor(&sensor.name) {
                core_temperatures.push(CpuCoreTemperature {
                    name: sensor.name.clone(),
                    temperature_celsius: value,
                });
            }
        }

        // Clock sensors
        if identifier.contains("/clock/") {
            let Some(value) = sensor
                .value
                .filter(|value| value.is_finite() && (100.0..=10_000.0).contains(value))
            else {
                continue;
            };
            let name_lower = sensor.name.to_ascii_lowercase();
            if is_per_core_sensor(&sensor.name)
                && !name_lower.contains("effective")
                && !name_lower.contains("average")
            {
                core_clocks.push(CpuCoreClock {
                    name: sensor.name.clone(),
                    frequency_mhz: value,
                });
            }
        }
    }

    core_temperatures.sort_by(|left, right| {
        core_number(&left.name)
            .cmp(&core_number(&right.name))
            .then_with(|| left.name.cmp(&right.name))
    });

    core_clocks.sort_by(|left, right| {
        core_number(&left.name)
            .cmp(&core_number(&right.name))
            .then_with(|| left.name.cmp(&right.name))
    });

    let fastest_core_clock_mhz = core_clocks
        .iter()
        .map(|c| c.frequency_mhz)
        .max_by(f64::total_cmp)
        .or_else(|| {
            sensors
                .iter()
                .filter(|sensor| {
                    let identifier = sensor.identifier.to_ascii_lowercase();
                    (identifier.starts_with("/intelcpu/") || identifier.starts_with("/amdcpu/"))
                        && identifier.contains("/clock/")
                })
                .filter_map(|sensor| sensor.value)
                .filter(|value| value.is_finite() && (100.0..=10_000.0).contains(value))
                .max_by(f64::total_cmp)
        });

    CpuTemperatureReadings {
        package_celsius: select_cpu_temperature(sensors),
        core_temperatures,
        fastest_core_clock_mhz,
        core_clocks,
    }
}

fn is_per_core_sensor(name: &str) -> bool {
    core_number(name).is_some()
}

fn core_number(name: &str) -> Option<usize> {
    let lowercase = name.to_ascii_lowercase();
    let core = lowercase.rfind("core")?;
    let suffix = lowercase[core + "core".len()..].trim_start();
    let digits = suffix
        .trim_start_matches(['#', ':'])
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>();
    if digits.is_empty() {
        return None;
    }

    let number = digits.parse::<usize>().ok()?;
    // Sensor labels with an explicit # use the conventional 1-based numbering;
    // labels without it may be zero-based, so preserve their number as given.
    if suffix.starts_with('#') {
        Some(number.saturating_sub(1))
    } else {
        Some(number)
    }
}

#[cfg(test)]
mod tests {
    use super::{select_cpu_readings, select_cpu_temperature, TemperatureSensor};

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

    #[test]
    fn deserializes_lhm_live_json_with_string_raw_value() {
        let json_data = r#"{
            "Text": "Sensor",
            "Children": [
                {
                    "Text": "AMD Ryzen 7 5700G",
                    "Children": [
                        {
                            "Text": "Core (Tctl/Tdie)",
                            "SensorId": "/amdcpu/0/temperature/2",
                            "Type": "Temperature",
                            "RawValue": "58.8 °C",
                            "Value": "58.8 °C"
                        },
                        {
                            "Text": "Core #1",
                            "SensorId": "/amdcpu/0/clock/3",
                            "Type": "Clock",
                            "RawValue": "4467.0 MHz",
                            "Value": "4467.0 MHz"
                        },
                        {
                            "Text": "Core #2",
                            "SensorId": "/amdcpu/0/clock/5",
                            "Type": "Clock",
                            "RawValue": "4467.0 MHz",
                            "Value": "4467.0 MHz"
                        }
                    ]
                }
            ]
        }"#;

        let root: super::LhmNode = serde_json::from_str(json_data).expect("Should deserialize LHM node");
        let mut sensors = Vec::new();
        super::collect_lhm_temperature_sensors(&root, &mut sensors);
        let readings = select_cpu_readings(&sensors);

        assert_eq!(readings.package_celsius, Some(58.8));
        assert_eq!(readings.fastest_core_clock_mhz, Some(4467.0));
        assert_eq!(readings.core_clocks.len(), 2);
    }

    #[test]
    fn deserializes_cpu_sensor_host_numeric_raw_value() {
        let json_data = r#"{
            "Text": "Computer",
            "Children": [
                {
                    "Text": "AMD Ryzen 7 5700G",
                    "Children": [
                        {
                            "Text": "Core (Tctl/Tdie)",
                            "SensorId": "/amdcpu/0/temperature/2",
                            "SensorType": "Temperature",
                            "RawValue": 58.8,
                            "Value": "58.8"
                        },
                        {
                            "Text": "Core #1",
                            "SensorId": "/amdcpu/0/clock/3",
                            "SensorType": "Clock",
                            "RawValue": 4467.0,
                            "Value": "4467.0"
                        }
                    ]
                }
            ]
        }"#;

        let root: super::LhmNode = serde_json::from_str(json_data).expect("Should deserialize host node");
        let mut sensors = Vec::new();
        super::collect_lhm_temperature_sensors(&root, &mut sensors);
        let readings = select_cpu_readings(&sensors);

        assert_eq!(readings.package_celsius, Some(58.8));
        assert_eq!(readings.fastest_core_clock_mhz, Some(4467.0));
    }
}
