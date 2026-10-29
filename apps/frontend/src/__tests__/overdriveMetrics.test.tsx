import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { CpuGauge } from '../components/overdrive/CpuGauge';
import { GpuPanel } from '../components/overdrive/GpuPanel';

describe('OverDrive dashboard metrics', () => {
  it('renders the CPU gauge MHz value directly', () => {
    render(<CpuGauge value={3801} maxValue={5000} />);

    expect(screen.getByText('3801')).toBeInTheDocument();
  });

  it('does not display fabricated GPU values while telemetry is unavailable', () => {
    render(
      <GpuPanel
        gpuName="No GPU"
        frequency={null}
        voltage={null}
        temperature={null}
        usage={null}
        memoryClock={null}
      />,
    );

    expect(screen.getAllByText('N/A').length).toBeGreaterThanOrEqual(5);
    expect(screen.queryByText('219MHz')).not.toBeInTheDocument();
    expect(screen.queryByText('33°C')).not.toBeInTheDocument();
  });
});
