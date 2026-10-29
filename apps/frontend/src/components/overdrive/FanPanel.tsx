import React, { useEffect } from 'react';
import { motion, useSpring, useTransform } from 'framer-motion';
import { PanelFrame } from './PanelFrame';

/* ================================================================
   FanPanel — Enhanced "Fan Speed" panel with animated fan blades,
   circular speed indicator arcs, RPM display, and glowing mode cards.
   Powered by Framer Motion for buttery-smooth spring physics transitions.
   ================================================================ */

interface FanPanelProps {
  activeMode?: 'silence' | 'standard' | 'turbo' | 'full';
  onModeChange?: (mode: string) => void;
  rpm?: number | null;
  percent?: number | null;
}

const modes = [
  { id: 'silence',  label: 'Silence',    speed: 0, pct: 0,   color: '#3d5a80' },
  { id: 'standard', label: 'Standard',   speed: 1, pct: 35,  color: '#3b98e8' },
  { id: 'turbo',    label: 'Turbo',      speed: 2, pct: 70,  color: '#e8a33b' },
  { id: 'full',     label: 'Full Speed', speed: 3, pct: 100, color: '#e84040' },
] as const;

/** Spring-animated digital RPM counter. */
function AnimatedRPM({ value }: { value: number }) {
  const spring = useSpring(0, { stiffness: 60, damping: 15, mass: 0.8 });

  useEffect(() => {
    spring.set(value);
  }, [value, spring]);

  const rounded = useTransform(spring, (v) => Math.round(v));

  return <motion.span>{rounded}</motion.span>;
}

/** Spring-animated digital Percentage counter. */
function AnimatedPct({ value }: { value: number }) {
  const spring = useSpring(0, { stiffness: 60, damping: 15, mass: 0.8 });

  useEffect(() => {
    spring.set(value);
  }, [value, spring]);

  const rounded = useTransform(spring, (v) => `${Math.round(v)}%`);

  return <motion.span>{rounded}</motion.span>;
}

/** Detailed 6-blade fan SVG with outer ring and spring-accelerating animation. */
function FanIcon({ speed, color, size = 48 }: { speed: number; color: string; size?: number }) {
  const bladeCount = 6;
  const blades: React.ReactNode[] = [];

  for (let i = 0; i < bladeCount; i++) {
    const angle = (360 / bladeCount) * i;
    blades.push(
      <path key={i}
        d="M24 10 C26 14 27 18 24 24 C21 18 22 14 24 10Z"
        fill={color} opacity={speed > 0 ? 0.85 : 0.35}
        transform={`rotate(${angle} 24 24)`} />
    );
  }

  // Use a spring-animated rotation duration to smoothly transition the spinning animation
  const springSpeed = useSpring(speed, { stiffness: 30, damping: 12 });
  useEffect(() => {
    springSpeed.set(speed);
  }, [speed, springSpeed]);

  // Map the animated speed value to spinning duration or style
  // Using pure CSS keyframes with variable speed (updating custom property or animationDuration)
  const durVal = speed === 0 ? 0 : Math.max(0.2, 1.6 - speed * 0.4);

  return (
    <svg width={size} height={size} viewBox="0 0 48 48" fill="none" style={{ overflow: 'visible' }}>
      {/* Outer ring */}
      <circle cx="24" cy="24" r="22" fill="none" stroke={speed > 0 ? color : '#2a3040'}
        strokeWidth="1.5" opacity={speed > 0 ? 0.4 : 0.2} />
      {speed > 0 && (
        <circle cx="24" cy="24" r="22" fill="none" stroke={color}
          strokeWidth="1" opacity={0.15}
          style={{ filter: `drop-shadow(0 0 4px ${color})` }} />
      )}
      {/* Spinning group */}
      <g style={{
        transformOrigin: '24px 24px',
        animation: speed > 0 ? `fan-spin ${durVal}s linear infinite` : 'none',
      }}>
        {blades}
        {/* Center hub */}
        <circle cx="24" cy="24" r="4.5" fill="#1a1e28" stroke={color}
          strokeWidth="1" opacity={speed > 0 ? 0.9 : 0.4} />
        <circle cx="24" cy="24" r="2" fill={speed > 0 ? color : '#3a4050'} opacity="0.7" />
      </g>
      <style>{`
        @keyframes fan-spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }
      `}</style>
    </svg>
  );
}

