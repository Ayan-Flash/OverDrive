import type { GPUData, GPUInfo } from '../../services';
import { useUiStore } from '../../stores';
import { formatClockSpeed, formatFanSpeed, formatPowerUsage } from '@common/utils';

interface GpuCardProps {
  gpu: GPUInfo;
  data: GPUData | undefined;
}

function MiniGauge({ value, max, color, label }: { value: number; max: number; color: string; label: string }) {
  const pct = Math.min(value / max, 1);
  const circumference = 2 * Math.PI * 16;
  const offset = circumference * (1 - pct);
  return (
    <div className="flex flex-col items-center gap-0.5">
      <svg width="40" height="40" viewBox="0 0 40 40" className="-rotate-90">
        <circle cx="20" cy="20" r="16" fill="none" stroke="currentColor" strokeWidth="3" className="text-gpu-700" />
        <circle cx="20" cy="20" r="16" fill="none" stroke={color} strokeWidth="3" strokeLinecap="round"
          strokeDasharray={circumference} strokeDashoffset={offset} />
      </svg>
      <span className="text-text-muted text-[8px] font-medium uppercase tracking-wider">{label}</span>
    </div>
  );
}

export function GpuCard({ gpu, data }: GpuCardProps) {
  const { selectedGpuId, setSelectedGpu } = useUiStore();
  const isSelected = selectedGpuId === gpu.id;

  const temp = data?.temperature_celsius;
  const tempColor = temp == null ? 'text-text-muted' : temp > 80 ? 'text-temp-hot' : temp > 65 ? 'text-temp-warm' : 'text-temp-cool';

  return (
    <button
      onClick={() => setSelectedGpu(gpu.id)}
      className={`card p-4 text-left transition-all duration-200 ${
        isSelected
          ? 'ring-accent/50 border-accent/30 bg-accent-subtle shadow-gpu ring-1'
          : 'card-hover'
      }`}
    >
      <div className="mb-3 flex items-start justify-between">
        <div className="min-w-0">
          <div className="mb-0.5 flex items-center gap-2">
            <span className="text-text-muted text-[10px] font-semibold uppercase tracking-widest">{gpu.vendor}</span>
            <span className="text-text-dim font-mono text-[10px]">#{gpu.index}</span>
          </div>
          <div className="text-text-primary truncate text-sm font-semibold">{gpu.name}</div>
        </div>
        {data && temp != null ? (
          <div className="flex flex-shrink-0 items-baseline gap-0.5">
            <span className={`text-2xl font-bold tabular-nums ${tempColor}`}>
              {temp.toFixed(0)}
            </span>
            <span className="text-text-muted text-xs">°C</span>
          </div>
        ) : <span className="text-text-muted text-xs">Unavailable</span>}
      </div>

      {data ? (
        <>
          <div className="mb-3 grid grid-cols-4 gap-1">
            {data.core_utilization_percent != null && <MiniGauge value={data.core_utilization_percent} max={100} color="#f04747" label="GPU" />}
            {data.memory_used_mb != null && data.memory_total_mb != null && data.memory_total_mb > 0 && <MiniGauge value={data.memory_used_mb} max={data.memory_total_mb} color="#22d3ee" label="VRAM" />}
            {data.power_watts != null && <MiniGauge value={data.power_watts} max={450} color="#f59e0b" label="PWR" />}
            {data.fan_speed_percent != null && <MiniGauge value={data.fan_speed_percent} max={100} color="#a78bfa" label="FAN" />}
          </div>

          <div className="flex flex-wrap gap-x-4 gap-y-1 text-[11px]">
            <span className="text-text-muted">Core <span className="metric-value text-text-primary">{data.core_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.core_clock_mhz)}</span></span>
            <span className="text-text-muted">VRAM <span className="metric-value text-text-primary">{data.memory_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.memory_clock_mhz)}</span></span>
            <span className="text-text-muted">Fan <span className="metric-value text-text-primary">{data.fan_speed_percent == null ? 'Unavailable' : formatFanSpeed(data.fan_speed_percent)}</span></span>
            <span className="text-text-muted">Power <span className="metric-value text-text-primary">{data.power_watts == null ? 'Unavailable' : formatPowerUsage(data.power_watts)}</span></span>
          </div>
        </>
      ) : (
        <div className="text-text-muted py-4 text-center text-xs">Waiting for data...</div>
      )}
    </button>
  );
}
