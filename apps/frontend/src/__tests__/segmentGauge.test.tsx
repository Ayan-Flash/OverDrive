import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { SegmentGauge } from '../components/armoury/SegmentGauge';

describe('SegmentGauge', () => {
  it('renders a segmented arc and preserves the metric unit', () => {
    const { container } = render(
      <SegmentGauge value={50} min={0} max={100} unit="%" label="Utilization" segments={10} />,
    );

    expect(screen.getByRole('img', { name: 'Utilization: 50%' })).toBeInTheDocument();
    expect(container.querySelectorAll('.segment-gauge__tick--active')).toHaveLength(5);
  });

  it('shows unavailable for missing, non-finite, or invalid-range readings', () => {
    const { rerender } = render(
      <SegmentGauge value={null} min={0} max={100} unit="°C" label="Temperature" />,
    );
    expect(screen.getByRole('img', { name: 'Temperature: unavailable' })).toBeInTheDocument();
    expect(screen.getByText('Temperature unavailable')).toBeInTheDocument();

    rerender(<SegmentGauge value={Number.NaN} min={0} max={100} unit="°C" label="Temperature" />);
    expect(screen.getByRole('img', { name: 'Temperature: unavailable' })).toBeInTheDocument();

    rerender(<SegmentGauge value={40} min={100} max={0} unit="°C" label="Temperature" />);
    expect(screen.getByRole('img', { name: 'Temperature: unavailable' })).toBeInTheDocument();
  });

  it('clamps the arc to its range while retaining the actual value', () => {
    const { container } = render(
      <SegmentGauge value={120} min={0} max={100} unit="°C" label="Temperature" segments={8} />,
    );

    expect(screen.getByRole('img', { name: 'Temperature: 120°C' })).toBeInTheDocument();
    expect(container.querySelectorAll('.segment-gauge__tick--active')).toHaveLength(8);
  });
});
