//! Native system memory and volume capacity readings.
//! Physical-memory used means `total - available` (available includes memory
//! Windows can reclaim without paging). Volume used means `total - volume_free`.
use serde::{Deserialize, Serialize};

fn win32_result(succeeded: bool, error: impl ToString) -> Result<(), String> {
    if succeeded {
        Ok(())
    } else {
        Err(error.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RamTelemetry {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub utilization_percent: f64,
}

pub fn calculate_ram(total_bytes: u64, available_bytes: u64) -> Result<RamTelemetry, String> {
    if total_bytes == 0 {
        return Err("total physical memory is zero".into());
    }
    if available_bytes > total_bytes {
        return Err("available physical memory exceeds total".into());
    }
    let used_bytes = total_bytes - available_bytes;
    let utilization_percent = (used_bytes as f64 / total_bytes as f64) * 100.0;
    if !utilization_percent.is_finite() || !(0.0..=100.0).contains(&utilization_percent) {
        return Err("invalid physical memory utilization".into());
    }
    Ok(RamTelemetry {
        total_bytes,
        available_bytes,
        used_bytes,
        utilization_percent,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageTelemetry {
    pub volume: String,
    pub total_bytes: u64,
    pub user_available_bytes: u64,
    pub volume_free_bytes: u64,
    pub used_bytes: u64,
    pub utilization_percent: f64,
}

pub fn calculate_storage(
    volume: String,
    total_bytes: u64,
    user_available_bytes: u64,
    volume_free_bytes: u64,
) -> Result<StorageTelemetry, String> {
    if volume.trim().is_empty() {
        return Err("volume identity is empty".into());
    }
    if total_bytes == 0 {
        return Err("volume capacity is zero".into());
    }
    if user_available_bytes > total_bytes || volume_free_bytes > total_bytes {
        return Err("free capacity exceeds volume capacity".into());
    }
    // The API reports user-available separately; volume usage is computed using total free space.
    let used_bytes = total_bytes - volume_free_bytes;
    let utilization_percent = used_bytes as f64 / total_bytes as f64 * 100.0;
    if !utilization_percent.is_finite() || !(0.0..=100.0).contains(&utilization_percent) {
        return Err("invalid volume utilization".into());
    }
    Ok(StorageTelemetry {
        volume,
        total_bytes,
        user_available_bytes,
        volume_free_bytes,
        used_bytes,
        utilization_percent,
    })
}

#[cfg(windows)]
pub fn read_ram() -> Result<RamTelemetry, String> {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended: u64,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }
    let mut status = MemoryStatusEx {
        length: std::mem::size_of::<MemoryStatusEx>() as u32,
        memory_load: 0,
        total_phys: 0,
        avail_phys: 0,
        total_page: 0,
        avail_page: 0,
        total_virtual: 0,
        avail_virtual: 0,
        avail_extended: 0,
    };
    // SAFETY: status points to a correctly sized, initialized MEMORYSTATUSEX structure for this call.
    win32_result(
        unsafe { GlobalMemoryStatusEx(&mut status) } != 0,
        std::io::Error::last_os_error(),
    )?;
    calculate_ram(status.total_phys, status.avail_phys)
}

#[cfg(not(windows))]
pub fn read_ram() -> Result<RamTelemetry, String> {
    Err("native RAM telemetry is currently supported on Windows only".into())
}

#[cfg(windows)]
pub fn read_storage(volume: &str) -> Result<StorageTelemetry, String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetDiskFreeSpaceExW(
            path: *const u16,
            available: *mut u64,
            total: *mut u64,
            free: *mut u64,
        ) -> i32;
    }
    validate_volume_path(volume)?;
    let mut path: Vec<u16> = std::ffi::OsStr::new(volume).encode_wide().collect();
    path.push(0);
    let (mut available, mut total, mut free) = (0u64, 0u64, 0u64);
    // SAFETY: path is NUL-terminated UTF-16 and output pointers reference initialized u64 values.
    win32_result(
        unsafe { GetDiskFreeSpaceExW(path.as_ptr(), &mut available, &mut total, &mut free) } != 0,
        std::io::Error::last_os_error(),
    )?;
    calculate_storage(volume.to_owned(), total, available, free)
}

fn validate_volume_path(volume: &str) -> Result<(), String> {
    if volume.trim().is_empty() || volume.encode_utf16().any(|unit| unit == 0) {
        return Err("invalid volume path".into());
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn read_storage(_volume: &str) -> Result<StorageTelemetry, String> {
    Err("native storage telemetry is currently supported on Windows only".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ram_calculations_and_zero_are_valid() {
        assert_eq!(calculate_ram(100, 40).unwrap().used_bytes, 60);
        assert_eq!(calculate_ram(100, 100).unwrap().utilization_percent, 0.0);
    }
    #[test]
    fn invalid_ram_inputs_are_rejected() {
        assert!(calculate_ram(0, 0).is_err());
        assert!(calculate_ram(4, 5).is_err());
    }
    #[test]
    fn storage_distinguishes_user_available_from_volume_free() {
        let s = calculate_storage("C:\\".into(), 100, 30, 40).unwrap();
        assert_eq!(s.used_bytes, 60);
        assert_eq!(s.user_available_bytes, 30);
        assert_eq!(s.volume_free_bytes, 40);
    }
    #[test]
    fn invalid_storage_inputs_are_rejected() {
        assert!(calculate_storage("".into(), 100, 20, 20).is_err());
        assert!(calculate_storage("C:\\".into(), 0, 0, 0).is_err());
        assert!(calculate_storage("C:\\".into(), 10, 11, 5).is_err());
    }
    #[test]
    fn invalid_storage_paths_are_rejected() {
        assert!(validate_volume_path("").is_err());
        assert!(validate_volume_path("C:\0bad").is_err());
    }
    #[test]
    fn failed_windows_api_call_is_returned_as_an_error() {
        assert!(win32_result(false, "simulated Win32 failure 5")
            .unwrap_err()
            .contains('5'));
        assert!(win32_result(true, "unused error").is_ok());
    }
}
