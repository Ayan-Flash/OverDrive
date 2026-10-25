use std::sync::Mutex;

use std::path::PathBuf;
use libloading::{Library, Symbol};
use tracing::{info, warn};

use super::provider::*;

type NvmlReturn = u32;
const NVML_SUCCESS: u32 = 0;
type NvmlDevice = *mut std::ffi::c_void;

fn load_nvml_library() -> Result<Library, String> {
    let mut candidates = Vec::<PathBuf>::new();

    #[cfg(windows)]
    {
        if let Some(program_files) = std::env::var_os("ProgramW6432")
            .or_else(|| std::env::var_os("ProgramFiles"))
        {
            candidates.push(
                PathBuf::from(program_files)
                    .join("NVIDIA Corporation")
                    .join("NVSMI")
                    .join("nvml.dll"),
            );
        }
        if let Some(windows_dir) = std::env::var_os("SystemRoot") {
            candidates.push(PathBuf::from(windows_dir).join("System32").join("nvml.dll"));
        }
    }

    // Keep PATH lookup as a fallback for custom driver installations.
    candidates.push(PathBuf::from("nvml.dll"));
    let mut errors = Vec::new();
    for candidate in candidates {
        match unsafe { Library::new(&candidate) } {
            Ok(library) => {
                info!(path = %candidate.display(), "Loaded NVIDIA NVML library");
                return Ok(library);
            }
            Err(error) => errors.push(format!("{}: {error}", candidate.display())),
        }
    }
    Err(format!("Could not load nvml.dll; tried {}", errors.join("; ")))
}

pub struct NvidiaProvider {
    lib: Mutex<Option<Library>>,
    devices: Mutex<Vec<(NvmlDevice, GpuIdentity)>>,
}

unsafe impl Send for NvidiaProvider {}
unsafe impl Sync for NvidiaProvider {}

impl NvidiaProvider {
    pub fn try_new() -> Result<Self, String> {
        let lib = load_nvml_library()?;

        let nvml_init: Symbol<unsafe extern "C" fn() -> NvmlReturn> = unsafe {
            lib.get(b"nvmlInit_v2").map_err(|e| format!("nvmlInit_v2 not found: {}", e))?
        };

        let nvml_device_get_count: Symbol<unsafe extern "C" fn(*mut u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceGetCount_v2")
                .map_err(|e| format!("nvmlDeviceGetCount_v2 not found: {}", e))?
        };

        let nvml_device_get_handle_by_index: Symbol<
            unsafe extern "C" fn(u32, *mut NvmlDevice) -> NvmlReturn,
        > = unsafe {
            lib.get(b"nvmlDeviceGetHandleByIndex_v2")
                .map_err(|e| format!("nvmlDeviceGetHandleByIndex not found: {}", e))?
        };

        let nvml_device_get_name: Symbol<
            unsafe extern "C" fn(NvmlDevice, *mut i8, u32) -> NvmlReturn,
        > = unsafe {
            lib.get(b"nvmlDeviceGetName")
                .map_err(|e| format!("nvmlDeviceGetName not found: {}", e))?
        };

        let result = unsafe { nvml_init() };
        if result != NVML_SUCCESS {
            return Err(format!("nvmlInit_v2 failed: error {}", result));
        }

        info!("NVML initialized successfully");

        let mut count: u32 = 0;
        let result = unsafe { nvml_device_get_count(&mut count) };
        if result != NVML_SUCCESS || count == 0 {
            if let Ok(nvml_shutdown) = unsafe {
                lib.get::<unsafe extern "C" fn() -> NvmlReturn>(b"nvmlShutdown")
            } {
                unsafe { nvml_shutdown(); }
            }
            return if result == NVML_SUCCESS {
                Err("NVML found no NVIDIA GPUs".into())
            } else {
                Err(format!("nvmlDeviceGetCount_v2 failed: error {result}"))
            };
        }

        let mut devices = Vec::new();
        for i in 0..count {
            let mut handle: NvmlDevice = std::ptr::null_mut();
            let result = unsafe { nvml_device_get_handle_by_index(i, &mut handle) };
            if result != NVML_SUCCESS {
                warn!(index = i, "Failed to get device handle");
                continue;
            }

            let mut name_buf = [0i8; 256];
            let name_result = unsafe { nvml_device_get_name(handle, name_buf.as_mut_ptr(), 256) };
            if name_result != NVML_SUCCESS {
                warn!(index = i, "Failed to read NVIDIA GPU name");
                continue;
            }
            let name = unsafe {
                std::ffi::CStr::from_ptr(name_buf.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };

            let identity = GpuIdentity {
                id: format!("nvidia-{}", i),
                name,
                vendor: GpuVendor::Nvidia,
                index: i,
                memory_total_mb: Self::get_memory_info(&lib, handle).map(|bytes| bytes / (1024 * 1024)),
            };

            info!(name = %identity.name, index = i, "Found NVIDIA GPU");
            devices.push((handle, identity));
        }

        if devices.is_empty() {
            if let Ok(nvml_shutdown) = unsafe {
                lib.get::<unsafe extern "C" fn() -> NvmlReturn>(b"nvmlShutdown")
            } {
                unsafe { nvml_shutdown(); }
            }
            return Err("NVML initialized but no usable NVIDIA devices were found".into());
        }

        Ok(Self {
            lib: Mutex::new(Some(lib)),
            devices: Mutex::new(devices),
        })
    }

    /// Query total memory (bytes) for the given device. Used once at
    /// enumeration to fill `memory_total_mb`.
    fn get_memory_info(lib: &Library, device: NvmlDevice) -> Option<u64> {
        Self::read_memory(lib, device).map(|(total, _used)| total)
    }

    /// Read `(total, used)` device memory in bytes via `nvmlDeviceGetMemoryInfo`.
    fn read_memory(lib: &Library, device: NvmlDevice) -> Option<(u64, u64)> {
        #[repr(C)]
        struct NvmlMemory {
            total: u64,
            free: u64,
            used: u64,
        }

        let func: Symbol<unsafe extern "C" fn(NvmlDevice, *mut NvmlMemory) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceGetMemoryInfo").ok()?
        };
        let mut mem = NvmlMemory { total: 0, free: 0, used: 0 };
        let result = unsafe { func(device, &mut mem) };
        if result == NVML_SUCCESS && mem.total > 0 && mem.used <= mem.total {
            Some((mem.total, mem.used))
        } else {
            None
        }
    }

