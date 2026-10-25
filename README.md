# GPUControl Pro

[![Version](https://img.shields.io/badge/version-0.1.0-blue)]()
[![License](https://img.shields.io/badge/license-MIT-green)]()
[![Rust](https://img.shields.io/badge/rust-1.70+-orange)]()
[![Tauri](https://img.shields.io/badge/tauri-v2.0+-blue)](https://tauri.app)
[![React](https://img.shields.io/badge/react-18+-blue)](https://react.dev)
[![CI](https://img.shields.io/badge/CI-passing-brightgreen)]()

---

**Windows hardware monitoring desktop app** — GPUControl Pro is built with Tauri, Rust, and React. It currently reads CPU and system-memory telemetry, and reads NVIDIA GPU telemetry through NVML when the installed driver exposes it.

The interface is a custom hardware dashboard. It does not yet match the sensor coverage or tuning capabilities of CPUID HWMonitor, Armory Crate, or MSI Center. GPU tuning controls are not implemented, and AMD/Intel GPU telemetry is not yet supported. See [hardware support](docs/HARDWARE_SUPPORT.md) for current requirements and API work needed.

## Quick Start

```bash
pnpm install
pnpm dev
```

See [docs/README.md](docs/README.md) for full documentation.
