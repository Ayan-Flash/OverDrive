import { useMemo } from 'react';
import { AreaChart, Area, ResponsiveContainer, YAxis } from 'recharts';
import type { ExportedSample } from '../../services';

interface RealtimeChartProps {
  data: ExportedSample[];
  metric: keyof ExportedSample;
  color: string;
  label: string;
  unit: string;
  height?: number;
}

export function RealtimeChart({ data, metric, color, label, unit, height = 120 }: RealtimeChartProps) {
  const chartData = useMemo(() => {
    if (data.length === 0) return [];
    return data.map((d) => ({ v: Number(d[metric]) }));
  }, [data, metric]);

  const currentValue = chartData.length > 0 ? chartData[chartData.length - 1].v : 0;
  const displayValue = typeof currentValue === 'number' ? currentValue.toFixed(1) : '--';

  return (
    <div className="card flex flex-col gap-2 p-3.5">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="size-2 rounded-full" style={{ backgroundColor: color, boxShadow: `0 0 6px ${color}60` }} />
          <span className="text-xs font-medium uppercase tracking-wider text-text-secondary">{label}</span>
        </div>
        <span className="metric-value text-sm text-text-primary">
          {displayValue}
          <span className="text-text-muted ml-0.5 text-[10px] font-normal">{unit}</span>
        </span>
      </div>

      <div style={{ height }}>
        {chartData.length > 1 ? (
          <ResponsiveContainer width="100%" height="100%">
            <AreaChart data={chartData} margin={{ top: 2, right: 0, bottom: 2, left: 0 }}>
              <defs>
                <linearGradient id={`grad-${metric}`} x1="0" y1="0" x2="0" y2="1">
                  <stop offset="0%" stopColor={color} stopOpacity={0.25} />
                  <stop offset="100%" stopColor={color} stopOpacity={0} />
                </linearGradient>
              </defs>
              <YAxis domain={['dataMin - 5', 'dataMax + 5']} hide />
              <Area
                type="monotone"
                dataKey="v"
                stroke={color}
                strokeWidth={1.5}
                fill={`url(#grad-${metric})`}
                dot={false}
                isAnimationActive={false}
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex h-full items-center justify-center">
            <span className="text-[10px] text-text-dim">Waiting for data...</span>
          </div>
        )}
      </div>
    </div>
  );
}
