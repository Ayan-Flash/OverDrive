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

use super::cpu_temperature::CpuCoreTemperature;
#[cfg(windows)]
use super::cpu_temperature::CpuTemperatureReader;
#[cfg(not(windows))]
use super::cpu_temperature::CpuTemperatureReadings;
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
    /// Individual labeled physical CPU core temperature sensors, when exposed.
    pub core_temperatures: Vec<CpuCoreTemperature>,
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
    #[cfg(windows)]
    win_usage: win_times::SystemCpuUsage,
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
                #[cfg(windows)]
                win_usage: win_times::SystemCpuUsage::new(),
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
        // Read external rate-limited sensors outside the inner mutex lock so
        // sample() never blocks on sensor query subprocesses.
        #[cfg(windows)]
        let temperature_readings = self.temperature.read();
        #[cfg(not(windows))]
        let temperature_readings = CpuTemperatureReadings::default();

        let voltage_volts = self.voltage.read().vcore;

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.sys.refresh_cpu_all();

        #[cfg(windows)]
        let native_frequencies = win_power::read_current_frequencies(inner.sys.cpus().len());
        #[cfg(not(windows))]
        let native_frequencies: Option<Vec<u64>> = None;

        let temperature_celsius = temperature_readings
            .package_celsius
            .or_else(|| read_cpu_temperature(&mut inner.components));

        let total_cpus = inner.sys.cpus().len();
        let num_core_clocks = temperature_readings.core_clocks.len();
        let cores_ratio = if num_core_clocks > 0 {
            (total_cpus / num_core_clocks).max(1)
        } else {
            1
        };

        let cores: Vec<CpuCoreSample> = inner
            .sys
            .cpus()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let lhm_core_freq = if num_core_clocks > 0 {
                    let core_idx = i / cores_ratio;
                    temperature_readings
                        .core_clocks
                        .get(core_idx)
                        .map(|clk| clk.frequency_mhz.round() as u64)
                        .filter(|f| *f >= 100)
                } else {
                    None
                };

                let frequency_mhz = lhm_core_freq
                    .or_else(|| {
                        native_frequencies
                            .as_ref()
                            .and_then(|frequencies| frequencies.get(i).copied())
                            .filter(|frequency| *frequency > 0)
                    })
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
        let native_package_freq = cores.iter().map(|c| c.frequency_mhz).max().unwrap_or(0);
        // CurrentMhz from CallNtPowerInformation is an OS throttle estimate on
        // modern Windows builds and can stay at the CPU's base clock. Prefer
        // the live per-core clock sensor when LibreHardwareMonitor exposes it.
        let package_freq = temperature_readings
            .fastest_core_clock_mhz
            .filter(|frequency| frequency.is_finite() && *frequency >= 100.0)
            .map(|frequency| frequency.round() as u64)
            .unwrap_or(native_package_freq);

        #[cfg(windows)]
        let usage_percent = inner.win_usage.sample().unwrap_or_else(|| inner.sys.global_cpu_usage());
        #[cfg(not(windows))]
        let usage_percent = inner.sys.global_cpu_usage();

        if package_freq > inner.max_freq_seen {
            inner.max_freq_seen = package_freq;
        }

        // Gauge scale: observed peak rounded up to the next 100 MHz with a
        // little headroom. Floored at 3000 MHz so the needle isn't pinned before
        // boost kicks in.
        let max_frequency_mhz = (((inner.max_freq_seen as f64) * 1.05) / 100.0).ceil() as u64 * 100;
        let max_frequency_mhz = max_frequency_mhz.max(3000);

        CpuSample {
            timestamp: chrono::Utc::now().timestamp_millis(),
            frequency_mhz: package_freq,
            max_frequency_mhz,
            usage_percent,
            temperature_celsius,
            core_temperatures: temperature_readings.core_temperatures,
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

/// Native Windows sub-millisecond CPU usage measurement via GetSystemTimes.
#[cfg(windows)]
pub mod win_times {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct FileTime {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }

    impl FileTime {
        fn as_u64(&self) -> u64 {
            ((self.dw_high_date_time as u64) << 32) | (self.dw_low_date_time as u64)
        }
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(
            idle_time: *mut FileTime,
            kernel_time: *mut FileTime,
            user_time: *mut FileTime,
        ) -> i32;
    }

    pub struct SystemCpuUsage {
        prev_idle: u64,
        prev_kernel: u64,
        prev_user: u64,
        has_baseline: bool,
    }

    impl SystemCpuUsage {
        pub fn new() -> Self {
            let mut s = Self {
                prev_idle: 0,
                prev_kernel: 0,
                prev_user: 0,
                has_baseline: false,
            };
            s.sample();
            s
        }

        pub fn sample(&mut self) -> Option<f32> {
            let mut idle = FileTime::default();
            let mut kernel = FileTime::default();
            let mut user = FileTime::default();

            let status = unsafe {
                GetSystemTimes(&mut idle, &mut kernel, &mut user)
            };
            if status == 0 {
                return None;
            }

            let idle_u64 = idle.as_u64();
            let kernel_u64 = kernel.as_u64();
            let user_u64 = user.as_u64();

            if !self.has_baseline {
                self.prev_idle = idle_u64;
                self.prev_kernel = kernel_u64;
                self.prev_user = user_u64;
                self.has_baseline = true;
                return None;
            }

            let idle_diff = idle_u64.saturating_sub(self.prev_idle);
            let kernel_diff = kernel_u64.saturating_sub(self.prev_kernel);
            let user_diff = user_u64.saturating_sub(self.prev_user);

            self.prev_idle = idle_u64;
            self.prev_kernel = kernel_u64;
            self.prev_user = user_u64;

            // In Windows, kernel_time already includes idle_time.
            // total_time = (kernel - prev_kernel) + (user - prev_user)
            let total = kernel_diff + user_diff;
            if total == 0 {
                return Some(0.0);
            }

            let busy = total.saturating_sub(idle_diff);
            let percent = (busy as f64 / total as f64 * 100.0) as f32;
            Some(percent.clamp(0.0, 100.0))
        }
    }
}

#[cfg(not(windows))]
pub mod win_times {}

#[cfg(windows)]
fn check_elevation() -> bool {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("net")
        .arg("session")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(not(windows))]
fn check_elevation() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_monitor_info_and_sample() {
        let monitor = CpuMonitor::new();
        let info = monitor.info();
        assert!(!info.model.is_empty(), "model should not be empty");
        assert!(info.logical_cores > 0, "logical cores should be > 0");

        let sample = monitor.sample();
        assert!(sample.frequency_mhz > 0, "frequency should be > 0");
        assert!(!sample.cores.is_empty(), "cores should not be empty");
        assert!(sample.usage_percent >= 0.0 && sample.usage_percent <= 100.0, "usage should be in 0..=100");
    }
}

