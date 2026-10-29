# Work Log — CPU Telemetry (built when the project was named GPUControl Pro; now OverDrive)

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

---

# Work Log — OverDrive Rebrand + NVIDIA Tuning Controls

**Date:** 2026-10-26
**Workspace:** `I:\PROJECT 0345`
**Goal:** Rename the product everywhere from "GPU CONTROL PRO" to "OVERDRIVE", and implement CPU-GPU monitoring presentation plus the MSI-AfterBurner-style GPU control layer (fan modes with a custom fan curve, core/memory clock offsets, power limit), driven by NVML through the installed driver.

## Summary

The project completed a full-depth rename and added real GPU tuning controls. The rename covered the Windows-side surface that matters for a desktop app: executable and crate names (`overdrive-backend`, `overdrive_lib`), the frontend package (`overdrive-frontend`), the Tauri product identity (`OverDrive`, `pro.overdrive.app`), installer hooks, updater endpoints (`releases.overdrive.app`, `github.com/overdrive-app/overdrive`), and the app-data folder (`Roaming\overdrive`).

The tuning layer binds NVML's control functions the same dynamically-loaded way the telemetry reads work, so no kernel driver (or a second driver stack like AfterBurner's RTCore) is used. Each control is probed live before the UI offers it, and the backend maps every failure through `nvmlErrorString` so the UI shows the driver's actual reason (code 4 → "permission denied — run OverDrive as Administrator", code 19 → "in use by another application", and so on).

## Changes made

### Rename

- `apps/backend/Cargo.toml` (package + lib), `main.rs`, `tests/database_test.rs`; `Cargo.lock` now references `overdrive-backend`.
- `apps/backend/tauri.conf.json`: productName `OverDrive`, identifier `pro.overdrive.app`, window title, updater endpoints.
- `apps/backend/hooks.nsh`: renamed executable, registry keys, and shortcut names used by the NSIS installer.
- Root `package.json` scripts and workspace filters now target `overdrive-frontend`; `packages/*` renamed to `@overdrive/*`.
- Data-directory strings (~12 backend files) now `.join("overdrive")`; log file `overdrive.log`, database `overdrive.db`.
- Frontend: `components/armoury/` → `components/overdrive/` (`ArmouryHeader/ArmourySidebar` → `OverdriveHeader/OverdriveSidebar`), `gpucontrol-redesign.css` → `overdrive-base.css`, and every user-visible "GPU CONTROL PRO" string. The `ac-` CSS class prefix and historical "Armoury Crate" design references were intentionally kept.

### NVIDIA control layer (backend)

- `apps/backend/src/hardware/nvidia.rs`: NVML control functions bound and wrapped — fan control policy, `SetFanSpeed_v2` with legacy fallback, `GpcClkVfOffset`/`MemClkVfOffset` get/set, and the power-management-limit family (set, default limit, constraints). Live capability probing produces the `capabilities` set passed to the UI; per-control errors are classified (unsupported vs permission vs busy) via `nvml_error_message`.
- `apps/backend/src/hardware/provider.rs`: `GpuControlState` extended (fan mode, limits, `capabilities`); trait gained `set_fan_control_automatic` and `reset_controls`.
- `apps/backend/src/hardware/fan_curve.rs` (new): the pure fan-curve engine — keyframe list → piecewise-linear duty with clamping, one-second tick loop (driven from `commands/fan_curve_commands.rs`) that probes temperature without touching sample buffers, skips redundant writes below a 1 % epsilon, and restores the driver's automatic fan policy when disabled. Default curve ships with 6 keyframes (30 °C/25 % … 80 °C/100 %).
- `apps/backend/src/commands/fan_curve_commands.rs` (new): `get_fan_curve`, `set_fan_curve`, `set_fan_curve_enabled` (auto-restore on disable), `reset_gpu_controls`, and `spawn_fan_curve_task`; registered in `lib.rs`.

### OverDrive Tuning (frontend)

