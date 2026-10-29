# CPU monitoring and single-window launch

## Observed causes

- Native CPU sampling waited for HTTP/WMI sensor calls; those slow queries delayed usage updates.
- Frontend polling could overlap, and errors left old numbers displayed as live.
- The elevated helper launch did not request a hidden window or tie helper lifetime to OverDrive.
- Bundled LibreHardwareMonitorLib is 0.9.6. Its CPU access uses PawnIO, not WinRing0. An elevated probe returned zero temperature with PawnIO absent. Administrator access alone did not resolve it.

## Changes

Native CPU sampling and slow sensor polling now run on separate workers. The UI polls without overlapping requests, identifies stale samples, and shows actual timestamped CPU history. The helper is compiled as a Windows GUI executable, launched hidden on both paths, and holds its parent process handle to exit with OverDrive. Process-name-wide termination was removed. Missing driver status is included in diagnostics and logged.

The user explicitly approved installing PawnIO. The installer embedded in the existing LibreHardwareMonitor package was extracted without launching its UI, and its Authenticode signature was verified as valid (namazso.eu). SHA-256: `A3A46226C5E2824F4CDD42BE0EECBABFC672C86F7889710F5AB1E6AD385B47A0`. Installer exit code was 0; installed version is 2.1.0.0.

## Verification

- Live elevated helper returned 66.13 degrees Celsius after driver installation.
- Helper main-window handle was zero, and it exited within five seconds when its parent exited.
- Frontend suite: 20 tests passed; full backend suite: 37 unit tests and 6 database integration tests passed. TypeScript checking passed.
- Frontend production build passed. Source-level dashboard evaluation passed after correcting full-width chart placement. Browser screenshot verification was unavailable.
- Optimized standalone executable built successfully with `cargo build -p overdrive-backend --release --features tauri/custom-protocol`: `target/release/overdrive-backend.exe`. Bundled helper SHA-256 matches its rebuilt source artifact. An optional later repeat probe did not return and is not counted as validation.

## Reference and scope

The existing `REVERSE_ENGINEERING_MSI_AFTERBURNER.md` was reused as reference. No new REA binary evidence was produced: REA tools were absent and npm acquisition failed with DNS resolution errors. These changes are original OverDrive implementation, not a complete reproduction of all Afterburner features.

Upstream evidence for driver dependency: https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LibreHardwareMonitor/UI/MainForm.cs and https://github.com/LibreHardwareMonitor/LibreHardwareMonitor/blob/v0.9.6/LibreHardwareMonitorLib/PawnIo/PawnIo.cs

Rebuild the helper with `apps/backend/tools/build-sensor-host.ps1`. A new Windows installation still needs PawnIO installed once and must accept the sensor elevation prompt for privileged CPU sensors. The app does not silently install a kernel driver.