    fn read_temp(&self, lib: &Library, device: NvmlDevice) -> Option<f64> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, u32, *mut u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceGetTemperature").ok()?
        };
        let mut temp: u32 = 0;
        let result = unsafe { func(device, 0, &mut temp) }; // NVML_TEMPERATURE_GPU = 0
        if result == NVML_SUCCESS && temp <= 150 { Some(temp as f64) } else { None }
    }

    fn read_clocks(&self, lib: &Library, device: NvmlDevice) -> (Option<f64>, Option<f64>) {
        if let Ok(func) = unsafe { lib.get::<unsafe extern "C" fn(NvmlDevice, u32, *mut u32) -> NvmlReturn>(b"nvmlDeviceGetClockInfo") } {
            let mut core: u32 = 0;
            let core_result = unsafe { func(device, 0, &mut core) }; // NVML_CLOCK_GRAPHICS = 0

            let mut mem: u32 = 0;
            let mem_result = unsafe { func(device, 1, &mut mem) }; // NVML_CLOCK_MEM = 1

            (
                if core_result == NVML_SUCCESS { Some(core as f64) } else { None },
                if mem_result == NVML_SUCCESS { Some(mem as f64) } else { None },
            )
        } else {
            (None, None)
        }
    }

    fn read_power(&self, lib: &Library, device: NvmlDevice) -> Option<f64> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceGetPowerUsage").ok()?
        };
        let mut power_milliwatts = 0;
        let result = unsafe { func(device, &mut power_milliwatts) };
        (result == NVML_SUCCESS).then_some(power_milliwatts as f64 / 1000.0)
    }

    fn read_max_power_limit(&self, lib: &Library, device: NvmlDevice) -> Option<f64> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32, *mut u32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetPowerManagementLimitConstraints").ok()? };
        let mut min_mw = 0;
        let mut max_mw = 0;
        let result = unsafe { func(device, &mut min_mw, &mut max_mw) };
        if result == NVML_SUCCESS && max_mw > 0 {
            Some(max_mw as f64 / 1000.0)
        } else {
            None
        }
    }

    fn read_fan_speed(&self, lib: &Library, device: NvmlDevice) -> Option<f64> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, u32, *mut u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceGetFanSpeed_v2").ok()?
        };
        let mut speed: u32 = 0;
        let result = unsafe { func(device, 0, &mut speed) };
        if result == NVML_SUCCESS && speed <= 100 {
            Some(speed as f64)
        } else {
            None
        }
    }

    fn read_utilization(&self, lib: &Library, device: NvmlDevice) -> (Option<f64>, Option<f64>) {
        #[repr(C)]
        struct NvmlUtilization {
            gpu: u32,
            memory: u32,
        }

        if let Ok(func) = unsafe { lib.get::<unsafe extern "C" fn(NvmlDevice, *mut NvmlUtilization) -> NvmlReturn>(b"nvmlDeviceGetUtilizationRates") } {
            let mut util = NvmlUtilization { gpu: 0, memory: 0 };
            let result = unsafe { func(device, &mut util) };
            if result == NVML_SUCCESS && util.gpu <= 100 && util.memory <= 100 {
                (Some(util.gpu as f64), Some(util.memory as f64))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }

    fn read_pcie_info(&self, lib: &Library, device: NvmlDevice) -> (Option<f64>, Option<f64>) {
        if let Ok(func) = unsafe { lib.get::<unsafe extern "C" fn(NvmlDevice, *mut u32) -> NvmlReturn>(b"nvmlDeviceGetCurrPcieLinkWidth") } {
            let mut width: u32 = 0;
            let result = unsafe { func(device, &mut width) };

            if result == NVML_SUCCESS {
                (Some(width as f64), None)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }
}

impl GpuProvider for NvidiaProvider {
    fn name(&self) -> &str {
        "NVIDIA NVML"
    }

    fn enumerate(&self) -> Result<Vec<GpuIdentity>, GpuProviderError> {
        let devices = self.devices.lock().map_err(|e| {
            GpuProviderError::ReadError(e.to_string())
        })?;
        Ok(devices.iter().map(|d| d.1.clone()).collect())
    }

    fn read_sample(&self, gpu_id: &str) -> Result<GpuSample, GpuProviderError> {
        let devices = self.devices.lock().map_err(|e| {
            GpuProviderError::ReadError(e.to_string())
        })?;

        let (device, identity) = devices
            .iter()
            .find(|d| d.1.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.into()))?;

        let lib = self.lib.lock().map_err(|e| {
            GpuProviderError::ReadError(e.to_string())
        })?;
        let lib_ref = lib.as_ref().ok_or_else(|| {
            GpuProviderError::NotAvailable("NVML library not loaded".into())
        })?;

        let temp = self.read_temp(lib_ref, *device);
        let (core_clock, mem_clock) = self.read_clocks(lib_ref, *device);
        let power = self.read_power(lib_ref, *device);
        let fan = self.read_fan_speed(lib_ref, *device);
        let (gpu_util, mem_util) = self.read_utilization(lib_ref, *device);
        // Real used VRAM from NVML (bytes → MB). Falls back to the cached total
        // if the query fails, so `memory_used_mb` is never a bogus 0.
        let memory = Self::read_memory(lib_ref, *device);

        Ok(GpuSample {
            gpu_id: identity.id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            temperature_celsius: temp,
            core_clock_mhz: core_clock,
            memory_clock_mhz: mem_clock,
            memory_used_mb: memory.map(|(_, used)| used / (1024 * 1024)),
            memory_total_mb: memory.map(|(total, _)| total / (1024 * 1024)).or(identity.memory_total_mb),
            fan_speed_percent: fan,
            // NVML exposes fan duty percent on many consumer cards, but not
            // physical RPM. Keep RPM unavailable instead of fabricating it.
            fan_rpm: None,
            power_watts: power,
            max_power_limit_watts: self.read_max_power_limit(lib_ref, *device),
            // Unsupported readings remain absent; successful zero values remain Some(0).
            core_voltage_mv: None,
            core_utilization_percent: gpu_util,
            memory_utilization_percent: mem_util,
        })
    }

    fn read_control_state(&self, gpu_id: &str) -> Result<GpuControlState, GpuProviderError> {
        if !self.devices.lock().map_err(|error| GpuProviderError::ReadError(error.to_string()))?
            .iter().any(|(_, identity)| identity.id == gpu_id)
        {
            return Err(GpuProviderError::GpuNotFound(gpu_id.into()));
        }
        Err(GpuProviderError::NotAvailable(
            "GPU control state is not exposed by the current NVML integration".into(),
        ))
    }

    fn set_fan_speed(&self, _gpu_id: &str, _percent: f64) -> Result<(), GpuProviderError> {
        Err(GpuProviderError::NotAvailable(
            "Fan control not implemented via NVML".into(),
        ))
    }

    fn set_core_clock_offset(&self, _gpu_id: &str, _offset_mhz: i32) -> Result<(), GpuProviderError> {
        Err(GpuProviderError::NotAvailable(
            "Clock control not implemented via NVML".into(),
        ))
    }

    fn set_memory_clock_offset(&self, _gpu_id: &str, _offset_mhz: i32) -> Result<(), GpuProviderError> {
        Err(GpuProviderError::NotAvailable(
            "Memory clock control not implemented via NVML".into(),
        ))
    }

    fn set_power_limit(&self, _gpu_id: &str, _percent: f64) -> Result<(), GpuProviderError> {
        Err(GpuProviderError::NotAvailable(
            "Power limit control not implemented via NVML".into(),
        ))
    }

    fn set_voltage_offset(&self, _gpu_id: &str, _offset_mv: i32) -> Result<(), GpuProviderError> {
        Err(GpuProviderError::NotAvailable(
            "Voltage control not implemented via NVML".into(),
        ))
    }
}

impl Drop for NvidiaProvider {
    fn drop(&mut self) {
        if let Ok(mut lib) = self.lib.lock() {
            if let Some(library) = lib.take() {
                unsafe {
                    if let Ok(shutdown) = library.get::<unsafe extern "C" fn() -> NvmlReturn>(b"nvmlShutdown") {
                        shutdown();
                    }
                }
                info!("NVML shutdown complete");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nvidia_provider_sample_live() {
        if let Ok(provider) = NvidiaProvider::try_new() {
            let gpus = provider.enumerate().expect("enumerate failed");
            assert!(!gpus.is_empty(), "Should have at least 1 NVIDIA GPU");
            let sample = provider.read_sample(&gpus[0].id).expect("read_sample failed");
            println!("NVIDIA GPU Sample: {:?}", sample);
            assert!(sample.temperature_celsius.is_some(), "GPU temperature should be available");
            println!("GPU Power watts: {:?}", sample.power_watts);
        }
    }
}