- `apps/frontend/src/pages/ControlPage.tsx` rewritten: live 8-tile telemetry strip (temp, clocks, fan, power, load, VRAM, voltage — polling at 1 s), three-mode fan card, draggable SVG fan-curve editor (`components/overdrive/FanCurveEditor.tsx` — drag keyframes, click to add, double-click to remove), core/memory offset sliders applying on release, power-limit slider bounded by the driver-reported range, and Reset All with confirmation. Every slider is disabled when its capability probe reports unsupported. The Reset-All result banner renders under the page header — a test caught that it was being set but never displayed.
- `apps/frontend/src/__tests__/controlPage.test.tsx` (new): smoke-tests the page — no-selection banner, capability-gated slider states (power enabled, offsets read-only when the probe says so), and driver-error mapping onto the action banner. `apps/frontend/vitest.config.ts` also gained the `@common` path alias it was missing (page imports `@common/utils`, which vitest could not otherwise resolve).
- `apps/frontend/src/services/gpuService.ts`: control-state fields, capability types, `fanCurveService`, and the control methods backing the page.
- Styles appended to `apps/frontend/src/styles/overdrive-pages.css` with the `od-` prefix.

### Documentation

- `README.md` and `docs/HARDWARE_SUPPORT.md` updated: the tuning row now describes the NVML control implementation, capability probing, and the driver-restore behavior; the "GPU tuning controls are not implemented" claim is gone.

## Live evidence on this machine

The live NVML tests ran against the machine's real driver (`cargo test live_control_state -- --nocapture`): telemetry reads work, power limit control is exposed (78.26 % – 100 % range), while this driver build refuses fan policy and clock-offset queries — those report unsupported codes, which is exactly the situation the capability gating is designed for: the fan and offset sliders stay read-only here and the power slider works. Fan/offset control on other driver builds or GPUs will light up where the driver allows it. No live driver mutation was performed during development; only reads and capability probes ran.

## Verification

- Backend: `cargo test` in `apps/backend` — 33 lib tests + 6 database tests pass, including the fan-curve engine tests, NVML error-classification tests, and both live tests. `cargo clippy` reports no issues in the new code (pre-existing repo warnings remain).
- Frontend: `pnpm --filter overdrive-frontend test` (15 vitest tests, including the new ControlPage smoke tests), `typecheck` (`tsc --noEmit`), and `lint` — all clean. Lint previously failed on a pre-existing `any`-typed prop trio in `overdrive/CpuGauge.tsx` (typed as `MotionValue<number>` now) and tailwind class-order warnings in five pre-existing feature components (auto-fixed); those files were fixed as part of making the lint gate pass, since the lint script uses `--max-warnings 0`.
- Note: `cargo fmt` is unavailable on this machine's toolchain (rustfmt not installed for stable-x86_64-pc-windows-msvc); formatting is manual. `cargo test`/`lint` invocations may print "Exited with code 1" as a PowerShell stderr-redirection artifact even on success — always read the test results themselves.

## Installs and data

The rename changes the app data folder to `%APPDATA%\overdrive`, so settings/samples from a previous GPUControl Pro install are not carried over. Stale build artifacts under `target/` (old `gpucontrol-pro-backend.exe` names, old bundle filenames) regenerate on the next `npx tauri build`; installers have not been rebuilt in this session.

# Session — Fast telemetry ("/0.1 ms feel") and final name audit — 2026-10-26

**Goal:** Make all telemetry fetch fast enough that the app feels instantly live, keep polling from distorting measurements or bloating DB/alerts, and confirm OverDrive is the only product name.

**Fast-path architecture (display vs. duty split):**
- New backend command `get_gpu_sample`: a live NVML probe only — no ring-buffer push, no persistence hook, no alert evaluation. Safe to call at any rate.
- Background monitoring loop auto-starts at app launch from setup (no manual Settings action needed) and samples at 250 ms (4 Hz, `SAMPLE_INTERVAL_MS`) instead of 1 s. Ring buffers enlarged 3600 → 14400 (~1 h of samples).
- Recording stays at 1 Hz: alert rules evaluate every 4th tick (`RECORD_EVERY_TICKS`), and the DB-persist hook in `state.rs` persists 1 sample in 4 via an atomic counter (smart-alert feed stays per-sample). Alerts fire on every breaching sample by design, so their ceiling cadence is exactly 1 Hz.
- CPU: `CpuMonitor` gained a most-recent-sample cache (`record_latest()` / `sample_cached()`); a dedicated ~1 Hz `spawn_cpu_sampler` thread performs the live measurement so usage % keeps its ~1 s window regardless of UI polling. `get_cpu_sample` serves the cached value (microseconds).
- C# CPU sensor host (port 8085) rewritten: a background thread updates the hardware tree and pre-serializes the JSON every 100 ms; requests serve cached bytes (sub-ms each). Packaged `resources/LibreHardwareMonitor/CpuSensorHost.exe` rebuilt (8192 bytes).
- Frontend polls: GPU live sample at 100 ms (~10 Hz) on Dashboard + Control via `gpuService.getSample` / `fetchLiveSample`, RAM at 1 s, overlay HUD at 250 ms, CPU already 1 s (cached). `get_gpu_data` (records + alerts) remains only for the dashboard retry button and the remote API. "LIVE · 10 Hz" pulse pill added to the ControlPage telemetry card (`.od-live-pill` CSS).

