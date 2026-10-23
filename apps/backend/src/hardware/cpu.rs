//! Real CPU telemetry via the `sysinfo` crate.
//!
//! Provides live, accurate readings for the Armoury-Crate-style dashboard:
//! CPU model/vendor, physical & logical core counts, per-core current
//! frequency (MHz), per-core & package utilization, and — where the OS
//! exposes a thermal sensor — package temperature.
//!
//! Notes on accuracy:
//! - **Frequency** is the real per-core *current* clock. On Windows it is read
//!   directly from Windows `CallNtPowerInformation(ProcessorInformation)`,
//!   which reports current MHz per logical processor. When unavailable, the
//!   platform-provided frequency is used without applying package-wide load
//!   estimates to individual cores.
//! - **Temperature** on Windows comes from a CPU-identified LibreHardwareMonitor
//!   package/die sensor; on other platforms it uses a labeled CPU component.
//!   Missing sensors are reported as `None` rather than a fabricated number.
//! - **Voltage** is not exposed by the OS without a kernel-mode sensor driver
//!   (e.g. LibreHardwareMonitor / WinRing0), so it is reported as `None`.

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sysinfo::{Components, System};

#[cfg(windows)]
use super::cpu_temperature::CpuTemperatureReader;
use super::cpu_voltage::VoltageReader;

/// Static CPU identity — queried once at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    pub model: String,
    pub vendor: String,
    pub physical_cores: usize,
    pub logical_cores: usize,
    pub is_elevated: bool,
}

/// Per-core live reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCoreSample {
    pub core_index: usize,
    pub frequency_mhz: u64,
    pub usage_percent: f32,
    pub temperature_celsius: Option<f64>,
}

/// Aggregate live reading for the whole package.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    pub timestamp: i64,
    /// Package frequency shown in the big gauge (MHz) — the busiest core's clock.
    pub frequency_mhz: u64,
    /// Gauge scale ceiling (MHz).
    pub max_frequency_mhz: u64,
    pub usage_percent: f32,
    pub temperature_celsius: Option<f64>,
    pub voltage_volts: Option<f64>,
    pub cores: Vec<CpuCoreSample>,
}

pub struct CpuMonitor {
    inner: Mutex<Inner>,
    info: CpuInfo,
    /// Rate-limited WMI voltage reader (updated every ~3s).
    voltage: VoltageReader,
    /// Rate-limited CPU sensor reader (updated every ~3s).
    #[cfg(windows)]
    temperature: CpuTemperatureReader,
}

struct Inner {
    sys: System,
    components: Components,
    /// Highest package frequency observed so far — used to derive a stable
    /// gauge scale when the OS does not report a turbo ceiling directly.
    max_freq_seen: u64,
}

impl Default for CpuMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuMonitor {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_all();

        let cpus = sys.cpus();
        let model = cpus
            .first()
            .map(|c| c.brand().trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "Unknown CPU".to_string());
        let vendor = cpus
            .first()
            .map(|c| c.vendor_id().trim().to_string())
            .unwrap_or_default();
        let logical_cores = cpus.len();
        let physical_cores = sys.physical_core_count().unwrap_or(logical_cores);

        tracing::info!(
            "CpuMonitor: {} ({} physical / {} logical cores)",
            model,
            physical_cores,
            logical_cores
        );

        let info = CpuInfo {
            model,
            vendor,
            physical_cores,
            logical_cores,
            is_elevated: check_elevation(),
        };

