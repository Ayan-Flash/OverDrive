import { create } from 'zustand';
import { gpuService, controlService } from '../services';
import type { GPUData, GPUInfo, GpuControlState, ExportedSample, AggregatedMetrics, AlertEvent } from '../services';

interface GpuStore {
  gpus: GPUInfo[];
  currentData: Map<string, GPUData>;
  history: Map<string, ExportedSample[]>;
  aggregated: Map<string, AggregatedMetrics | null>;
  controlStates: Map<string, GpuControlState>;
  alerts: AlertEvent[];
  loading: boolean;
  error: string | null;

  fetchGpus: () => Promise<void>;
  fetchData: (gpuId: string) => Promise<void>;
  fetchLiveSample: (gpuId: string) => Promise<void>;
  fetchHistory: (gpuId: string, count?: number) => Promise<void>;
  fetchControlState: (gpuId: string) => Promise<void>;
  setFanSpeed: (gpuId: string, percent: number) => Promise<void>;
  setCoreClock: (gpuId: string, offset: number) => Promise<void>;
  setMemoryClock: (gpuId: string, offset: number) => Promise<void>;
  setPowerLimit: (gpuId: string, percent: number) => Promise<void>;
  setVoltage: (gpuId: string, offset: number) => Promise<void>;
}

export const useGpuStore = create<GpuStore>((set, get) => ({
  gpus: [],
  currentData: new Map(),
  history: new Map(),
  aggregated: new Map(),
  controlStates: new Map(),
  alerts: [],
  loading: false,
  error: null,

  fetchGpus: async () => {
    set({ loading: true, error: null });
    try {
      const gpus = await gpuService.list();
      set({ gpus, loading: false });
    } catch (err) {
      set({ loading: false, error: String(err) });
    }
  },

  fetchData: async (gpuId: string) => {
    try {
      const response = await gpuService.getData(gpuId);
      const data = new Map(get().currentData);
      data.set(gpuId, response.sample);
      const alerts = response.alerts as AlertEvent[];
      set({ currentData: data, alerts: [...get().alerts, ...alerts].slice(-100) });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  fetchLiveSample: async (gpuId: string) => {
    // Fast display path backed by `get_gpu_sample`: a live driver probe with
    // no recording or alert side effects, safe to poll at ~10 Hz.
    try {
      const sample = await gpuService.getSample(gpuId);
      const data = new Map(get().currentData);
      data.set(gpuId, sample);
      set({ currentData: data });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  fetchHistory: async (gpuId: string, count = 120) => {
    try {
      const response = await gpuService.getHistory(gpuId, count);
      const history = new Map(get().history);
      history.set(gpuId, response.samples);
      const aggregated = new Map(get().aggregated);
      aggregated.set(gpuId, response.aggregated);
      set({ history, aggregated });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  fetchControlState: async (gpuId: string) => {
    try {
      const state = await gpuService.getControlState(gpuId);
      const states = new Map(get().controlStates);
      states.set(gpuId, state);
      set({ controlStates: states });
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setFanSpeed: async (gpuId: string, percent: number) => {
    try {
      await controlService.setFanSpeed(gpuId, percent);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setCoreClock: async (gpuId: string, offset: number) => {
    try {
      await controlService.setCoreClockOffset(gpuId, offset);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setMemoryClock: async (gpuId: string, offset: number) => {
    try {
      await controlService.setMemoryClockOffset(gpuId, offset);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setPowerLimit: async (gpuId: string, percent: number) => {
    try {
      await controlService.setPowerLimit(gpuId, percent);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  setVoltage: async (gpuId: string, offset: number) => {
    try {
      await controlService.setVoltageOffset(gpuId, offset);
    } catch (err) {
      set({ error: String(err) });
    }
  },
}));
