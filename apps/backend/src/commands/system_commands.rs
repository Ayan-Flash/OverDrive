use crate::hardware::system_telemetry::{read_ram, read_storage, RamTelemetry, StorageTelemetry};
use crate::utils::error::{AppError, AppResult};

#[tauri::command]
pub fn get_ram_telemetry() -> AppResult<RamTelemetry> {
    read_ram().map_err(AppError::Internal)
}

#[tauri::command]
pub fn get_storage_telemetry() -> AppResult<Vec<StorageTelemetry>> {
    // Capacity is refreshed on demand; the dashboard polls this command every 60 seconds.
    #[cfg(windows)]
    let volumes: Vec<String> = {
        let mask = get_logical_drives();
        if mask == 0 {
            return Err(AppError::Internal(
                std::io::Error::last_os_error().to_string(),
            ));
        }
        (0..26)
            .filter_map(|index| {
                if mask & (1 << index) != 0 {
                    Some(format!("{}:\\", (b'A' + index as u8) as char))
                } else {
                    None
                }
            })
            .collect()
    };
    #[cfg(not(windows))]
    let volumes: Vec<String> = Vec::new();
    Ok(volumes
        .iter()
        .filter_map(|volume| match read_storage(volume) {
            Ok(data) => Some(data),
            Err(error) => {
                tracing::debug!(volume, %error, "Storage volume unavailable");
                None
            }
        })
        .collect())
}

#[cfg(windows)]
fn get_logical_drives() -> u32 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetLogicalDrives() -> u32;
    }
    // SAFETY: GetLogicalDrives takes no arguments and returns the drive bitmask.
    unsafe { GetLogicalDrives() }
}
