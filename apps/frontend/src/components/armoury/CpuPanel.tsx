import { useEffect } from 'react';

import { motion, useSpring, useTransform } from 'framer-motion';
import { PanelFrame } from './PanelFrame';

/* ================================================================
   CpuPanel — CPU summary with thread frequency, utilization,
   core counts, voltage, and package temperature.
   ================================================================ */

interface CpuPanelProps {
  frequency: number;             // MHz
  usage: number;                 // package utilization, percent
  voltage: number | null;        // Volts (null when unavailable)
  temperature: number | null;    // Celsius (null when unavailable)
  memoryFrequency?: number | null; // MT/s, unavailable when the OS does not expose it
  maxFrequency?: number;
}

function SpringValue({ value, unit = '', decimals = 0 }: { value: number; unit?: string; decimals?: number }) {
  const spring = useSpring(value, { stiffness: 80, damping: 17, mass: 0.5 });
  useEffect(() => {
    spring.set(value);
  }, [value, spring]);

  const display = useTransform(spring, (v) => `${v.toFixed(decimals)}${unit}`);
  return <motion.span>{display}</motion.span>;
}

export function CpuPanel({
  frequency,
  usage,
  voltage,
  temperature,
  memoryFrequency = null,
  maxFrequency = 5000,
}: CpuPanelProps) {
  const freqPercent = Math.min((frequency / maxFrequency) * 100, 100);
  const segmentCount = 20;
  const activeSegments = Math.round((freqPercent / 100) * segmentCount);

  return (
    <PanelFrame title="CPU" className="ac-panel--flex1 ac-info-panel">
      {/* Frequency row */}
      <div className="ac-info-row">
        <span className="ac-info-row__label">Frequency</span>
        <span className="ac-info-row__value">
          <SpringValue value={frequency} unit="MHz" />
        </span>
      </div>

      <div className="ac-info-row">
        <span className="ac-info-row__label">Usage</span>
        <span className="ac-info-row__value"><SpringValue value={usage} unit="%" /></span>
      </div>

      {/* Segmented frequency bar with Framer Motion stagger/smooth layout */}
      <div className="ac-freq-bar ac-freq-bar--segmented">
        {Array.from({ length: segmentCount }, (_, i) => {
          const isActive = i < activeSegments;
          return (
            <motion.div
              key={i}
              className={`ac-freq-bar__segment ${isActive ? 'ac-freq-bar__segment--active' : ''}`}
              animate={{
                opacity: isActive ? 1 : 0.15,
                scaleY: isActive ? 1 : 0.85,
                backgroundColor: isActive ? 'rgba(59, 152, 232, 1)' : 'rgba(59, 152, 232, 0.1)'
              }}
              transition={{ duration: 0.25, ease: 'easeOut' }}
            />
          );
        })}
      </div>

      <div className="ac-info-row">
        <span className="ac-info-row__label">Memory Frequency</span>
        <span className="ac-info-row__value">{memoryFrequency != null ? `${memoryFrequency} MT/s` : 'N/A'}</span>
      </div>

      {/* Package temperature */}
      <div className="ac-info-row">
        <span className="ac-info-row__label">Temperature</span>
        <span className="ac-info-row__value" style={{ display: 'flex', alignItems: 'center' }}>
          {temperature != null ? (
            <SpringValue value={temperature} unit="°C" />
          ) : (
            <span style={{ fontSize: '10px', color: 'var(--ac-text-muted)', fontWeight: 500 }}>
              N/A (sensor unavailable)
            </span>
          )}
        </span>
      </div>

      {/* Voltage */}
      <div className="ac-info-row">
        <span className="ac-info-row__label">Voltage</span>
        <span className="ac-info-row__value">
          {voltage != null ? (
            <SpringValue value={voltage * 1000} unit="mV" />
          ) : (
            'N/A'
          )}
        </span>
      </div>
    </PanelFrame>
  );
}
