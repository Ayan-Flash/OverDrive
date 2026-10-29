# OverDrive

[![Version](https://img.shields.io/badge/version-0.1.0-blue)]()
[![License](https://img.shields.io/badge/license-MIT-green)]()
[![Rust](https://img.shields.io/badge/rust-1.70+-orange)]()
[![Tauri](https://img.shields.io/badge/tauri-v2.0+-blue)](https://tauri.app)
[![React](https://img.shields.io/badge/react-18+-blue)](https://react.dev)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen)]()

---

**Windows hardware monitoring and tuning desktop app** — OverDrive is built with Tauri, Rust, and React. It reads CPU and system-memory telemetry, reads NVIDIA GPU telemetry through NVML when the installed driver exposes it, and applies NVIDIA GPU tuning controls through the same driver interface.

The OverDrive Tuning page follows the MSI AfterBurner model: fan control in three modes (driver automatic, fixed speed, or a custom temperature→duty fan curve applied every second by a background engine), core and memory clock offset sliders, a power limit slider with the driver-reported range, and a one-click reset to driver defaults. Every control is gated by capabilities probed live from the driver, so unsupported controls stay read-only instead of accepting changes that cannot be applied — which controls are actually exposed varies by GPU model and driver build. The interface still does not match the sensor coverage of CPUID HWMonitor or Armory Crate, and AMD/Intel GPU telemetry is not yet supported. See [hardware support](docs/HARDWARE_SUPPORT.md) for current requirements and API work needed.

## Quick Start

```bash
pnpm install
pnpm dev
```

See [docs/README.md](docs/README.md) for full documentation.
