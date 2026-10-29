# Reverse Engineering Notes — MSI Afterburner 4.6.7 Beta 2 installer

**Date:** 2026-10-28
**Analyst:** OverDrive project (agent-assisted, REA CLI v6.7.0 + 7-Zip 26.05)
**Source artifact:** `MSIAfterburnerSetup467Beta2.zip` (user-provided download)

> **IP stance.** This report records *facts* — file names, sizes, digests, configuration
> values, wire-format constants and functional behavior — gathered to document the
> Afterburner control model for interoperability documentation. No MSI binary, code,
> skin, artwork, or documentation text has been copied into OverDrive or into this
> report. All OverDrive implementations are original. Vendor artifacts analyzed here
> remain in the project temp directory and are not redistributed.

---

## 1. Extraction chain

The shipped archive is a three-layer package: a ZIP wrapping a signed .NET WPF
installer shim, which carries the real NSIS installer as two embedded .NET
`.resources` streams.

| # | Layer | What it is | Facts |
|---|-------|------------|-------|
| L0 | `MSIAfterburnerSetup467Beta2.zip` | Outer ZIP | contains one item, see L1 |
| L1 | `MSIAfterburnerInstaller467Beta2_20260922.exe` | .NET WPF installer shim, PE32/CLI, v1.0.0.3 | 89,971,272 B; sha256 `bc7f14ae14728c621c620745e8bb60ec3f5603a62d7243c5ef0c33c97c21bf99`; embedded MSI signature present and valid |
| L2a | `.resources` stream 1 | BAML (compiled WPF UI for the installer, localized) | 833,665 B |
| L2b | `.resources` stream 2 | `MSIAfterburnerSetupFile` payload ZIP | 44,159,529 B; sha256 `e7b5139f…` (stream), ZIP digests `81c5e73a…` |
| L3 | `MSIAfterburnerSetup467Beta2.exe` | NSIS-3 Unicode installer | installs the product; safe-extracted 3,500 files |

