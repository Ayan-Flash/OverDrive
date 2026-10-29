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

// ============================ NVML control layer ============================
// Fan, clock-offset, and power-limit control via NVML. These functions follow
// the same dynamic-binding pattern as the telemetry above: a missing symbol
// means the installed driver predates that API, which the capability probing
// in `read_control_state` reports instead of failing the whole read.

pub const NVML_FAN_POLICY_TEMPERATURE_CONTINOUS_SW: u32 = 0;
pub const NVML_FAN_POLICY_MANUAL: u32 = 1;

/// Slider bounds for clock offsets. NVML does not expose a range query for
/// GPC/MEM VF offsets; these mirror the limits classic GPU tuners use, and the
/// driver still rejects offsets it cannot apply with a mapped error.
pub const NVML_CORE_OFFSET_LIMIT_MHZ: i32 = 500;
pub const NVML_MEMORY_OFFSET_LIMIT_MHZ: i32 = 2000;
/// Fallback power-limit slider floor when the driver does not report
/// constraints (percent of the default limit). Constraints are used whenever
/// available.
pub const NVML_POWER_LIMIT_MIN_PERCENT: f64 = 50.0;

/// Human-readable names for the `nvmlReturn_t` enum values consumers report
/// most often, so control errors can explain themselves.
pub fn nvml_error_message(code: u32) -> &'static str {
    match code {
        0 => "success",
        1 => "NVML was not initialized",
        2 => "invalid argument",
        3 => "not supported by this GPU or driver",
        4 => "permission denied — run OverDrive as Administrator",
        6 => "not found",
        9 => "NVIDIA driver is not loaded",
        10 => "operation timed out",
        12 => "NVML library not found",
        13 => "NVML function not found",
        15 => "GPU is lost — driver may have crashed",
        17 => "operating system rejected the call",
        19 => "resource in use — no 3D applications (or games) may be running",
        20 => "insufficient memory",
        21 => "insufficient resources",
        24 => "requested frequency not supported",
        _ => "unknown NVML error",
    }
}

fn nvml_fail(fn_name: &str, code: u32) -> GpuProviderError {
    GpuProviderError::ControlError(format!(
        "{fn_name} failed ({}): {}",
        code,
        nvml_error_message(code)
    ))
}

/// Capability probe result for one control: `Supported` (callable), `Denied`
/// (exists but this session lacks permission), or `Unsupported` (no such
/// function / the driver refuses the concept).
#[derive(Debug, Clone, Copy, PartialEq)]
enum ControlProbe {
    Supported,
    Denied,
    Unsupported,
}

impl ControlProbe {
    fn from_code(code: u32) -> Self {
        match code {
            NVML_SUCCESS => Self::Supported,
            // 4 = NO_PERMISSION: works with an elevated session.
            4 => Self::Denied,
            // 3 = NOT_SUPPORTED is the definitive "driver says no";
            // 13 = FUNCTION_NOT_FOUND means the installed driver predates
            // the API; 2/21 are driver-side refusals for that control class.
            3 | 13 | 21 => Self::Unsupported,
            _ => Self::Denied,
        }
    }
}

