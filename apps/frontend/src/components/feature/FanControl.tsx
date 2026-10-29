import { useState } from 'react';
import { controlService } from '../../services';
import { formatFanSpeed } from '@common/utils';

interface FanControlProps {
  gpuId: string;
  currentSpeed: number;
}

export function FanControl({ gpuId, currentSpeed }: FanControlProps) {
  const [speed, setSpeed] = useState(currentSpeed);
  const [applying, setApplying] = useState(false);

  const handleChange = async (value: number) => {
    setSpeed(value);
    setApplying(true);
    try {
      await controlService.setFanSpeed(gpuId, value);
    } catch {
      setSpeed(currentSpeed);
    } finally {
      setApplying(false);
    }
  };

  return (
    <div className="card flex flex-col gap-3 p-4">
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium uppercase tracking-wider text-text-secondary">Fan Speed</span>
        <div className="flex items-center gap-2">
          {applying && <span className="text-[10px] text-text-dim">Applying...</span>}
          <span className="metric-value text-base text-cyan-400">{formatFanSpeed(speed)}</span>
        </div>
      </div>
      <input type="range" min={0} max={100} step={1} value={speed}
        onChange={(e) => handleChange(Number(e.target.value))}
        className="slider-gpu" />
      <div className="flex justify-between text-[10px] text-text-dim">
        <span>0%</span><span>50%</span><span>100%</span>
      </div>
    </div>
  );
}
