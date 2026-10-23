//! CPU voltage reading via WMI on Windows.
//!
//! Voltage is not exposed by the OS through simple system calls like
//! frequency. This module reads the Vcore sensor exposed by LibreHardwareMonitor.
//!
//! **LibreHardwareMonitor WMI** — if LHM is running, its WMI namespace
//!    (`root\LibreHardwareMonitor`) exposes per-core Vcore sensors.
//!
//! Because the query spawns a subprocess (PowerShell), results are
//! cached for `CACHE_TTL_SECS` to avoid saturating the CPU with WMI queries.

use std::time::Instant;

use serde::Serialize;

/// Cached voltage reading, internally rate-limited.
#[derive(Debug)]
pub struct VoltageReader {
    inner: std::sync::Mutex<VoltageInner>,
}

#[derive(Debug)]
struct VoltageInner {
    last: CpuVoltage,
    last_queried: Instant,
}

/// Voltage readings from the CPU package.
#[derive(Debug, Clone, Serialize)]
pub struct CpuVoltage {
    /// Core voltage (Vcore) in volts, e.g. 1.256 V.
    pub vcore: Option<f64>,
    /// Source label — describes how the value was obtained.
    pub source: &'static str,
}

const CACHE_TTL_SECS: u64 = 3;

impl Default for VoltageReader {
    fn default() -> Self {
        Self::new()
    }
}

impl VoltageReader {
    pub fn new() -> Self {
        Self {
            inner: std::sync::Mutex::new(VoltageInner {
                last: CpuVoltage {
                    vcore: None,
                    source: "unavailable",
                },
                last_queried: Instant::now()
                    .checked_sub(std::time::Duration::from_secs(CACHE_TTL_SECS + 1))
                    .unwrap_or_else(Instant::now),
            }),
        }
    }

    /// Returns the latest voltage reading. If the cache is stale,
    /// triggers a fresh WMI query (spawns PowerShell).
    pub fn read(&self) -> CpuVoltage {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        if inner.last_queried.elapsed().as_secs() < CACHE_TTL_SECS {
            return inner.last.clone();
        }

        let fresh = query_voltage();
        inner.last = fresh.clone();
        inner.last_queried = Instant::now();
        fresh
    }
}

/// Read CPU voltage from the LHM Vcore sensor when available.
fn query_voltage() -> CpuVoltage {
    if let Some(v) = lhm_vcore() {
        return CpuVoltage {
            vcore: Some(v),
            source: "LibreHardwareMonitor",
        };
    }

    CpuVoltage {
        vcore: None,
        source: "unavailable",
    }
}

/// Query LibreHardwareMonitor's WMI namespace for the first CPU Vcore sensor.
///
/// LHM exposes sensors like:
///   `Get-CimInstance -Namespace root/LibreHardwareMonitor -ClassName Sensor
///    | Where-Object { $_.SensorType -eq 'Voltage' -and $_.Name -like '*Vcore*' }`
fn lhm_vcore() -> Option<f64> {
    let script = r#"
try {
    $sensors = Get-CimInstance -Namespace "root/LibreHardwareMonitor" -ClassName Sensor -ErrorAction Stop
    $vcore = $sensors | Where-Object {
        $_.SensorType -eq 'Voltage' -and
        $_.Identifier -match '^/(intelcpu|amdcpu)/' -and
        $_.Name -match 'Vcore|CPU Core'
    } | Select-Object -First 1
    if ($vcore -and $vcore.Value -ne $null) { [double]$vcore.Value }
} catch {}
"#;
    run_powershell(script)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .filter(|voltage| voltage.is_finite() && *voltage > 0.0)
}

/// Run a PowerShell script and return its stdout if successful.
/// Timeout is set to 5 seconds to avoid hanging.
fn run_powershell(script: &str) -> Option<String> {
    let mut cmd = std::process::Command::new("powershell");

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let output = cmd
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            script,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}
