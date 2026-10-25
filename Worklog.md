# Work Log — GPUControl Pro CPU Telemetry

**Date:** 2026-10-26  
**Workspace:** `I:\PROJECT 0345`  
**Goal:** Diagnose the missing CPU temperature and stale CPU clock readings, make the telemetry path reliable, and build fresh Windows installers.

## Summary

The CPU sensors were available on the machine. GPUControl Pro was failing to recognize LibreHardwareMonitor (LHM)'s live JSON format: LHM labels a sensor's type as `Type`, while the app only accepted `SensorType`. The same live feed contained a Ryzen 7 5700G temperature of 65.4 °C and changing core clock readings up to 4392 MHz.

There was also a startup reliability problem: launching LHM hidden did not trigger the UI checkbox event that starts its web server. The app now has a headless CPU sensor host that uses the bundled LHM hardware library and serves telemetry on loopback. No external API, cloud service, or API key is required.

## Diagnosis

1. Inspected the running application and confirmed the reported state: CPU temperature was unavailable and the displayed fastest core clock was stuck, while other telemetry was updating.
2. Inspected the bundled LibreHardwareMonitor 0.9.6 executable and relevant managed members with the REA reverse-engineering CLI. The UI loads its settings from `LibreHardwareMonitor.config`; the web-server setting is a UI option whose `Changed` callback starts the listener. Loading it as enabled does not itself fire the callback. A hidden launch could therefore remain running without opening the data endpoint.
3. Corrected the settings-file location during the earlier investigation. This alone did not solve startup because the server depends on the UI event.
4. Queried the machine's local `http://127.0.0.1:8085/data.json` endpoint. It returned HTTP 200 and a 70,033-byte LHM sensor tree. I found 20 AMD CPU clock and temperature entries, including `Core (Tctl/Tdie)` at 65.4 °C, physical core clocks up to 4392 MHz, and lower, varying effective clocks.
5. Compared that payload with the Rust deserializer. LHM emits the sensor type in `Type`; the app only mapped `SensorType`. As a result, valid temperature and clock rows were skipped. LHM values can also be strings with units (for example, `65.4 °C`), which the existing numeric-string fallback handles.

## Changes made

### Sensor provider and startup

- Added `apps/backend/tools/CpuSensorHost.cs`, a small headless .NET Framework host using `LibreHardwareMonitorLib.dll`.
- The host opens CPU hardware, updates the hardware tree, returns temperature and clock sensors in the JSON structure the Rust reader consumes, and binds directly to `127.0.0.1:8085` using `TcpListener`. Direct loopback binding avoids HTTP.sys URL reservation requirements for normal per-user installs.
- Compiled `CpuSensorHost.exe` with the Windows .NET Framework C# compiler and included its CLR configuration alongside the existing LHM runtime dependencies.
- Added `apps/backend/src/hardware/sensor_runtime.rs` to locate bundled resources, copy runtime dependencies into the app data folder, launch the host hidden, wait up to 15 seconds for a working feed, and stop the child process on app exit.
- The runtime copy skips the legacy `LibreHardwareMonitor.exe` and its `.exe.config`. Earlier versions may still have the old GUI process open during an in-place update; attempting to overwrite that locked executable could prevent the new host from being copied.
- The provider listens only on loopback. It does not expose sensor data to the network.

### CPU parsing and display data

- Updated `apps/backend/src/hardware/cpu_temperature.rs` to accept both LHM's `Type` field and the headless host's `SensorType` field.
- Retained CPU identity filtering (`/amdcpu/` and `/intelcpu/`) so unrelated board thermal readings are not reported as CPU temperature.
- The CPU sampling code uses available CPU sensor temperatures and the fastest live physical core clock, while preserving per-core clock and temperature data where available.
- Updated CPU commands and frontend CPU data handling to carry the additional readings and distinguish available telemetry from unavailable values.

### Other tracked work in this task