        Self {
            inner: Mutex::new(Inner {
                sys,
                components: new_components(),
                max_freq_seen: 0,
            }),
            info,
            voltage: VoltageReader::new(),
            #[cfg(windows)]
            temperature: CpuTemperatureReader::new(),
        }
    }

    /// Static CPU identity (cheap — cloned from cached values).
    pub fn info(&self) -> CpuInfo {
        self.info.clone()
    }

    /// Take a fresh live sample. Expected to be polled ~once per second; the
    /// interval between calls is what CPU usage percentages are computed over.
    pub fn sample(&self) -> CpuSample {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.sys.refresh_cpu_all();

        #[cfg(windows)]
        let native_frequencies = win_power::read_current_frequencies(inner.sys.cpus().len());
        #[cfg(not(windows))]
        let native_frequencies: Option<Vec<u64>> = None;

        #[cfg(windows)]
        let temperature_celsius = self.temperature.read();
        #[cfg(not(windows))]
        let temperature_celsius = read_cpu_temperature(&mut inner.components);

        let cores: Vec<CpuCoreSample> = inner
            .sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let frequency_mhz = native_frequencies
                    .as_ref()
                    .and_then(|frequencies| frequencies.get(i).copied())
                    .filter(|frequency| *frequency > 0)
                    .unwrap_or_else(|| c.frequency());
                CpuCoreSample {
                    core_index: i,
                    frequency_mhz,
                    usage_percent: c.cpu_usage(),
                    // The package sensor is not a per-core measurement.
                    temperature_celsius: None,
                }
            })
            .collect();

        // The busiest core's clock is the most representative "current speed"
        // for the package (this matches how Armoury Crate reports the big number).
        let package_freq = cores.iter().map(|c| c.frequency_mhz).max().unwrap_or(0);
        let usage_percent = inner.sys.global_cpu_usage();

        if package_freq > inner.max_freq_seen {
            inner.max_freq_seen = package_freq;
        }

        // Gauge scale: observed peak rounded up to the next 100 MHz with a
        // little headroom. Floored at 3000 MHz so the needle isn't pinned before
        // boost kicks in.
        let max_frequency_mhz = (((inner.max_freq_seen as f64) * 1.05) / 100.0).ceil() as u64 * 100;
        let max_frequency_mhz = max_frequency_mhz.max(3000);

        let voltage_volts = self.voltage.read().vcore;

        CpuSample {
            timestamp: chrono::Utc::now().timestamp_millis(),
            frequency_mhz: package_freq,
            max_frequency_mhz,
            usage_percent,
            temperature_celsius,
            voltage_volts,
            cores,
        }
    }
}

/// Find a labeled CPU/package temperature. Unlabeled thermal components may
/// belong to the GPU or another device, so they are never presented as CPU data.
fn read_cpu_temperature(components: &mut Components) -> Option<f64> {
    components.refresh_list();

    for component in components.iter() {
        let temp = component.temperature() as f64;
        if !temp.is_finite() || temp <= 0.0 {
            continue;
        }
        let label = component.label().to_lowercase();
        if label.contains("cpu")
            || label.contains("core")
            || label.contains("package")
            || label.contains("tctl")
            || label.contains("tdie")
        {
            return Some(temp);
        }
    }
    None
}

#[cfg(windows)]
fn new_components() -> Components {
    // Windows sysinfo components are generic ACPI zones, not CPU-identified sensors.
    Components::new()
}

#[cfg(not(windows))]
fn new_components() -> Components {
    Components::new_with_refreshed_list()
}

/// Native Windows per-logical-processor frequency query.
#[cfg(windows)]
pub mod win_power {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    #[allow(dead_code)]
    struct ProcessorPowerInformation {
        number: u32,
        max_mhz: u32,
        current_mhz: u32,
        mhz_limit: u32,
        max_idle_state: u32,
        current_idle_state: u32,
    }

    #[link(name = "powrprof")]
    unsafe extern "system" {
        fn CallNtPowerInformation(
            information_level: u32,
            input_buffer: *const std::ffi::c_void,
            input_buffer_length: u32,
            output_buffer: *mut std::ffi::c_void,
            output_buffer_length: u32,
        ) -> i32;
    }

    pub fn read_current_frequencies(processor_count: usize) -> Option<Vec<u64>> {
        if processor_count == 0 || processor_count > u32::MAX as usize {
            return None;
        }
        let mut information = vec![ProcessorPowerInformation::default(); processor_count];
        let output_size = std::mem::size_of_val(information.as_slice());
        let output_size = u32::try_from(output_size).ok()?;
        let status = unsafe {
            CallNtPowerInformation(
                11, // ProcessorInformation
                std::ptr::null(),
                0,
                information.as_mut_ptr().cast(),
                output_size,
            )
        };
        if status != 0 {
            return None;
        }
        Some(information.into_iter().map(|info| u64::from(info.current_mhz)).collect())
    }
}

#[cfg(not(windows))]
pub mod win_power {}

#[cfg(windows)]
fn check_elevation() -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("net")
        .arg("session")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn check_elevation() -> bool {
    false
}
