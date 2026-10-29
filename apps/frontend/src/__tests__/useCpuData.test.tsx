import { act, cleanup, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useCpuData } from '../components/overdrive/useCpuData';
import { cpuService, type CpuSample } from '../services/cpuService';

vi.mock('../services/cpuService', () => ({
  cpuService: { getInfo: vi.fn(), getSample: vi.fn() },
}));

const sample = (timestamp: number, usage = 25): CpuSample => ({
  timestamp, frequency_mhz: 4200, max_frequency_mhz: 5000,
  usage_percent: usage, temperature_celsius: 61, voltage_volts: null,
  cores: [{ core_index: 0, frequency_mhz: 4200, usage_percent: usage, temperature_celsius: null }],
});

describe('CPU live polling', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.mocked(cpuService.getInfo).mockResolvedValue({
      model: 'Test CPU', vendor: 'Test', physical_cores: 1, logical_cores: 1, is_elevated: true,
    });
    vi.mocked(cpuService.getSample).mockResolvedValue(sample(100));
  });
  afterEach(() => {
    cleanup();
    vi.useRealTimers();
    vi.resetAllMocks();
  });

  it('updates metrics on successive samples and recovers after a failed read', async () => {
    const { result } = renderHook(useCpuData);
    await act(async () => {});
    expect(result.current.usage).toBe(25);
    vi.mocked(cpuService.getSample).mockRejectedValueOnce(new Error('disconnected'));
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(result.current.telemetryAvailable).toBe(false);
    expect(result.current.temperature).toBeNull();
    vi.mocked(cpuService.getSample).mockResolvedValue(sample(200, 80));
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(result.current.telemetryAvailable).toBe(true);
    expect(result.current.usage).toBe(80);
    expect(result.current.sampledAt).toBe(200);
    expect(cpuService.getInfo).toHaveBeenCalledTimes(1);
  });

  it('marks a frozen backend cache unavailable until a new sample arrives', async () => {
    const { result } = renderHook(useCpuData);
    await act(async () => { await vi.advanceTimersByTimeAsync(6000); });
    expect(result.current.telemetryAvailable).toBe(false);
    vi.mocked(cpuService.getSample).mockResolvedValue(sample(101));
    await act(async () => { await vi.advanceTimersByTimeAsync(1000); });
    expect(result.current.telemetryAvailable).toBe(true);
  });

  it('does not overlap slow requests or keep polling after unmount', async () => {
    let resolve!: (value: CpuSample) => void;
    vi.mocked(cpuService.getSample).mockReturnValue(new Promise((done) => { resolve = done; }));
    const { result, unmount } = renderHook(useCpuData);
    await act(async () => { await vi.advanceTimersByTimeAsync(3000); });
    expect(cpuService.getSample).toHaveBeenCalledTimes(1);
    unmount();
    await act(async () => { resolve(sample(123)); await vi.advanceTimersByTimeAsync(3000); });
    expect(cpuService.getSample).toHaveBeenCalledTimes(1);
    expect(result.current.telemetryAvailable).toBe(false);
  });
});
