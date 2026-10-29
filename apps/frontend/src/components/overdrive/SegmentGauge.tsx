export interface SegmentGaugeProps {
  value: number | null | undefined;
  min: number;
  max: number;
  unit: string;
  label: string;
  precision?: number;
  segments?: number;
}

function isValid(value: number | null | undefined, min: number, max: number): value is number {
  return typeof value === 'number' && Number.isFinite(value) && Number.isFinite(min) && Number.isFinite(max) && max > min;
}

export function SegmentGauge({
  value,
  min,
  max,
  unit,
  label,
  precision = 0,
  segments = 28,
}: SegmentGaugeProps) {
  const valid = isValid(value, min, max);
  const ratio = valid ? Math.min(1, Math.max(0, (value - min) / (max - min))) : 0;
  const safeSegments = Number.isFinite(segments) ? Math.max(4, Math.floor(segments)) : 28;
  const activeSegments = Math.round(ratio * safeSegments);
  const centerX = 120;
  const centerY = 112;
  const innerRadius = 77;
  const outerRadius = 91;
  const marks = Array.from({ length: safeSegments }, (_, index) => {
    const angle = Math.PI - (index / (safeSegments - 1)) * Math.PI;
    const x1 = centerX + innerRadius * Math.cos(angle);
    const y1 = centerY - innerRadius * Math.sin(angle);
    const x2 = centerX + outerRadius * Math.cos(angle);
    const y2 = centerY - outerRadius * Math.sin(angle);
    return (
      <line
        key={index}
        x1={x1.toFixed(2)}
        y1={y1.toFixed(2)}
        x2={x2.toFixed(2)}
        y2={y2.toFixed(2)}
        className={
          index < activeSegments
            ? 'segment-gauge__tick segment-gauge__tick--active'
            : 'segment-gauge__tick'
        }
      />
    );
  });

  return (
    <div
      className="segment-gauge"
      role="img"
      aria-label={valid ? label + ': ' + value.toFixed(precision) + unit : label + ': unavailable'}
    >
      <svg className="segment-gauge__arc" viewBox="0 0 240 132" aria-hidden="true">
        {marks}
      </svg>
      <div className="segment-gauge__reading" aria-hidden="true">
        <div className="segment-gauge__value">
          {valid ? value.toFixed(precision) : '—'}
          {valid && unit && <span>{unit}</span>}
        </div>
        <div className="segment-gauge__label">{valid ? label : label + ' unavailable'}</div>
      </div>
    </div>
  );
}

export default SegmentGauge;

