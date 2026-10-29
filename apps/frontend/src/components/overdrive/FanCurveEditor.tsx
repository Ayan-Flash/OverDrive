import React, { useCallback, useMemo, useRef, useState } from 'react';
import type { FanCurvePoint } from '../../services';

/* ================================================================
   FanCurveEditor — classic (temperature → duty%) fan curve editor.
   Drag keyframes to reshape the curve, click empty space to add a
   keyframe, double-click a keyframe to remove it (two remain).
   The vertical marker shows the live GPU temperature target.
   ================================================================ */

const VIEW_W = 360;
const VIEW_H = 220;
const PAD = { left: 36, right: 14, top: 12, bottom: 26 };
const TEMP_MIN = 20;
const TEMP_MAX = 90;
const FAN_MIN = 0;
const FAN_MAX = 100;

interface FanCurveEditorProps {
  points: FanCurvePoint[];
  onChange: (points: FanCurvePoint[]) => void;
  currentTemperature?: number | null;
  disabled?: boolean;
}

function tempToX(temp: number): number {
  const t = (Math.min(TEMP_MAX, Math.max(TEMP_MIN, temp)) - TEMP_MIN) / (TEMP_MAX - TEMP_MIN);
  return PAD.left + t * (VIEW_W - PAD.left - PAD.right);
}

function fanToY(fan: number): number {
  const f = (Math.min(FAN_MAX, Math.max(FAN_MIN, fan)) - FAN_MIN) / (FAN_MAX - FAN_MIN);
  return VIEW_H - PAD.bottom - f * (VIEW_H - PAD.top - PAD.bottom);
}

function xToTemp(x: number): number {
  const t = (x - PAD.left) / (VIEW_W - PAD.left - PAD.right);
  return TEMP_MIN + Math.min(1, Math.max(0, t)) * (TEMP_MAX - TEMP_MIN);
}

function yToFan(y: number): number {
  const f = (VIEW_H - PAD.bottom - y) / (VIEW_H - PAD.top - PAD.bottom);
  return FAN_MIN + Math.min(1, Math.max(0, f)) * (FAN_MAX - FAN_MIN);
}

/** Insert a point keeping the list sorted and temperatures unique. */
function withSortedPoint(points: FanCurvePoint[], next: FanCurvePoint): FanCurvePoint[] {
  const others = points.filter((p) => Math.abs(p.temperature_celsius - next.temperature_celsius) > 0.5);
  return [...others, next].sort((a, b) => a.temperature_celsius - b.temperature_celsius);
}