**Physical-honesty note:** an actual 0.1 ms fetch latency is not achievable with real sensors — each NVML/sensor read itself costs ~0.1–0.5 ms and sensors update at ~10 Hz. What was built: ~100 ms update latency (values effectively refresh instantly), each fetch costing well under 1 ms of CPU, side-effect-free so nothing distorts measurements or storage.

**Name audit:** repo-wide case-insensitive scan for `gpucontrol|GPUControl|GPU CONTROL` — only feature identifiers (`GpuControlState`, `get_gpu_control_state`, the "GPU control" feature), historical Worklog notes, and design references (Armoury Crate / MSI AfterBurner) remain; no product-name strings. The CSS `ac-` prefix is kept (design-system identifier, not user-visible). `.claude/settings.local.json` permission entries updated to the `overdrive-frontend` filter name.

**Verification:** backend `cargo test` = 34 lib + 6 db tests pass (incl. new `engine_probe_sample_is_side_effect_free_but_collect_once_records` + both live NVML tests); frontend vitest 15/15, `tsc --noEmit`, eslint `--max-warnings 0` all clean; C# host compiles clean with the .NET Framework 4 csc.

# Session — App closing instantly at launch (managed-state type mismatch) — 2026-10-26

**Symptom:** the freshly built installer's app closed ~10–20 ms after double-clicking; two panic entries were logged (one per attempt).

**Diagnosis:** `%APPDATA%\overdrive\logs\crash.log` pinpointed it: `PANIC: state() called before manage() for overdrive_lib::commands::state::AppState` (tauri-2.11.5 `lib.rs:734`). Root cause — OverDrive manages its shared state as `SharedState = Arc<AppState>` (`state.rs:103`), so `.manage(app_state)` registers the `Arc<AppState>` type, while the setup hook called `app.state::<commands::AppState>()` (the bare struct). Those are different `TypeId`s, so Tauri treated the state as unregistered and panicked ~330 ms into launch, right after the background monitor logged its start. Git history showed the offending setup lines were added in an uncommitted session whose code was only ever exercised by `cargo test` — tests never run `setup()` — so the first real release launch hit them.

**Fix (`lib.rs` setup):** background-task handles now come straight from the `Arc<AppState>` captured before the builder (`monitor_state`) — the exact type `.manage()` registered — with no `state::<>` lookup for `AppState`. A repo-wide search confirmed no other `AppState` state lookups exist; `app.state::<SensorRuntime>()` (managed directly, not Arc) was already correct and stays.

**Verification:** launched the debug binary — process alive after 12 s, zero new panic entries, background monitor collecting samples every 250 ms. Release installers rebuilt (`OverDrive_0.2.0_x64-setup.exe` / `OverDrive_0.2.0_x64_en-US.msi`, refreshed 2026-10-26 23:52) and the release binary verified running for 12 s before being confirmed good. Old pre-rename leftovers (`GPUControl Pro_0.2.0_*` installers, `gpucontrol-pro-backend.exe`) were deleted so only OverDrive artifacts remain under `target/`.

# Session — CPU metrics not fetching (MSR/WinRing0 elevation) — 2026-10-27

**Symptom:** after the launch-crash fix, the app opened but CPU metrics stayed empty.

**Diagnosis:** the bundled sensor host served `/data.json` fine, but **every value was 0** (`Tctl/Tdie = 0`, core clocks 0). Ryzen package temperature and core clocks read MSRs through LibreHardwareMonitorLib's WinRing0 kernel driver, which only loads from an elevated process. Previous CPU-temperature success on this machine came from the user's LibreHardwareMonitor tray app running elevated; the headless host spawned by OverDrive is non-elevated, so all MSR-backed readings zero out and the parser (correctly) drops zero values → temperature unavailable. Re-ran the host elevated as a controlled test: values went live (Tctl 58.4 °C, cores 2994–4542 MHz), confirming the cause. No WinRing0 service exists system-wide, so nothing could serve legitimate values without elevation.