Extraction method: REA CLI for identity/signed-managed inspection and member
inventory; 7-Zip for the ZIP and NSIS layers. Extraction was performed into the
project temp directory `...\Temp\opencode\msiab\` (kept for analysis only; not
committed). A full file inventory of the NSIS extraction is retained at
`...\msiab\extract\nsis\_inventory.txt` (3,500 entries, 200,912 B).

## 2. Product binary inventory

Root of the install tree (paths relative to `...\msiab\extract\nsis\`):

| File | Version | Size (B) | sha256 (when noted) |
|------|---------|----------|---------------------|
| `MSIAfterburner.exe` | 4.6.7.16935 | 831,880 | `6ef4d1f68bd91641ceb04a3ce923a231c7cfe0d62b28b9328ab80803481a12e3` |
| `RTHAL.dll` (HAL) | 2.66.0.0 | 980,480 | `9251cdbc614f7dc42b9052b706bab96d0d2a083b98dba32f88965fb6b69f9cbe` |
| `RTCore.dll` | 1.59.0.0 | 263,168 | `f38831100cca8c9387bb0382b4b7374a0ee2bea0578023e0355591b3c31e6592` |
| `RTCore64.sys` | — | 40,688 | `a4f44e267698d47f6a905e96b356582b4ee9cf4049e8c792ffda1b7356e68d35` |
| `RTCore32.sys` | — | 35,056 | — |
| `RTUI.dll` | 1.54.0.0 | 417,280 | — |
| `RTMUI.dll` | 1.16.0.0 | 112,640 | — |
| `RTFC.dll` | 1.23.0.0 | 76,288 | — |
| `MSIAfterburner.cfg` | — | 8,111 | — |
| `RTCore.cfg` | — | 7,739 | — |
| `MSIAfterburner.dat` | — | 36,387 | — |
| `MSIAfterburner.oem` / `.oem1` / `.oem2` | — | 95,247 / 98,416 / 11,112 | — |
| `hidapi.dll` | — | 139,776 | — |
| `LogitechLcd.dll` | — | 205,592 | — |

Top-level directories: `$PLUGINSDIR`, `Bundle`, `Doc`, `Graphics`, `Help`,
`Legacy`, `Localization` (16 subdirs: BUL, CHN, DUT, FR, GER, ID, ITA, JP, KOR,
POL, PTBR, RUS, SPA, TC, TR, UKR), `Messages`, `Plugins`, `Redist`, `SDK`,
`Skins` (23 `.usf` files), `Sound`.

## 3. Control / monitoring IPC model (SDK headers)

The most valuable finding for OverDrive: Afterburner documents its complete
control and monitoring surface as shared-memory protocol definitions shipped in
`SDK\Include\`. These are functional interface facts (constants, structure
layout), not code we reuse.

### 3.1 Files

| Header | Size (B) | Role |
|--------|----------|------|
| `MAHMSharedMemory.h` | 9,064 | Monitoring ("M"AHM = Hardware Monitoring): read-only telemetry publication |
| `MACMSharedMemory.h` | 12,956 | Control ("M"ACM = Hardware Control): control application protocol |
| `MSIAfterburnerExports.h` | 5,101 | Exports of `MSIAfterburner.exe` (macro `MSIAFTERBURNER_API`, `extern "C"` + dllexport/dllimport) |
| `MSIAfterburnerMonitoringSourceDesc.h` | 2,422 | Monitoring-source descriptor used by plugin hosts |

### 3.2 MAHM — monitoring publication

- Shared-memory header struct followed by per-source monitoring entries.
- Each entry carries the current `data`, plus session `min`/`max`, a unit type,
  and a flags field.
- Entry visibility flags: `…_FLAG_SHOW_IN_OSD = 0x1`, `SHOW_IN_LCD = 0x2`,
  `SHOW_IN_TRAY = 0x4`.
- Monitoring-source ID space (selected groups):

| Range | Sources observed in header |
|-------|---------------------------|
| `0x00–0x03` | GPU / PCB / memory / VRM temperature |
| `0x10–0x15` | fan speed, fan tachometer (×3 channels) |
| `0x20–0x22` | core clock, shader clock, memory clock |
| `0x30–0x35` | GPU usage, memory usage, framebuffer usage, video-engine usage, bus usage, per-process memory usage |
| `0x40–0x43` | GPU voltage, aux voltage, memory voltage, aux2 voltage |
| `0x50–0x56` | framerate, frametime, min/avg/max framerate, 1% and 0.1% low |
| `0x60–0x61` | relative power, absolute power |
| `0x70–0x71` | GPU temperature limit, GPU power limit |

Per-entry `data + min/max + units` is the shape OverDrive should mirror as it
deepens monitoring (OverDrive currently publishes live `data`-only samples).

### 3.3 MACM — control application protocol

- Header-level flags for multi-GPU coordination: `LINK = 0x00000001`,
  `SYNC = 0x00000002`, `LINK_THERMAL = 0x00000004`.
- Per-control capability flags in one bitmask:

| Flag | Value | Flag | Value |
|------|-------|------|-------|
| `CORE_CLOCK` | 0x000001 | `CORE_VOLTAGE_BOOST` | 0x000080 |
| `SHADER_CLOCK` | 0x000002 | `MEMORY_VOLTAGE_BOOST` | 0x000100 |
| `MEMORY_CLOCK` | 0x000004 | `AUX_VOLTAGE_BOOST` | 0x000200 |
| `FAN_SPEED` | 0x000008 | `POWER_LIMIT` | 0x000400 |
| `CORE_VOLTAGE` | 0x000010 | `CORE_CLOCK_BOOST` | 0x000800 |
| `MEMORY_VOLTAGE` | 0x000020 | `MEMORY_CLOCK_BOOST` | 0x001000 |
| `AUX_VOLTAGE` | 0x000040 | `THERMAL_LIMIT` | 0x002000 |
| | | `THERMAL_PRIORITIZE` | 0x004000 |
| | | `AUX2_VOLTAGE` | 0x008000 |

- Each control entry exposes a current / minimum / maximum / default
  (`Cur/Min/Max/Def`) quadruple — this is why AB sliders always know their
  driver-reported range. OverDrive already mirrors the *offset-limit* part via
  NVML (`core_clock_offset_limit_mhz` etc.).
- Command codes: `INIT = 0x00AB0000`, `FLUSH = 0x00AB0001`,
  `FLUSH_WITHOUT_APPLYING = 0x00AB0002`, `REFRESH_VF_CURVE = 0x00AB0003`
  (the `AB` mnemonic is Afterburner's own marker).
- VF curve: a fixed-size curve-node array (256 nodes observed in the layout)
  with per-node lock semantics, edited by AB's Voltage/Frequency editor.

## 4. UI control surface (from `Help\` topic names)

The Help directory enumerates every skinned control; this is the complete
canonical inventory of AB's main-panel surface:

**Sliders:** core clock, shader clock, memory clock, core voltage, memory
voltage, aux voltage, aux2 voltage, power limit, thermal limit, fan speed.
**Buttons:** apply, close/defaults, detach, Fahrenheit toggle, fan channels
1–4, fan auto, fan sync, help, hide, info, link, link-thermal, lock, min, OC
Scanner, prioritize, profile 0–9, save, setup, show-GPU-panel,
show-thermal-panel, show-voltage-panel, show-voltage-frequency-editor,
startup, sync, test, user-define. Plus monitoring window and schedule map
placeholders.

Notes that map onto OverDrive decisions:

- `BUTTON_PROFILE0` = reset-to-default slot; `BUTTON_PROFILE1..5` = the five
  quick profile slots. `[PredefinedApplications]` in `MSIAfterburner.cfg`
  confirms the CLI contract: `-profile1` … `-profile5` and
  `-lowpowerprofile` (an external "load profile" entry points `%AB%` with that
  command line).
- The single-GPU surface does not expose the multi-GPU flags: AB renders
  LINK/SYNC/LINK_THERMAL controls when multiple identical GPUs exist. For our
  single-GPU app these are out of scope (documented, not implemented).

## 5. Configuration facts (`MSIAfterburner.cfg` defaults)

Facts likely to inform OverDrive tuning/monitoring design (selected):

| Key | Default (shipped) | Meaning |
|-----|-------------------|---------|
| `HwPollPeriod` | 1000 | telemetry poll period, ms |
| `MonitoringDataBufferSize` | 3600 | per-source history depth |
| `FanSpeedReadbackDelay` | 1000 | fan % readback delay, ms |
| `StartupViaTaskScheduler` | 1 | startup uses Task Scheduler mechanism |
| `StartupDelay` | 0 | startup delay, ms |
| `MMIOUserMode` | 1 | user-mode MMIO backend (RTCore) enabled |
| `HAL` / `HALErrorReporting` | 1 / 0 | RTHAL layer enabled |
| `Driver` | 1 | driverctrl handshake enabled (`HandshakePeriod` = 10000) |
| `Sync` / `Link` | 1 / 1 | multi-GPU sync/link defaults |
| `ProfileContents` | 1 | which domains a profile captures |
| `RestoreAfterSuspendedMode` | 1 | re-apply settings after resume |
| `PauseInSuspendedMode` | 1 | pause polling during sleep |
| `DataFilteringThreshold` | 10 | measurement smoothing threshold |
| `I2CCache` | 1 | I²C read caching |
| `VFCurveEditorMinFrequency/MaxFrequency` | 500 / 3500 | VF editor x-bounds, MHz |
| `VFCurveEditorMinVoltage/MaxVoltage` | 700 / 1250 | VF editor y-bounds, mV |
| `UnlockVoltageControl` / `UnlockVoltageMonitoring` | 0 / 0 | expert voltage unlock |
| `ForceConstantVoltage` | 0 | voltage override mode |

Per-vendor HAL sections confirm the backend enumeration used on this machine:
`[NVAPIHAL] Enable=1` (+ `GenericMemoryUsageMonitoring=1`,
`UnifiedMemoryUsageMonitoring=1`, `NormalizedTotalPowerMonitoring=1`,
`PS20RelVoltageControl=1`), `[ATIADLHAL]`, `[IGCLHAL]`, `[UnifiedHAL]`,
`[CPUHAL]` — i.e., AB uses a HAL layer over NVAPI/ADL/IGCL with a task-scheduler
startup, not elevated UI persistence.

`RTCore.cfg` is the device-identifier database (NVIDIA GPU families by PCI
device-id — e.g., `[GPU_10DE]` mapping families to device-id ranges), used by
RTCore to route per-family register access. This is what the vendor's
"Hardware database format reference.pdf" documents.

## 6. USF skin format (observed, parsed no further)

Skins are vendor `USF` container files; observed header bytes
begin `30 41 5A 53` ("`0AZS`") followed shortly by `30 46 53 55` ("`0FSU`"),
best interpreted as `USF0` markers in little-endian byte order. The official format
reference (`SDK\Doc\USF skin format reference.pdf`, plus
`Localization reference.pdf` and `Hardware database format reference.pdf`) is
present in the package if a parser is ever warranted. **OverDrive does not
adopt the USF system** — we keep our own tokenized CSS design system — so the
format is documented here only for completeness.

## 7. OC Scanner bundle

`Bundle\OCScanner\`: `MSIOCScanner_x64.exe` (97,280 B), `scanner.dll`
(626,688 B), `gpu_stressor.exe` (3,397,120 B), plus `Help` topics for
`BUTTON_OCSCANNER` / `BUTTON_TEST` / `BUTTON_USER_DEFINE`. This is the scan-then-apply
workflow AB wraps. Out of OverDrive scope for now; noted as a future "stress test"
integration point only.

## 8. What OverDrive implemented from these findings

Modeled on the Afterburner profile/control model, shipped as original code:

- **Portable control snapshot** — `apps/backend/src/commands/profile_commands.rs`:
  `ControlSnapshot` (core/memory/voltage offsets, optional power limit %,
  `fan_mode` ∈ {automatic, fixed, curve}, optional fixed fan %, optional curve
  points) with `serde` snake_case wire names matching the frontend.
- **Apply semantics** — `apply_control_snapshot_inner`: fan mode applied first
  (engine off + restore automatic → fixed → curve), unsupported controls
  (voltage on this NVIDIA path) degrade to a logged warning while offsets/power
  hard-fail on driver error. This mirrors AB's FLUSH behavior (apply what the
  driver accepts, never silently skip success reporting).
- **P1–P5 quick slots** — `QUICK_SLOT_COUNT = 5`, files `slot1..5.json` under
  `overdrive/profiles/slots/` in the user's config dir; commands
  `get_quick_slots`, `quick_save_slot`, `quick_load_slot`, `quick_delete_slot`,
  `apply_control_snapshot` (registered in `apps/backend/src/lib.rs`); unit tests
  included. (AB's equivalent slots live as ProfileContents-flagged registry
  state; we chose plain JSON files for transparency and per-slot timestamps.)
- **Frontend** — `gpuService.ts` (`profileService.applySnapshot/slots/saveSlot/
  loadSlot/clearSlot`, typed `ControlSnapshot`, `QuickSlotSummary`,
  `SnapshotFanMode`), `ControlPage.tsx` Quick Slots strip (P1–P5 chips with
  Auto/Fixed/Curve mode badge, one-line state summary, Save/Load/✕ per chip,
  session-local APPLIED marker) and `overdrive-pages.css` slot-chip styles.
- **Verification:** `tsc --noEmit` clean; `vite build` clean; `cargo check`
  workspace-wide clean.

## 9. Remaining gaps vs the Afterburner model (documented, deliberately open)

| AB capability | OverDrive status | Reason / route |
|---------------|------------------|----------------|
| Voltage control (core/mem/aux/aux2) + boosts | not implemented | NVML exposes no such control; would require RTCC/kernel MMIO — out of scope, slider stays placeholder-disabled |
| Thermal limit slider + prioritize | not implemented | NVML exposes temperature-limit read only; target-setting is power-limit-only |
| VF curve editor (256 nodes, lock, refresh) | not implemented | needs curve-object control (no NVML offset-grid equivalent); candidate future feature via NVAPI direct if ever pursued |
| Shader clock slider | not implemented | legacy domain, absent on modern drivers |
| LINK / SYNC / LINK_THERMAL flags | not implemented | single-GPU application by design |
| OC Scanner (scan + stressor) | not implemented | separate bundled tool in AB; possible future wrapper |
| MAHM per-entry min/max + units telemetry | partial | OverDrive live samples carry `data`-only; min/max tracking exists only where computed client-side |
| Skins/localization system | not applicable | OverDrive uses its own design-token CSS |
| 10 profile slots (0–9) | 5 quick slots + legacy named profiles | Afterburner's 5-slot CLI contract replicated; 6–9 are shell-button slots without documented CLI |

## 10. Provenance

- Analysis ran exclusively against the user's downloaded archive and the safe
  extraction in the project temp directory; every digest above was computed
  locally (`Get-FileHash`, REA evidence hashing).
- Re-verify any fact with:
  `Get-FileHash -LiteralPath <extract-path> -Algorithm SHA256`.
- Vendor archive and extraction remain un-committed, local, and can be deleted
  after the findings are reviewed.