- Improved Windows CPU sampling and voltage handling, and tightened NVIDIA telemetry validation/error reporting.
- Updated CPU, GPU, dashboard, and control-page presentation to use the live readings and show unavailable telemetry explicitly.
- Added hardware support notes in `docs/HARDWARE_SUPPORT.md` and clarified capabilities and limitations in `README.md`.
- Removed the legacy simulated and WMI provider files, plus obsolete frontend npm lockfiles; the repository uses the root pnpm workspace lockfile.
- Added the bundled LibreHardwareMonitor runtime folder to `apps/backend/tauri.conf.json` resources.

## Build and verification

- `cargo check --release` completed successfully.
- `npx tauri build` completed successfully and produced both Windows installers.
- The final installer manifests were checked: both include `CpuSensorHost.exe` and its `.exe.config`; neither includes the C# source file.
- The live LHM feed was read directly and contained the CPU temperature and varying clocks described above. This verifies the machine's sensors and confirms the JSON schema mismatch.
- The C# host compiled and was launched as a process during development. Its response was not independently distinguished from the already active LHM endpoint during the live feed inspection; the new installed app has not yet been run against the host on this machine.
- No automated test suite was run for this work. The release build and the manual, read-only live-feed inspection were the verification performed.

## Installer artifacts

- NSIS: `target/release/bundle/nsis/GPUControl Pro_0.2.0_x64-setup.exe` (11,474,194 bytes; built 2026-10-26 02:26 local time).
- MSI: `target/release/bundle/msi/GPUControl Pro_0.2.0_x64_en-US.msi` (15,798,272 bytes; built 2026-10-26 02:25 local time).

The installers were built fresh with the completed CPU telemetry parser fixes. To apply the new telemetry, close any running instance of GPUControl Pro, run the new installer, and launch the application (as Administrator if LibreHardwareMonitor isn't already running in the background).

## Follow-up Resolution — Root Cause & Approach 1 Implementation

### Root Cause Diagnosed
1. In `apps/backend/src/hardware/cpu_temperature.rs`, `LhmNode` defined `raw_value: Option<f64>`. In live feeds from LibreHardwareMonitor (LHM), `RawValue` is a string with units (e.g. `"58.8 °C"` or `"4467.0 MHz"`). Serde strictly rejected string inputs for `f64`, causing deserialization of the root JSON tree to silently fail and return `None`.
2. When the HTTP feed returned `None`, the backend fell back to WMI queries on `'root/LibreHardwareMonitor'`, which were removed in LHM 0.9.5+, returning `[]` and leaving temperature and fastest core clock as `None`.
3. `select_cpu_readings` lumped temperature and clock sensors together without checking `/temperature/` vs `/clock/` in the sensor identifier, risking low effective clocks being misclassified as temperatures or high temperatures being misclassified as clocks.

### Approach 1 Implementation
- Changed `raw_value: Option<serde_json::Value>` in `LhmNode` to seamlessly accept both numeric floats (from `CpuSensorHost`) and string floats with units (from `LibreHardwareMonitor`).
- Added robust numeric parsing helper `parse_lhm_number` supporting whitespace unit splitting and comma/dot decimal separators.
- Separated temperature (`identifier.contains("/temperature/")`) and clock (`identifier.contains("/clock/")`) filtering.
- Added `core_clocks` to `CpuTemperatureReadings` and mapped physical core clocks from LHM to logical CPU samples in `cpu.rs`.
- Fixed optional chaining on `cpu.coreTemperatures` in `DashboardPage.tsx` and updated test assertions.
- Added 2 new unit tests for LHM live string and host numeric deserialization; all 26 backend tests and 12 frontend vitest tests passed.

## Files of particular interest

- `apps/backend/src/hardware/cpu_temperature.rs` — LHM JSON parsing and CPU sensor selection.
- `apps/backend/src/hardware/sensor_runtime.rs` — provider deployment and process lifecycle.
- `apps/backend/tools/CpuSensorHost.cs` — headless sensor host source.
- `apps/backend/resources/LibreHardwareMonitor/` — bundled compiled host and LHM dependencies.
- `apps/backend/tauri.conf.json` — packaged resource configuration.
- `docs/HARDWARE_SUPPORT.md` — documented hardware support details.