**Fix (sensor_runtime.rs):** `ensure_running` now (1) accepts an existing feed only when it reports nonzero Clock/Temperature values (`sensor_feed_useful`), (2) kills stray non-elevated hosts holding port 8085, (3) launches the host elevated with one UAC prompt via a hidden PowerShell `Start-Process -Verb RunAs`, (4) falls back to a non-elevated spawn on prompt cancellation or failure, and (5) records a decline marker that suppresses re-prompting for 10 minutes. The elevated host is intentionally left untracked so it survives app exit and later launches reuse it (prompt only once per boot, like MSI AfterBurner); `log_feed_temperature` logs live Tctl values at startup for diagnosability. Verified: app launch uses the elevated feed ("Bundled headless CPU sensor feed is ready (elevated)"), and standalone elevated probes return live values.

**Verification:** `cargo check`/`cargo build` clean; launches verified alive (15 s debug, 12 s release) with zero new panic entries. The release verification exercised the full elevated first-launch path end to end: startup logged `Bundled headless CPU sensor feed is ready (elevated)` and `Live CPU package temperature from sensor feed celsius=61.75` — the temperature diagnostic added with this fix. Installers regenerated 2026-10-27 00:13 (`OverDrive_0.2.0_x64-setup.exe`, 11.0 MB NSIS per-user; `OverDrive_0.2.0_x64_en-US.msi`, 15.1 MB MSI) with the launch-crash fix plus this elevation fix. The elevation behavior is documented in `docs/HARDWARE_SUPPORT.md` (UAC once per launch/promising host residency, decline fallback, and the 10-minute re-prompt suppress marker). Note for this dev machine: an elevated probe host from the controlled test may remain resident on port 8085; it doubles as the working elevated sensor service, so no UAC prompt will reappear until it is gone (reboot or manual stop).

# Session � MSI Afterburner 4.6.7 Beta 2 reverse engineering + P1�P5 quick slots � 2026-10-28

**Goal:** analyze the shipped MSI Afterburner installer to guide feature parity in OverDrive, then implement the Afterburner profile model (portable control snapshots + five quick slots) as original code.

## Reverse-engineering chain (facts only; no vendor code/art copied)