/** Mini arc indicator showing speed percentage. Animates using SVG strokeDashoffset or SVG path generation with Framer Motion. */
function SpeedArc({ pct, color, size = 48 }: { pct: number; color: string; size?: number }) {
  const r = 20;
  const cx = size / 2;
  const cy = size / 2;
  const sweep = 270;
  const startA = 135;

  const toRad = (d: number) => (d * Math.PI) / 180;
  const bgEnd = startA + sweep;

  const bgStart = { x: cx + r * Math.cos(toRad(bgEnd)), y: cy + r * Math.sin(toRad(bgEnd)) };
  const bgEndP = { x: cx + r * Math.cos(toRad(startA)), y: cy + r * Math.sin(toRad(startA)) };
  const bgArc = `M ${bgStart.x} ${bgStart.y} A ${r} ${r} 0 1 0 ${bgEndP.x} ${bgEndP.y}`;

  // Calculate arc length (circumference of 270 deg of radius 20)
  // C = 2 * PI * r = 2 * PI * 20 = 125.66
  // Sweep fraction = 270 / 360 = 0.75
  // We can animate the path using Framer Motion's motion.path component and standard animate properties
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`}
      style={{ position: 'absolute', top: 0, left: 0, overflow: 'visible' }}>
      {/* Background Track */}
      <path d={bgArc} fill="none" stroke="rgba(30,40,55,0.4)" strokeWidth="2" strokeLinecap="round" />

      {/* Active Foreground Track */}
      {pct > 0 && (
        <motion.path
          d={bgArc}
          fill="none"
          stroke={color}
          strokeWidth="2.5"
          strokeLinecap="round"
          style={{ filter: `drop-shadow(0 0 3px ${color})` }}
          initial={{ pathLength: 0 }}
          animate={{ pathLength: pct / 100 }}
          transition={{ type: 'spring', stiffness: 50, damping: 15 }}
          opacity={0.85}
        />
      )}
    </svg>
  );
}

export function FanPanel({ activeMode = 'standard', onModeChange, rpm, percent }: FanPanelProps) {
  const active = modes.find(m => m.id === activeMode) ?? modes[1];
  const displayPercent = percent ?? null;

  return (
    <PanelFrame title="Fan Speed">
      {/* RPM display */}
      <div className="ac-fan-rpm">
        <span className="ac-fan-rpm__value" style={{ color: active.color, textShadow: `0 0 10px ${active.color}40` }}>
          {rpm != null && rpm > 0 ? <AnimatedRPM value={rpm} /> : 'N/A'}
        </span>
        <span className="ac-fan-rpm__unit">RPM</span>
        <span className="ac-fan-rpm__pct">
          {displayPercent != null ? <AnimatedPct value={displayPercent} /> : 'N/A'}
        </span>
      </div>

      {/* Mode cards */}
      <div className="ac-fan-modes">
        {modes.map((mode) => {
          const isActive = activeMode === mode.id;
          return (
            <motion.button
              key={mode.id}
              className={`ac-fan-mode ${isActive ? 'ac-fan-mode--active' : ''}`}
              onClick={() => onModeChange?.(mode.id)}
              whileHover={{ scale: 1.04, backgroundColor: 'rgba(255, 255, 255, 0.04)' }}
              whileTap={{ scale: 0.96 }}
              animate={{
                borderColor: isActive ? `${mode.color}50` : 'rgba(255, 255, 255, 0.0)',
                boxShadow: isActive
                  ? `0 0 12px ${mode.color}20, inset 0 0 8px ${mode.color}08`
                  : '0 0 0px rgba(0,0,0,0)',
              }}
              transition={{ type: 'spring', stiffness: 100, damping: 20 }}
              style={{
                position: 'relative',
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                gap: '6px',
                cursor: 'pointer',
                padding: '8px 8px 6px',
                borderRadius: '8px',
                background: 'transparent',
                border: '1px solid transparent',
              }}
            >
              <div style={{ position: 'relative', width: 48, height: 48 }}>
                <SpeedArc pct={mode.pct} color={mode.color} />
                <FanIcon speed={mode.speed} color={mode.color} />
              </div>
              <motion.span
                className="ac-fan-mode__label"
                animate={{ color: isActive ? mode.color : 'var(--ac-text-secondary)' }}
                transition={{ duration: 0.2 }}
                style={{
                  fontSize: '9px',
                  fontWeight: 600,
                  letterSpacing: '0.4px',
                  textTransform: 'uppercase',
                }}
              >
                {mode.label}
              </motion.span>
            </motion.button>
          );
        })}
      </div>
    </PanelFrame>
  );
}