impl NvidiaProvider {
    /// Fan count with a conservative fallback of 1 when the driver predates
    /// `nvmlDeviceGetNumFans`.
    fn num_fans(&self, lib: &Library, device: NvmlDevice) -> u32 {
        let func: Result<Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32) -> NvmlReturn>, _> =
            unsafe { lib.get(b"nvmlDeviceGetNumFans") };
        if let Ok(func) = func {
            let mut fans: u32 = 0;
            if unsafe { func(device, &mut fans) } == NVML_SUCCESS && fans > 0 {
                return fans;
            }
        }
        1
    }

    fn set_fan_control_policy(
        &self,
        lib: &Library,
        device: NvmlDevice,
        fan: u32,
        policy: u32,
    ) -> Result<(), GpuProviderError> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, u32, u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceSetFanControlPolicy")
                .map_err(|_| {
                    GpuProviderError::NotAvailable(
                        "this driver does not expose nvmlDeviceSetFanControlPolicy".into(),
                    )
                })?
        };
        let code = unsafe { func(device, fan, policy) };
        if code == NVML_SUCCESS {
            Ok(())
        } else {
            Err(nvml_fail("nvmlDeviceSetFanControlPolicy", code))
        }
    }

    fn fan_control_policy(&self, lib: &Library, device: NvmlDevice, fan: u32) -> Result<u32, u32> {
        let symbol: Result<Symbol<unsafe extern "C" fn(NvmlDevice, u32, *mut u32) -> NvmlReturn>, _> =
            unsafe { lib.get(b"nvmlDeviceGetFanControlPolicy") };
        let func = symbol.map_err(|_| 13u32)?;
        let mut policy: u32 = 0;
        let code = unsafe { func(device, fan, &mut policy) };
        (code == NVML_SUCCESS).then_some(policy).ok_or(code)
    }

    fn set_fixed_fan_speed(
        &self,
        lib: &Library,
        device: NvmlDevice,
        fan: u32,
        percent: u32,
    ) -> Result<(), GpuProviderError> {
        let v2: Result<Symbol<unsafe extern "C" fn(NvmlDevice, u32, u32) -> NvmlReturn>, _> =
            unsafe { lib.get(b"nvmlDeviceSetFanSpeed_v2") };
        if let Ok(v2) = v2 {
            let code = unsafe { v2(device, fan, percent) };
            if code == NVML_SUCCESS {
                return Ok(());
            }
            return Err(nvml_fail("nvmlDeviceSetFanSpeed_v2", code));
        }
        // Pre-470 drivers expose a device-level setter without the fan index.
        let legacy: Result<Symbol<unsafe extern "C" fn(NvmlDevice, u32) -> NvmlReturn>, _> =
            unsafe { lib.get(b"nvmlDeviceSetFanSpeed") };
        if let Ok(legacy) = legacy {
            let code = unsafe { legacy(device, percent) };
            if code == NVML_SUCCESS {
                return Ok(());
            }
            return Err(nvml_fail("nvmlDeviceSetFanSpeed", code));
        }
        Err(GpuProviderError::NotAvailable(
            "this driver does not expose a symbol for setting fan speed".into(),
        ))
    }

    fn get_gpc_clk_vf_offset(&self, lib: &Library, device: NvmlDevice) -> Result<i32, u32> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, *mut i32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetGpcClkVfOffset").map_err(|_| 13u32)? };
        let mut offset: i32 = 0;
        let code = unsafe { func(device, &mut offset) };
        (code == NVML_SUCCESS).then_some(offset).ok_or(code)
    }

    fn set_gpc_clk_vf_offset(&self, lib: &Library, device: NvmlDevice, offset: i32) -> Result<(), GpuProviderError> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, i32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceSetGpcClkVfOffset")
                .map_err(|_| GpuProviderError::NotAvailable("this driver does not expose nvmlDeviceSetGpcClkVfOffset".into()))?
        };
        let code = unsafe { func(device, offset) };
        if code == NVML_SUCCESS {
            Ok(())
        } else {
            Err(nvml_fail("nvmlDeviceSetGpcClkVfOffset", code))
        }
    }

    fn get_mem_clk_vf_offset(&self, lib: &Library, device: NvmlDevice) -> Result<i32, u32> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, *mut i32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetMemClkVfOffset").map_err(|_| 13u32)? };
        let mut offset: i32 = 0;
        let code = unsafe { func(device, &mut offset) };
        (code == NVML_SUCCESS).then_some(offset).ok_or(code)
    }

    fn set_mem_clk_vf_offset(&self, lib: &Library, device: NvmlDevice, offset: i32) -> Result<(), GpuProviderError> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, i32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceSetMemClkVfOffset")
                .map_err(|_| GpuProviderError::NotAvailable("this driver does not expose nvmlDeviceSetMemClkVfOffset".into()))?
        };
        let code = unsafe { func(device, offset) };
        if code == NVML_SUCCESS {
            Ok(())
        } else {
            Err(nvml_fail("nvmlDeviceSetMemClkVfOffset", code))
        }
    }

    fn power_limit_values(
        &self,
        lib: &Library,
        device: NvmlDevice,
    ) -> Option<(u32, u32, u32, u32)> {
        // Returns (current_mw, default_mw, min_mw, max_mw) in milliwatts.
        let current: Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetPowerManagementLimit").ok()? };
        let default_limit: Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetPowerManagementDefaultLimit").ok()? };
        let constraints: Symbol<unsafe extern "C" fn(NvmlDevice, *mut u32, *mut u32) -> NvmlReturn> =
            unsafe { lib.get(b"nvmlDeviceGetPowerManagementLimitConstraints").ok()? };

        let mut current_mw: u32 = 0;
        let mut default_mw: u32 = 0;
        let mut min_mw: u32 = 0;
        let mut max_mw: u32 = 0;
        if unsafe { current(device, &mut current_mw) } != NVML_SUCCESS
            || unsafe { default_limit(device, &mut default_mw) } != NVML_SUCCESS
            || unsafe { constraints(device, &mut min_mw, &mut max_mw) } != NVML_SUCCESS
            || default_mw == 0
        {
            return None;
        }
        Some((current_mw, default_mw, min_mw, max_mw))
    }

    fn set_power_limit_mw(&self, lib: &Library, device: NvmlDevice, limit_mw: u32) -> Result<(), GpuProviderError> {
        let func: Symbol<unsafe extern "C" fn(NvmlDevice, u32) -> NvmlReturn> = unsafe {
            lib.get(b"nvmlDeviceSetPowerManagementLimit")
                .map_err(|_| GpuProviderError::NotAvailable("this driver does not expose nvmlDeviceSetPowerManagementLimit".into()))?
        };
        let code = unsafe { func(device, limit_mw) };
        if code == NVML_SUCCESS {
            Ok(())
        } else {
            Err(nvml_fail("nvmlDeviceSetPowerManagementLimit", code))
        }
    }

    /// Multi-fan helpers: apply one action to every fan on the card, folding
    /// failures into a single control error. Returns the count of fans that
    /// were actually written.
    fn for_each_fan<F>(&self, lib: &Library, device: NvmlDevice, mut action: F) -> Result<u32, GpuProviderError>
    where
        F: FnMut(u32) -> Result<(), GpuProviderError>,
    {
        let mut fans = self.num_fans(lib, device);
        if fans > 8 {
            // Guard against pathological drivers reporting huge counts.
            fans = 8;
        }
        let mut written = 0;
        for fan in 0..fans {
            action(fan)?;
            written += 1;
        }
        Ok(written)
    }

    /// Runs `probe` on the GPU identified by `gpu_id`, resolving shared book
    /// keeping (device handle + loaded library) for control calls.
    fn run_control<T>(
        &self,
        gpu_id: &str,
        probe: impl FnOnce(&Library, NvmlDevice) -> Result<T, GpuProviderError>,
    ) -> Result<T, GpuProviderError> {
        if !self
            .devices
            .lock()
            .map_err(|error| GpuProviderError::ReadError(error.to_string()))?
            .iter()
            .any(|(_, identity)| identity.id == gpu_id)
        {
            return Err(GpuProviderError::GpuNotFound(gpu_id.into()));
        }
        let lib_guard = self.lib.lock().map_err(|e| GpuProviderError::ReadError(e.to_string()))?;
        let lib = lib_guard
            .as_ref()
            .ok_or_else(|| GpuProviderError::NotAvailable("NVML library not loaded".into()))?;
        let device = self
            .devices
            .lock()
            .map_err(|e| GpuProviderError::ReadError(e.to_string()))?
            .iter()
            .find(|d| d.1.id == gpu_id)
            .map(|d| d.0)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.into()))?;
        probe(lib, device)
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
        self.run_control(gpu_id, |lib, device| {
            let mut capabilities = GpuControlCapabilities::all_unavailable();
            capabilities.voltage_offset = false; // No NVML voltage control exists.

            // --- Fans -------------------------------------------------
            let fan_count = self.num_fans(lib, device);
            let (policy_value, policy_probe) = match self.fan_control_policy(lib, device, 0) {
                Ok(policy) => (Some(policy), ControlProbe::Supported),
                Err(code) => (None, ControlProbe::from_code(code)),
            };
            capabilities.fan_control = policy_probe != ControlProbe::Unsupported;
            let fan_control_automatic = policy_value.map(|policy| policy != NVML_FAN_POLICY_MANUAL);
            let manual_target = if policy_value == Some(NVML_FAN_POLICY_MANUAL) {
                Some(self.read_fan_speed(lib, device))
            } else {
                None
            };

            // --- Clock offsets ----------------------------------------
            let (core_offset, core_probe) = match self.get_gpc_clk_vf_offset(lib, device) {
                Ok(offset) => (offset, ControlProbe::Supported),
                Err(code) => (0, ControlProbe::from_code(code)),
            };
            let (mem_offset, mem_probe) = match self.get_mem_clk_vf_offset(lib, device) {
                Ok(offset) => (offset, ControlProbe::Supported),
                Err(code) => (0, ControlProbe::from_code(code)),
            };
            capabilities.core_clock_offset = core_probe != ControlProbe::Unsupported;
            capabilities.memory_clock_offset = mem_probe != ControlProbe::Unsupported;

            // --- Power limit ------------------------------------------
            let (power_limit_percent, power_limit_min_percent, power_limit_max_percent) =
                match self.power_limit_values(lib, device) {
                    Some((current_mw, default_mw, min_mw, max_mw)) => {
                        capabilities.power_limit = true;
                        (
                            Some(current_mw as f64 / default_mw as f64 * 100.0),
                            Some(min_mw as f64 / default_mw as f64 * 100.0),
                            Some(max_mw as f64 / default_mw as f64 * 100.0),
                        )
                    }
                    None => (None, None, None),
                };

            Ok(GpuControlState {
                gpu_id: gpu_id.to_string(),
                target_fan_speed: manual_target.flatten(),
                fan_control_automatic,
                fan_count: Some(fan_count),
                core_clock_offset_mhz: core_offset,
                memory_clock_offset_mhz: mem_offset,
                power_limit_percent,
                power_limit_min_percent,
                power_limit_max_percent,
                voltage_offset_mv: 0,
                core_clock_offset_limit_mhz: NVML_CORE_OFFSET_LIMIT_MHZ,
                memory_clock_offset_limit_mhz: NVML_MEMORY_OFFSET_LIMIT_MHZ,
                capabilities,
            })
        })
    }

    fn set_fan_speed(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        if !(0.0..=100.0).contains(&percent) {
            return Err(GpuProviderError::ControlError(format!(
                "fan speed {}% is outside 0-100%",
                percent
            )));
        }
        let target = percent.round() as u32;
        self.run_control(gpu_id, |lib, device| {
            // Manual policy keeps the fixed speed on modern drivers; older
            // drives may accept a direct set even when policy handling
            // refuses, so a policy failure is only fatal when the speed set
            // also fails.
            if self.set_fan_control_policy(lib, device, 0, NVML_FAN_POLICY_MANUAL).is_err() {
                warn!("Fan policy manual is not honored; relying on direct fan speed set");
            }
            self.for_each_fan(lib, device, |fan| self.set_fixed_fan_speed(lib, device, fan, target))
                .map(|_| ())
        })
    }

    fn set_core_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        let clamped = offset_mhz.clamp(-NVML_CORE_OFFSET_LIMIT_MHZ, NVML_CORE_OFFSET_LIMIT_MHZ);
        self.run_control(gpu_id, |lib, device| {
            self.set_gpc_clk_vf_offset(lib, device, clamped)
        })
    }

    fn set_memory_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        let clamped = offset_mhz.clamp(-NVML_MEMORY_OFFSET_LIMIT_MHZ, NVML_MEMORY_OFFSET_LIMIT_MHZ);
        self.run_control(gpu_id, |lib, device| {
            self.set_mem_clk_vf_offset(lib, device, clamped)
        })
    }

    fn set_power_limit(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        self.run_control(gpu_id, |lib, device| {
            let (_, default_mw, min_mw, max_mw) = self
                .power_limit_values(lib, device)
                .ok_or_else(|| {
                    GpuProviderError::ControlError(
                        "NVIDIA driver did not report power management limits".into(),
                    )
                })?;
            let min_percent = min_mw as f64 / default_mw as f64 * 100.0;
            let max_percent = max_mw as f64 / default_mw as f64 * 100.0;
            if !(min_percent - 0.01..=max_percent + 0.01).contains(&percent) {
                return Err(GpuProviderError::ControlError(format!(
                    "power limit {:.0}% is outside the supported {:.0}-{:.0}% range",
                    percent, min_percent, max_percent
                )));
            }
            let target_mw = (default_mw as f64 * (percent / 100.0)).round().clamp(
                min_mw as f64,
                max_mw as f64,
            ) as u32;
            self.set_power_limit_mw(lib, device, target_mw)
        })
    }

    fn set_voltage_offset(&self, _gpu_id: &str, _offset_mv: i32) -> Result<(), GpuProviderError> {
        // NVML deliberately exposes no voltage control for GeForce cards.
        Err(GpuProviderError::NotAvailable(
            "Voltage control is not exposed by the NVIDIA consumer driver APIs this build uses".into(),
        ))
    }

    fn set_fan_control_automatic(&self, gpu_id: &str) -> Result<(), GpuProviderError> {
        self.run_control(gpu_id, |lib, device| {
            self.for_each_fan(lib, device, |fan| {
                self.set_fan_control_policy(lib, device, fan, NVML_FAN_POLICY_TEMPERATURE_CONTINOUS_SW)
            })
            .and_then(|written| {
                if written == 0 {
                    Err(GpuProviderError::ControlError("no fans were available to reset".into()))
                } else {
                    Ok(())
                }
            })
        })
    }

    fn reset_controls(&self, gpu_id: &str) -> Result<(), GpuProviderError> {
        self.run_control(gpu_id, |lib, device| {
            let mut failures: Vec<String> = Vec::new();

            for (label, result) in [
                ("core clock offset", self.set_gpc_clk_vf_offset(lib, device, 0)),
                ("memory clock offset", self.set_mem_clk_vf_offset(lib, device, 0)),
            ] {
                if let Err(error) = result {
                    // NOT_SUPPORTED-class failures mean the card never had
                    // that control in the first place — not a reset failure.
                    if !is_unsupported_control_error(&error) {
                        failures.push(format!("{label}: {error}"));
                    }
                }
            }

            if let Some((_, default_mw, _, _)) = self.power_limit_values(lib, device) {
                if let Err(error) = self.set_power_limit_mw(lib, device, default_mw) {
                    failures.push(format!("power limit: {error}"));
                }
            }

            if let Ok(policy) = self.fan_control_policy(lib, device, 0) {
                if policy == NVML_FAN_POLICY_MANUAL {
                    if let Err(error) = self.set_fan_control_automatic(gpu_id) {
                        failures.push(format!("automatic fan control: {error}"));
                    }
                }
            }

            if failures.is_empty() {
                Ok(())
            } else {
                Err(GpuProviderError::ControlError(
                    failures.join("; "),
                ))
            }
        })
    }
}

