# Hardware support and telemetry requirements

OverDrive uses Tauri with a Rust backend and React UI. This stack is suitable for a Windows hardware dashboard; replacing it would not make sensor readings more accurate. Accuracy depends on reading real hardware sensors through APIs supported by the GPU, CPU, motherboard, and installed drivers.

## Current support

| Hardware or feature | Current source | Current behavior |
| --- | --- | --- |
| CPU identity, utilization, frequency | Windows APIs and `sysinfo` | Live readings where Windows exposes them |
| CPU package and per-core temperatures | Installer-managed OverDrive Hardware Sensors service using LibreHardwareMonitorLib and PawnIO; read-only loopback JSON at port 8085 | EXE installer includes the driver setup. No per-launch elevation prompt; availability depends on CPU support and machine policy |
| CPU voltage | LibreHardwareMonitor/OpenHardwareMonitor WMI sensors, when available | Optional; values stay unavailable when no matching sensor is exposed |
| RAM and volume capacity | Windows system APIs | Live readings |
| NVIDIA GPU | NVIDIA NVML, dynamically loaded from the installed driver | Live telemetry; each unsupported sensor remains unavailable |
| AMD or Intel GPU | None yet | Not supported; the current WMI class does not provide the GPU sensor telemetry this app needs |
| NVIDIA GPU tuning (fan, clocks, power limit) | NVML control functions from the installed driver | Implemented behind live capability probing: power limit, core/memory clock offsets, and fan control (driver policy, fixed duty, or a custom fan curve applied by a 1 s background engine). Controls the driver refuses (unsupported or needs administrator rights) stay disabled and report the driver's reason |

When no supported provider is available, the app now returns no detected GPU rather than substituting synthetic device names or measurements.

## Refresh rates (live display vs. recording)

OverDrive separates the *display* path from the *recording* path so the UI updates feel instant without distorting measurements, the database, or alerts:

| Path | Cadence | Mechanism |
| --- | --- | --- |
| GPU live display | 100 ms (~10 Hz) UI poll | `get_gpu_sample` command → single live NVML probe only; no buffer push, no persistence, no alert evaluation |
| GPU background sampling | 250 ms (4 Hz) | Automatic background loop started at launch; pushes ring buffers (~1 h of history at 14400 samples) and feeds the smart-alert stream |
| GPU DB persistence | 1 Hz (1 in 4 loop ticks throttled) | Same loop; recorded history cadence unchanged from the earlier 1 s loop |
| Alert rules | 1 Hz | Rules fire on every breaching sample by design, so evaluation is fixed at 1 Hz in the background loop, never per UI fetch |
| CPU live display | 1 s UI poll of a cached sample | A dedicated ~1 Hz sampler thread performs the real measurement (`record_latest`); `get_cpu_sample` returns the cached clone in microseconds, so usage percentages keep their ~1 s measurement window |
| CPU sensor service | 1 second updater | Re-reads the hardware tree and caches JSON; the unelevated app reads the local service |
| RAM display | 1 s | Windows system APIs |
| Overlay HUD | 250 ms | `get_overlay_data` |
| Custom fan curve | 1 s re-apply | Background engine interpolates the curve duty; restores the driver's automatic policy when disabled |

**Latency reality:** a physical fetch latency of 0.1 ms is not achievable with hardware sensors — a single NVML or sensor read itself costs roughly 0.1–0.5 ms and sensor silicon updates at about 10 Hz. OverDrive therefore targets ~100 ms *update* latency (values appear effectively instant) with every individual fetch costing well under 1 ms. UI polls use the side-effect-free paths above, so polling faster never inflates database rows, alert history, or skews CPU-usage measurement windows.

**CPU sensor requirements:** Starting in 0.2.1, use the machine-wide EXE installer. It installs PawnIO and the automatic OverDrive Hardware Sensors service during setup. The UI never launches an elevated helper. See [service installation](SENSOR_SERVICE_INSTALL.md).

## API and driver work needed

- **NVIDIA monitoring:** use the NVML library provided with a compatible NVIDIA driver. NVML is NVIDIA's documented C API for monitoring and management. Some readings depend on GPU model, driver, and operating mode, so the app must preserve unavailable values instead of inventing defaults.
- **AMD monitoring and tuning:** integrate AMD ADLX for supported Windows systems. ADLX provides performance monitoring and GPU tuning interfaces; its SDK and runtime compatibility need to be handled in the application build and installer.
- **Intel monitoring:** evaluate Intel's supported Windows telemetry interfaces for the target GPU generations. Intel Level Zero metrics cover supported Intel processor graphics but require compatible runtime/driver support; metric availability is hardware-dependent.
- **CPU and motherboard sensors:** Windows does not expose every board sensor through a universal built-in API. OverDrive bundles a headless sensor host built on LibreHardwareMonitorLib (`CpuSensorHost.exe`, started automatically and serving loopback-only JSON on port 8085), which supports modern releases where WMI sensor publication is unavailable; a separately installed LibreHardwareMonitor with its Remote Web Server on the same port is also accepted. Some sensors still require administrator rights or its hardware access driver. The app does not read CPUID HWMonitor's private process data; direct CPUID integration would require its licensed SDK.
- **NVIDIA tuning controls (implemented):** OverDrive binds NVML's control functions the same way it binds its read functions — dynamically from the installed driver, with no kernel driver injection. Used functions: `nvmlDeviceSetFanControlPolicy`/`nvmlDeviceSetFanSpeed_v2` (legacy `nvmlDeviceSetFanSpeed` fallback), `nvmlDeviceGpcClkVfOffset`/`MemClkVfOffset` getters and setters, and the power-management-limit family. Before any control is offered, the app probes each one live to build a capability set; the UI disables anything the probe marks unsupported and every failure is translated by an NVML error-code table (`nvml_error_message`, e.g. code 4 → "permission denied — run OverDrive as Administrator"). The custom fan curve runs as a backend engine that re-applies the interpolated duty once per second and restores the driver's automatic fan policy when disabled.
- **Controls, general:** implement a vendor-specific API for every advertised control (AMD ADLX next), query capability limits from the device, verify the result after applying a change, and leave unsupported controls disabled. Fan and overclocking APIs are not interchangeable across vendors or GPU models.

There is no single Windows API that provides CPUID HWMonitor-level sensor coverage and universal fan/overclocking control. Broad coverage requires vendor integrations and, for some motherboard sensors, a sensor driver. The app must report source and availability per reading so a missing sensor is never mistaken for a zero value.
