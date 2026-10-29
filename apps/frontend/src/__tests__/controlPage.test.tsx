import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ControlPage } from '../pages/ControlPage';
import { useGpuStore } from '../stores/gpuStore';
import { useUiStore } from '../stores/uiStore';
import type { FanCurveView, GpuControlState } from '../services';

vi.mock('../services', () => ({
  gpuService: {
    getControlState: vi.fn(),
    // The 100 ms display poll stays inert: a pending promise never resolves.
    getSample: () => new Promise(() => {}),
  },
  controlService: {
    setFanSpeed: vi.fn(),
    setCoreClockOffset: vi.fn(),
    setMemoryClockOffset: vi.fn(),
    setPowerLimit: vi.fn(),
    setVoltageOffset: vi.fn(),
    restoreFanControlAutomatic: vi.fn(),
    resetControls: vi.fn(),
  },
  fanCurveService: { get: vi.fn(), set: vi.fn(), setEnabled: vi.fn() },
}));

import { controlService, fanCurveService, gpuService } from '../services';

/** Mirrors the live control state this machine's driver reports: power limit
 *  is exposed, the rest of the controls probe as unsupported. */
const controlState: GpuControlState = {
  gpu_id: 'gpu-0',
  target_fan_speed: null,
  fan_control_automatic: true,
  fan_count: 3,
  core_clock_offset_mhz: 100,
  memory_clock_offset_mhz: 0,
  power_limit_percent: 100,
  power_limit_min_percent: 78.26,
  power_limit_max_percent: 100,
  voltage_offset_mv: 0,
  core_clock_offset_limit_mhz: 500,
  memory_clock_offset_limit_mhz: 2000,
  capabilities: {
    fan_control: false,
    core_clock_offset: false,
    memory_clock_offset: false,
    power_limit: true,
    voltage_offset: false,
  },
};

const curveView: FanCurveView = {
  gpu_id: 'gpu-0',
  enabled: false,
  points: [
    { temperature_celsius: 30, fan_percent: 25 },
    { temperature_celsius: 80, fan_percent: 100 },
  ],
};

const sample = {
  gpu_id: 'gpu-0',
  timestamp: 0,
  temperature_celsius: 67.5,
  core_clock_mhz: 4400,
  memory_clock_mhz: 9000,
  memory_used_mb: 2048,
  memory_total_mb: 8192,
  fan_speed_percent: 42,
  fan_rpm: 1600,
  power_watts: 120.4,
  max_power_limit_watts: null,
  core_voltage_mv: 1050.5,
  core_utilization_percent: 33,
  memory_utilization_percent: null,
};

describe('OverDrive Tuning page', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useGpuStore.setState({
      gpus: [{ id: 'gpu-0', name: 'Test GPU', vendor: 'nvidia', index: 0 }],
      currentData: new Map([['gpu-0', sample]]),
      history: new Map(),
      aggregated: new Map(),
      controlStates: new Map(),
      alerts: [],
      loading: false,
      error: null,
    });
    useUiStore.setState({ selectedGpuId: 'gpu-0', currentPage: 'control' });
    vi.mocked(gpuService.getControlState).mockResolvedValue(controlState);
    vi.mocked(fanCurveService.get).mockResolvedValue(curveView);
  });

  it('asks users to select a GPU when none is selected', () => {
    useUiStore.setState({ selectedGpuId: null });
    render(<ControlPage />);
    expect(screen.getByText(/No GPU selected/i)).toBeInTheDocument();
  });

  it('renders live telemetry and capability-gated tuning controls', async () => {
    render(<ControlPage />);

    // Telemetry strip renders the store sample directly.
    expect(screen.getByText('67.5°C')).toBeInTheDocument();
    expect(screen.getByText('120.4 W')).toBeInTheDocument();

    // The power slider works because the capability probe reports it; the
    // clock offset sliders stay read-only because the probe does not.
    expect(await screen.findByLabelText(/Power limit/i)).toBeEnabled();
    expect(screen.getByLabelText(/Core clock offset —/i)).toBeDisabled();
    expect(screen.getByLabelText(/Memory clock offset —/i)).toBeDisabled();
    expect(screen.getByText(/not exposed by this build/i)).toBeInTheDocument();
  });

  it('maps driver errors to a readable action banner on Reset All', async () => {
    vi.mocked(controlService.resetControls).mockRejectedValueOnce(
      'set controls failure: provider not available: no telemetry provider loaded for gpu-0',
    );
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);

    render(<ControlPage />);
    fireEvent.click(screen.getByRole('button', { name: /Reset All/i }));

    expect(await screen.findByText(/provider not available/i)).toBeInTheDocument();
    confirmSpy.mockRestore();
  });
});