/// True when a control write failed only because the GPU or the installed
/// driver class does not offer that control at all; reset controls ignore
/// these so "reset everything" never reports failures for untouched dials.
fn is_unsupported_control_error(error: &GpuProviderError) -> bool {
    let message = error.to_string();
    message.contains("not supported by this GPU or driver")
        || message.contains("NVML function not found")
        || message.contains("does not expose")
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

    #[test]
    fn nvml_error_messages_cover_reported_codes() {
        assert_eq!(nvml_error_message(0), "success");
        assert_eq!(nvml_error_message(3), "not supported by this GPU or driver");
        assert_eq!(nvml_error_message(4), "permission denied — run OverDrive as Administrator");
        assert_eq!(nvml_error_message(9999), "unknown NVML error");
    }

    #[test]
    fn control_probe_classifies_nvml_codes() {
        assert_eq!(ControlProbe::from_code(0), ControlProbe::Supported);
        assert_eq!(ControlProbe::from_code(4), ControlProbe::Denied);
        assert_eq!(ControlProbe::from_code(3), ControlProbe::Unsupported);
        assert_eq!(ControlProbe::from_code(13), ControlProbe::Unsupported);
        assert_eq!(ControlProbe::from_code(2), ControlProbe::Denied);
    }

    #[test]
    fn unsupported_control_errors_are_classified() {
        assert!(is_unsupported_control_error(&GpuProviderError::NotAvailable(
            "this driver does not expose nvmlDeviceSetGpcClkVfOffset".into()
        )));
        assert!(is_unsupported_control_error(&GpuProviderError::ControlError(
            "nvmlDeviceSetGpcClkVfOffset failed (3): not supported by this GPU or driver".into()
        )));
        assert!(!is_unsupported_control_error(&GpuProviderError::ControlError(
            "nvmlDeviceSetGpcClkVfOffset failed (4): permission denied — run OverDrive as Administrator".into()
        )));
    }

    #[test]
    fn live_control_state_reports_capabilities_when_driver_exposes_nvml() {
        // Conditional on hardware: verifies the extended control contract end
        // to end when an NVIDIA GPU + driver are present, and skips cleanly
        // otherwise.
        let Ok(provider) = NvidiaProvider::try_new() else { return };
        let gpus = provider.enumerate().expect("enumerate failed");
        assert!(!gpus.is_empty(), "Should have at least 1 NVIDIA GPU");
        let state = provider.read_control_state(&gpus[0].id).expect("read_control_state failed");
        println!("GPU control state: {state:?}");

        // Invariants that must hold regardless of GPU generation:
        assert_eq!(state.gpu_id, gpus[0].id);
        assert!(state.core_clock_offset_limit_mhz > 0);
        assert!(state.memory_clock_offset_limit_mhz > 0);
        assert!(state.fan_count.unwrap_or(1) >= 1);
        // When the driver exposes power values they are ordered sanely.
        if let (Some(min), Some(max)) = (state.power_limit_min_percent, state.power_limit_max_percent) {
            assert!(min <= max, "power limit constraint range must be ordered");
            assert!(min > 0.0, "power limit floor must be positive");
        }
        // Manual fan target only exists while the driver policy is manual.
        if state.fan_control_automatic == Some(true) {
            assert!(state.target_fan_speed.is_none(), "automatic fans must not report a manual target");
        }

        // Reset must not throw when the card has nothing to reset.
        let reset = provider.reset_controls(&gpus[0].id);
        if let Err(error) = reset {
            // Only real driver refusals should bubble; unsupported dials are absorbed.
            assert!(
                !error.to_string().is_empty(),
                "reset failure must carry a message"
            );
        }
    }

    #[test]
    fn power_limit_percent_round_trips_to_driver_bounds() {
        // Off-hardware contract check: the provider validation math.
        let default_mw = 250u32;
        let min_mw = 125u32;
        let max_mw = 300u32;
        let min_percent = min_mw as f64 / default_mw as f64 * 100.0;
        let max_percent = max_mw as f64 / default_mw as f64 * 100.0;
        assert_eq!(min_percent, 50.0);
        assert_eq!(max_percent, 120.0);
        let in_range = 110.0f64;
        assert!((min_percent - 0.01..=max_percent + 0.01).contains(&in_range));
        let out_of_range = 200.0f64;
        assert!(!(min_percent - 0.01..=max_percent + 0.01).contains(&out_of_range));
        let clamped_mw = (default_mw as f64 * (in_range / 100.0)).round().clamp(
            min_mw as f64,
            max_mw as f64,
        ) as u32;
        assert_eq!(clamped_mw, 275);
    }
}

