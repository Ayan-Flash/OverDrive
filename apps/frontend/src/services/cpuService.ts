import { invoke } from '@tauri-apps/api/core';

/* ================================================================
   cpuService — real CPU telemetry from the Rust backend (sysinfo).
   Mirrors gpuService's invoke-based shape.
   ================================================================ */

export interface CpuInfo {
  model: string;
  vendor: string;
  physical_cores: number;
  logical_cores: number;
  is_elevated: boolean;
}

export interface CpuCoreSample {
  core_index: number;
  frequency_mhz: number;
  usage_percent: number;
  temperature_celsius: number | null;
}

export interface CpuSample {
  timestamp: number;
  frequency_mhz: number;
  max_frequency_mhz: number;
  usage_percent: number;
  /** null when the OS exposes no CPU thermal sensor. */
  temperature_celsius: number | null;
  /** Labeled physical core temperatures, only when the sensor provider exposes them. */
  core_temperatures?: Array<{ name: string; temperature_celsius: number }>;
  /** null — CPU voltage requires a kernel-mode sensor driver. */
  voltage_volts: number | null;
  cores: CpuCoreSample[];
}

export const cpuService = {
  getInfo: () => invoke<CpuInfo>('get_cpu_info'),
  getSample: () => invoke<CpuSample>('get_cpu_sample'),
};

export interface RamTelemetry {
  total_bytes: number;
  available_bytes: number;
  used_bytes: number;
  utilization_percent: number;
}

export interface StorageTelemetry {
  volume: string;
  total_bytes: number;
  user_available_bytes: number;
  volume_free_bytes: number;
  used_bytes: number;
  utilization_percent: number;
}

export const systemTelemetryService = {
  getRam: () => invoke<RamTelemetry>('get_ram_telemetry'),
  getStorage: () => invoke<StorageTelemetry[]>('get_storage_telemetry'),
};
