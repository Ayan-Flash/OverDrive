import { useCallback, useEffect, useRef, useState } from 'react';
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
};

export function useCpuData(): CpuData {
  const [data, setData] = useState<CpuData>(DEFAULT_DATA);
  const infoRef = useRef<CpuInfo | null>(null);
  const infoFetched = useRef(false);

  useEffect(() => {
    if (infoFetched.current) return;
    infoFetched.current = true;
    cpuService
      .getInfo()
      .then((info) => {
        infoRef.current = info;
        setData((prev) => ({
          ...prev,
          model: info.model,
          vendor: info.vendor,
          physicalCores: info.physical_cores,
          logicalCores: info.logical_cores,
          isElevated: info.is_elevated,
        }));
      })
      .catch(() => {});
  }, []);

  const poll = useCallback(async () => {
    try {
      let info = infoRef.current;
      if (!info) {
        try {
          info = await cpuService.getInfo();
          infoRef.current = info;
        } catch {
          /* Ignore and proceed with sample */
        }
      }

      const s = await cpuService.getSample();

      setData({
        model: info?.model ?? 'CPU',
        vendor: info?.vendor ?? '',
        physicalCores: info?.physical_cores ?? 0,
        logicalCores: info?.logical_cores ?? s.cores.length,
        frequency: s.frequency_mhz,
        usage: s.usage_percent,
        voltage: s.voltage_volts,
        temperature: s.temperature_celsius,
        maxFrequency: s.max_frequency_mhz,
        cores: s.cores.map((c) => ({
          coreIndex: c.core_index,
          frequency: c.frequency_mhz,
          usage: c.usage_percent,
        })),
        coreTemperatures: (s.core_temperatures ?? []).map((sensor) => ({
          name: sensor.name,
          temperature: sensor.temperature_celsius,
        })),
        isElevated: info?.is_elevated ?? false,
        telemetryAvailable: true,
      });
    } catch {
      /* Backend not reachable — keep last known values. */
    }
  }, []);

  useEffect(() => {
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  }, [poll]);

  return data;
}
