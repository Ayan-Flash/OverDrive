import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { DashboardPage } from '../pages/DashboardPage';
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
    useUiStore.setState({ selectedGpuId: null, currentPage: 'dashboard' });
  });

  it('shows unavailable readings instead of inventing hardware values', () => {
    render(<DashboardPage />);

    expect(screen.getByText('No compatible GPU was detected.')).toBeInTheDocument();
    expect(screen.getByText('Waiting for hardware')).toBeInTheDocument();
    expect(screen.getAllByText('—').length).toBeGreaterThan(0);
  });

  it('opens the existing hardware controls from the system summary', () => {
    render(<DashboardPage />);

    fireEvent.click(screen.getByRole('button', { name: /open hardware controls/i }));

    expect(useUiStore.getState().currentPage).toBe('control');
  });
});

