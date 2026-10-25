# Hardware support and telemetry requirements

GPUControl Pro uses Tauri with a Rust backend and React UI. This stack is suitable for a Windows hardware dashboard; replacing it would not make sensor readings more accurate. Accuracy depends on reading real hardware sensors through APIs supported by the GPU, CPU, motherboard, and installed drivers.

## Current support

| Hardware or feature | Current source | Current behavior |
| --- | --- | --- |
| CPU identity, utilization, frequency | Windows APIs and `sysinfo` | Live readings where Windows exposes them |
| CPU package and per-core temperatures | LibreHardwareMonitor local JSON endpoint, with legacy WMI fallback | Optional; package and individually labeled core sensors appear when exposed. LibreHardwareMonitor must be running with its Remote Web Server enabled (default `http://127.0.0.1:8085/data.json`) |
| CPU voltage | LibreHardwareMonitor/OpenHardwareMonitor WMI sensors, when available | Optional; values stay unavailable when no matching sensor is exposed |
| RAM and volume capacity | Windows system APIs | Live readings |
| NVIDIA GPU | NVIDIA NVML, dynamically loaded from the installed driver | Monitoring only; each unsupported sensor remains unavailable |
| AMD or Intel GPU | None yet | Not supported; the current WMI class does not provide the GPU sensor telemetry this app needs |
| GPU fan, clock, power, and voltage tuning | None in the Windows provider | Not supported; the UI reports this instead of accepting changes that cannot be applied |

When no supported provider is available, the app now returns no detected GPU rather than substituting synthetic device names or measurements.

## API and driver work needed

- **NVIDIA monitoring:** use the NVML library provided with a compatible NVIDIA driver. NVML is NVIDIA's documented C API for monitoring and management. Some readings depend on GPU model, driver, and operating mode, so the app must preserve unavailable values instead of inventing defaults.
- **AMD monitoring and tuning:** integrate AMD ADLX for supported Windows systems. ADLX provides performance monitoring and GPU tuning interfaces; its SDK and runtime compatibility need to be handled in the application build and installer.
- **Intel monitoring:** evaluate Intel's supported Windows telemetry interfaces for the target GPU generations. Intel Level Zero metrics cover supported Intel processor graphics but require compatible runtime/driver support; metric availability is hardware-dependent.
- **CPU and motherboard sensors:** Windows does not expose every board sensor through a universal built-in API. CPU temperature now reads LibreHardwareMonitor's loopback-only JSON endpoint, which supports modern releases where WMI sensor publication is unavailable, then falls back to legacy WMI. LibreHardwareMonitor must be separately installed and running with its Remote Web Server enabled; some sensors also require administrator rights or its hardware access driver. The app does not read CPUID HWMonitor's private process data; direct CPUID integration would require its licensed SDK.
- **Controls:** implement a vendor-specific API for every advertised control, query capability limits from the device, verify the result after applying a change, and leave unsupported controls disabled. Fan and overclocking APIs are not interchangeable across vendors or GPU models.

There is no single Windows API that provides CPUID HWMonitor-level sensor coverage and universal fan/overclocking control. Broad coverage requires vendor integrations and, for some motherboard sensors, a sensor driver. The app must report source and availability per reading so a missing sensor is never mistaken for a zero value.