export function FanCurveEditor({ points, onChange, currentTemperature, disabled }: FanCurveEditorProps) {
  const svgRef = useRef<SVGSVGElement>(null);
  const [dragIndex, setDragIndex] = useState<number | null>(null);

  const curvePath = useMemo(() => {
    if (points.length === 0) return '';
    const sorted = [...points].sort((a, b) => a.temperature_celsius - b.temperature_celsius);
    return sorted
      .map((point, i) => `${i === 0 ? 'M' : 'L'} ${tempToX(point.temperature_celsius).toFixed(1)} ${fanToY(point.fan_percent).toFixed(1)}`)
      .join(' ');
  }, [points]);

  const currentPos = useMemo(() => {
    if (currentTemperature == null || points.length === 0) return null;
    const sorted = [...points].sort((a, b) => a.temperature_celsius - b.temperature_celsius);
    if (currentTemperature <= sorted[0].temperature_celsius) {
      return { x: tempToX(currentTemperature), y: fanToY(sorted[0].fan_percent) };
    }
    if (currentTemperature >= sorted[sorted.length - 1].temperature_celsius) {
      return { x: tempToX(currentTemperature), y: fanToY(sorted[sorted.length - 1].fan_percent) };
    }
    let duty = sorted[0].fan_percent;
    for (let i = 0; i < sorted.length - 1; i++) {
      const a = sorted[i];
      const b = sorted[i + 1];
      if (currentTemperature >= a.temperature_celsius && currentTemperature <= b.temperature_celsius) {
        const span = b.temperature_celsius - a.temperature_celsius;
        const t = span <= 0 ? 0 : (currentTemperature - a.temperature_celsius) / span;
        duty = a.fan_percent + t * (b.fan_percent - a.fan_percent);
        break;
      }
    }
    return { x: tempToX(currentTemperature), y: fanToY(duty) };
  }, [points, currentTemperature]);

  /** Convert a pointer event to viewBox coordinates. */
  const toSvgPoint = useCallback((clientX: number, clientY: number) => {
    const svg = svgRef.current;
    if (!svg) return null;
    const rect = svg.getBoundingClientRect();
    const sx = ((clientX - rect.left) / rect.width) * VIEW_W;
    const sy = ((clientY - rect.top) / rect.height) * VIEW_H;
    return { x: sx, y: sy };
  }, []);

  const handlePointDown = (index: number) => (event: React.PointerEvent) => {
    if (disabled) return;
    event.stopPropagation();
    setDragIndex(index);
    (event.target as Element).setPointerCapture?.(event.pointerId);
  };

  const handleMove = (event: React.PointerEvent) => {
    if (disabled || dragIndex == null) return;
    const svgPoint = toSvgPoint(event.clientX, event.clientY);
    if (!svgPoint) return;
    const next = withSortedPoint(points, {
      temperature_celsius: Math.round(xToTemp(svgPoint.x)),
      fan_percent: Math.round(yToFan(svgPoint.y)),
    });
    // Keep dragging the same keyframe through re-sorting: track it by the
    // temperature it was just assigned.
    const idx = next.findIndex((p) => Math.abs(p.temperature_celsius - Math.round(xToTemp(svgPoint.x))) < 0.5);
    onChange(next);
    setDragIndex(idx >= 0 ? idx : null);
  };

  const handleEnd = () => {
    setDragIndex(null);
  };

  const handleBackgroundClick = (event: React.MouseEvent) => {
    if (disabled) return;
    if (dragIndex != null) return;
    const svgPoint = toSvgPoint(event.clientX, event.clientY);
    if (!svgPoint) return;
    const next = withSortedPoint(points, {
      temperature_celsius: Math.round(xToTemp(svgPoint.x)),
      fan_percent: Math.round(yToFan(svgPoint.y)),
    });
    if (next.length <= 10) onChange(next);
  };

  const removePoint = (index: number) => (event: React.MouseEvent) => {
    if (disabled) return;
    event.stopPropagation();
    if (points.length <= 2) return;
    onChange(points.filter((_, i) => i !== index));
  };

  const gridTemps = [20, 30, 40, 50, 60, 70, 80, 90];
  const gridFans = [0, 25, 50, 75, 100];

  return (
    <svg
      ref={svgRef}
      viewBox={`0 0 ${VIEW_W} ${VIEW_H}`}
      className="od-fancurve"
      onPointerMove={handleMove}
      onPointerUp={handleEnd}
      onPointerLeave={handleEnd}
      onClick={handleBackgroundClick}
      style={{ width: '100%', userSelect: 'none', touchAction: 'none' }}
      role="img"
      aria-label={`Fan curve editor with ${points.length} keyframes`}
    >
      {/* Plot background */}
      <rect x={PAD.left} y={PAD.top} width={VIEW_W - PAD.left - PAD.right} height={VIEW_H - PAD.top - PAD.bottom} fill="rgba(8, 12, 20, 0.55)" stroke="rgba(90, 110, 140, 0.35)" />

      {/* Horizontal gridlines + labels */}
      {gridFans.map((fan) => (
        <g key={`fan-${fan}`}>
          <line x1={PAD.left} x2={VIEW_W - PAD.right} y1={fanToY(fan)} y2={fanToY(fan)} stroke="rgba(90, 110, 140, 0.18)" strokeDasharray="3 4" />
          <text x={PAD.left - 6} y={fanToY(fan) + 3} textAnchor="end" fontSize="8" fill="rgba(150, 170, 200, 0.8)">{fan}%</text>
        </g>
      ))}

      {/* Vertical gridlines + labels */}
      {gridTemps.map((temp) => (
        <g key={`temp-${temp}`}>
          <line x1={tempToX(temp)} x2={tempToX(temp)} y1={PAD.top} y2={VIEW_H - PAD.bottom} stroke="rgba(90, 110, 140, 0.15)" strokeDasharray="3 4" />
          <text x={tempToX(temp)} y={VIEW_H - PAD.bottom + 14} textAnchor="middle" fontSize="8" fill="rgba(150, 170, 200, 0.8)">{temp}°</text>
        </g>
      ))}

      {/* Curve */}
      {curvePath && <path d={curvePath} fill="none" stroke="#3b98e8" strokeWidth="2" style={{ filter: 'drop-shadow(0 0 3px rgba(59, 152, 232, 0.5))' }} />}

      {/* Live temperature marker */}
      {currentPos && (
        <g pointerEvents="none">
          <line x1={currentPos.x} x2={currentPos.x} y1={PAD.top} y2={VIEW_H - PAD.bottom} stroke="rgba(64, 220, 160, 0.5)" strokeWidth="1" />
          <circle cx={currentPos.x} cy={currentPos.y} r="4" fill="#40dc9e" opacity="0.9" />
        </g>
      )}

      {/* Keyframes */}
      {points.map((point, i) => (
        <g
          key={`${point.temperature_celsius}-${i}`}
          transform={`translate(${tempToX(point.temperature_celsius)}, ${fanToY(point.fan_percent)})`}
          onPointerDown={handlePointDown(i)}
          onDoubleClick={removePoint(i)}
          style={{ cursor: disabled ? 'default' : 'grab' }}
        >
          {/* Larger invisible hit area */}
          <circle r="10" fill="transparent" />
          <circle r="5" fill="#0b0f18" stroke="#3b98e8" strokeWidth="2" />
          <text y="-10" textAnchor="middle" fontSize="8" fill="rgba(59, 152, 232, 0.95)" fontWeight="600">
            {point.fan_percent}
          </text>
        </g>
      ))}
    </svg>
  );
}
