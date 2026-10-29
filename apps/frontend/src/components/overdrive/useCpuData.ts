import { useEffect, useState } from 'react';
import { cpuService, type CpuInfo } from '../../services/cpuService';

/* ================================================================
   useCpuData — real CPU telemetry from the Rust backend.
   Values stay null when the backend/OS cannot expose the sensor.
   ================================================================ */

export interface CpuCoreData {
  coreIndex: number;
  frequency: number; // MHz
  usage: number; // %
}

export interface CpuTemperatureData {
  name: string;
  temperature: number;
}

export interface CpuData {
  model: string;
  vendor: string;
  physicalCores: number;
  logicalCores: number;
  frequency: number;
  usage: number;
  voltage: number | null;
  temperature: number | null;
  maxFrequency: number;
  cores: CpuCoreData[];
  coreTemperatures: CpuTemperatureData[];
  isElevated: boolean;
  telemetryAvailable: boolean;
  sampledAt: number | null;
}

const DEFAULT_DATA: CpuData = {
  model: 'CPU',
  vendor: '',
  physicalCores: 0,
  logicalCores: 0,
  frequency: 0,
  usage: 0,
  voltage: null,
  temperature: null,
  maxFrequency: 5000,
  cores: [],
  coreTemperatures: [],
  isElevated: false,
  telemetryAvailable: false,
  sampledAt: null,
};

export function useCpuData(): CpuData {
  const [data, setData] = useState<CpuData>(DEFAULT_DATA);

  useEffect(() => {
    let active = true;
    let inFlight = false;
    let info: CpuInfo | null = null;
    let lastTimestamp: number | null = null;
    let lastProgressAt = Date.now();

    // Serial polling prevents slow IPC calls from overwriting newer samples.
    // A stalled backend must not leave frozen numbers presented as live data.
    const unavailable = () => setData((previous) => ({
      ...previous,
      telemetryAvailable: false,
      temperature: null,
      voltage: null,
      coreTemperatures: [],
    }));
    const poll = async () => {
      if (!active) return;
      if (Date.now() - lastProgressAt > 5000) unavailable();
      if (inFlight) return;
      inFlight = true;
      try {
        const [identity, sample] = await Promise.all([
          info ? Promise.resolve(info) : cpuService.getInfo().catch(() => null),
          cpuService.getSample(),
        ]);
        if (!active) return;
        info = identity;
        if (sample.timestamp !== lastTimestamp) {
          lastTimestamp = sample.timestamp;
          lastProgressAt = Date.now();
        }
        if (Date.now() - lastProgressAt > 5000) {
          unavailable();
          return;
        }
        setData({
          model: info?.model ?? 'CPU',
          vendor: info?.vendor ?? '',
          physicalCores: info?.physical_cores ?? 0,
          logicalCores: info?.logical_cores ?? sample.cores.length,
          frequency: sample.frequency_mhz,
          usage: sample.usage_percent,
          voltage: sample.voltage_volts,
          temperature: sample.temperature_celsius,
          maxFrequency: sample.max_frequency_mhz,
          cores: sample.cores.map((core) => ({
            coreIndex: core.core_index,
            frequency: core.frequency_mhz,
            usage: core.usage_percent,
          })),
          coreTemperatures: (sample.core_temperatures ?? []).map((sensor) => ({
            name: sensor.name,
            temperature: sensor.temperature_celsius,
          })),
          isElevated: info?.is_elevated ?? false,
          telemetryAvailable: true,
          sampledAt: sample.timestamp,
        });
      } catch {
        if (active) unavailable();
      } finally {
        inFlight = false;
      }
    };

    void poll();
    const timer = setInterval(() => void poll(), 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, []);

  return data;
}
