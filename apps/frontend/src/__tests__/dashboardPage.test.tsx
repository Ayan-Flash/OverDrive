import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DashboardPage } from '../pages/DashboardPage';
import { useAlertStore } from '../stores/alertStore';
import { useGpuStore } from '../stores/gpuStore';
import { useUiStore } from '../stores/uiStore';

vi.mock('../components/armoury/useCpuData', () => ({
  useCpuData: () => ({
    model: 'CPU',
    vendor: '',
    physicalCores: 0,
    logicalCores: 0,
    frequency: 0,
    usage: 0,
    voltage: null,
    temperature: null,
    maxFrequency: 0,
    cores: [],
    isElevated: false,
  }),
}));

vi.mock('../components/feature/RealtimeChart', () => ({
  RealtimeChart: () => <div>GPU history chart</div>,
}));

vi.mock('../services', () => ({
  alertService: { getHistory: vi.fn(() => new Promise(() => {})) },
  monitoringService: { isRunning: vi.fn(() => new Promise(() => {})) },
  overlayService: { getDetectedGames: vi.fn(() => new Promise(() => {})) },
}));

vi.mock('../services/cpuService', () => ({
  systemTelemetryService: {
    getRam: () => new Promise(() => {}),
    getStorage: () => new Promise(() => {}),
  },
}));

describe('Dashboard overview', () => {
  beforeEach(() => {
    useGpuStore.setState({
      gpus: [],
      currentData: new Map(),
      history: new Map(),
      aggregated: new Map(),
      controlStates: new Map(),
      alerts: [],
      loading: false,
      error: null,
    });
    useAlertStore.setState({ alerts: [], rules: [], unacknowledgedCount: 0 });
    useUiStore.setState({ selectedGpuId: null, currentPage: 'dashboard' });
  });

  it('shows explicit unavailable states when hardware telemetry is missing', () => {
    render(<DashboardPage />);

    expect(screen.getByText('CPU data unavailable')).toBeInTheDocument();
    expect(screen.getByText('No compatible GPU was detected.')).toBeInTheDocument();
    expect(screen.getByText('System memory telemetry isn’t available.')).toBeInTheDocument();
    expect(screen.getByText('Storage telemetry isn’t available.')).toBeInTheDocument();
    expect(screen.getAllByRole('img', { name: 'Temperature: unavailable' })).toHaveLength(2);
  });

  it('renders numeric zero as a reading and null GPU values as unavailable', () => {
    useGpuStore.setState({
      gpus: [{ id: 'gpu-0', name: 'Test GPU', vendor: 'nvidia', index: 0 }],
      currentData: new Map([['gpu-0', {
        gpu_id: 'gpu-0', timestamp: 0, temperature_celsius: 0, core_clock_mhz: null,
        memory_clock_mhz: null, memory_used_mb: null, memory_total_mb: null,
        fan_speed_percent: null, fan_rpm: null, power_watts: 0, max_power_limit_watts: null,
        core_voltage_mv: null, core_utilization_percent: 0, memory_utilization_percent: null,
      }]]),
    });
    useUiStore.setState({ selectedGpuId: 'gpu-0' });
    render(<DashboardPage />);
    expect(screen.getByText('0 W')).toBeInTheDocument();
    expect(screen.getByText('0%')).toBeInTheDocument();
    expect(screen.getByText('Core clock').parentElement?.querySelector('strong')).toHaveTextContent('Unavailable');
  });

  it('routes the header shortcuts to existing settings and hardware control pages', () => {
    render(<DashboardPage />);
    fireEvent.click(screen.getByRole('button', { name: /settings/i }));
    expect(useUiStore.getState().currentPage).toBe('settings');

    fireEvent.click(screen.getByRole('button', { name: /my pc/i }));
    expect(useUiStore.getState().currentPage).toBe('control');
  });

  it('routes each quick action to its existing destination', () => {
    const { unmount } = render(<DashboardPage />);
    const actions = [
      ['Game Analysis', 'reports'],
      ['Optimize PC', 'ai'],
      ['System Health', 'alerts'],
      ['Gaming Overlay', 'overlay'],
    ] as const;

    for (const [label, destination] of actions) {
      fireEvent.click(screen.getByRole('button', { name: new RegExp(label, 'i') }));
      expect(useUiStore.getState().currentPage).toBe(destination);
    }
    unmount();
  });
});