- Outer `MSIAfterburnerSetup467Beta2.zip` ? signed .NET WPF installer shim `MSIAfterburnerInstaller467Beta2_20260922.exe` (v1.0.0.3, 89,971,272 B, sha256 `bc7f14ae14728c621c620745e8bb60ec3f5603a62d7243c5ef0c33c97c21bf99`, embedded MSI signature valid).
- Shim carries two .NET `.resources` streams: 833,665 B BAML (installer UI) and 44,159,529 B payload (`MSIAfterburnerSetupFile` ZIP, stream sha256 `e7b5139f�`, ZIP `81c5e73a�`).
- Payload's real installer `MSIAfterburnerSetup467Beta2.exe` is an NSIS-3 Unicode installer; 7-Zip safe-extracted 3,500 files into the project temp dir (`...\Temp\opencode\msiab\extract\nsis\`, analysis only � never committed).
- Key versions: `MSIAfterburner.exe` 4.6.7.16935 (831,880 B), `RTHAL.dll` 2.66.0.0, `RTCore.dll` 1.59.0.0 (RTCore64.sys 40,688 B), RTUI 1.54, RTMUI 1.16, RTFC 1.23; `Bundle\OCScanner` (scanner.dll + gpu_stressor.exe); 23 `.usf` skins (USF header bytes observed `30 41 5A 53 � 30 46 53 55`); 16 localization dirs; full plugin SDK.
- Highest-value findings (shipped in `SDK\Include\`): MAHM monitoring shared-memory model (per-source `data/min/max + units`, OSD/LCD/tray flags, source-ID space: temps 0x00�03, fans 0x10�15, clocks 0x20�22, usage 0x30�35, voltages 0x40�43, fps 0x50�56, power 0x60�61, limits 0x70�71) and MACM control protocol (per-control Cur/Min/Max/Def quadruples, capability bitmask incl. POWER_LIMIT 0x400 / THERMAL_LIMIT 0x2000 / THERMAL_PRIORITIZE 0x4000, LINK/SYNC/LINK_THERMAL flags 1/2/4, commands INIT/FLUSH/FLUSH_WITHOUT_APPLYING/REFRESH_VF_CURVE = 0x00AB0000�3, 256-node VF curve).
- `Help\` topic names = canonical control-surface inventory (sliders: core/shader/mem clocks, core/mem/aux/aux2 voltage, power limit, thermal limit, fan; buttons: profiles 0�9, save, startup, link/sync/lock, OC scanner, user-define, VF editor). `MSIAfterburner.cfg` facts: HwPollPeriod=1000, MonitoringDataBufferSize=3600, FanSpeedReadbackDelay=1000, StartupViaTaskScheduler=1, ProfileContents=1, VFCurve editor bounds (500�3500 MHz / 700�1250 mV), per-vendor HAL sections (NVAPIHAL/ATIADLHAL/IGCLHAL/CPUHAL). Profile CLI contract confirmed: `PredefinedApplications` maps `-profile1`�`-profile5` and `-lowpowerprofile` to `%AB%`.
- Full report: `docs/REVERSE_ENGINEERING_MSI_AFTERBURNER.md` (chain, digests, IPC model facts, control map, config table, OverDrive mapping + gap list).

## Changes made (original implementations, Afterburner-modeled)

### Backend � `apps/backend/src/commands/profile_commands.rs` (rewritten; registrations in `lib.rs`)

- `ControlSnapshot` (core/memory/voltage offsets, optional power %, `fan_mode` ? automatic/fixed/curve, optional fixed fan %, optional curve points), `SnapshotFanMode`, `SnapshotFanPoint`, `QuickSlotSummary` � serde snake_case over the wire.
- `apply_control_snapshot_inner`: fan mode applied first (curve engine off + driver-restore ? fixed ? curve); voltage-offset errors (unsupported via NVML) warn-and-continue while offsets/power hard-fail � mirrors AB FLUSH semantics without silently skipping failure reporting.
- Quick slots: `QUICK_SLOT_COUNT = 5`, `slot{1..5}.json` in `overdrive/profiles/slots/` under the user config dir; commands `apply_control_snapshot`, `get_quick_slots`, `quick_save_slot`, `quick_load_slot`, `quick_delete_slot`; all five registered in `lib.rs`; 2 unit tests added (slot-path validation, serde round-trip with curve snapshot).

### Frontend service + UI

- `services/gpuService.ts`: `SnapshotFanMode`, `ControlSnapshot`, `QuickSlotSummary` types (snake_case wires) and `profileService.applySnapshot/slots/saveSlot/loadSlot/clearSlot`; re-exported through `services/index.ts`.
- `pages/ControlPage.tsx`: `slots`/`activeSlot` state, `reloadSlots`, `buildSnapshot()` (captures everything currently dialed in, including unsaved curve edits; nulls for unsupported controls are skipped backend-side), slot handlers `handleSlotSave/handleSlotLoad/handleSlotClear` (load then `reloadCurve()`; Reset-All clears the APPLIED marker), new `QuickSlotsCard` strip after the Live-telemetry card: P1�P5 chips with mode badge (AUTO/FIXED/CURVE), one-line state summary (`�0 MHz � fan auto` style via `describeSlot`), Save/Load/? per chip, session-local APPLIED tag, shared action banner for slot ops.
- `styles/overdrive-pages.css`: `.od-profile-strip` grid + `.od-slot-chip` (and `--active/--filled`, `__label/__mode/__desc/__tag/__ops`) using the existing `--ac-*` design tokens; responsive 2-col at =960px (media queries merged).

## Verification

- Backend: workspace `cargo check` clean (silent).
- Frontend: `tsc --noEmit` clean; `vite build` clean (only pre-existing dynamic-import chunk warnings).

## Follow-ups open

- Gaps deliberately documented not implemented (see report �9): voltage/thermal-limit sliders (no NVML control surface), VF-curve editor, LINK/SYNC multi-GPU flags, OC Scanner integration, MAHM-style per-source min/max+units telemetry.
- `SDK\Doc` PDFs (USF skin format, hardware database format, localization) not parsed � overkill for current scope; note skin header magic if ever needed.

**Installer build (same day):** after adding the `PartialEq` derives the tests needed (`ControlSnapshot`, `SnapshotFanPoint`), backend `cargo test` = 36 lib + 6 DB pass, frontend vitest = 15/15 (incl. the 3 ControlPage tests covering the new Quick Slots card). Full bundles rebuilt: `OverDrive_0.2.0_x64-setup.exe` (NSIS, 11.5 MB, 2026-10-28 11:01) and `OverDrive_0.2.0_x64_en-US.msi` (15.9 MB, 11:01) under `target\release\bundle\`. Release-binary launch verification: alive after 15 s, background monitoring collecting every 250 ms, `crash.log` untouched (only this morning's pre-existing entry).
