import { useEffect, useMemo, useRef } from 'react';
import { motion, useSpring, useTransform } from 'framer-motion';
import type { MotionValue } from 'framer-motion';

interface CpuGaugeProps {
  value: number;
  maxValue?: number;
  size?: number;
}

const SWEEP = 270;
const START = 135;
const TICKS = 72;
const V = 220;
const CX = V / 2;
const CY = V / 2;

function polar(r: number, deg: number) {
  const rad = (deg * Math.PI) / 180;
  return { x: CX + r * Math.cos(rad), y: CY + r * Math.sin(rad) };
}

function arcPath(r: number, a1: number, a2: number) {
  const s = polar(r, a2);
  const e = polar(r, a1);
  const large = a2 - a1 > 180 ? 1 : 0;
  return `M ${s.x} ${s.y} A ${r} ${r} 0 ${large} 0 ${e.x} ${e.y}`;
}

const FULL_ARC = arcPath(90, START, START + SWEEP);

function Needle({ frac, color, isBoosting }: { frac: MotionValue<number>; color: string; isBoosting: boolean }) {
  const rotation = useTransform(frac, [0, 1], [START, START + SWEEP]);

  return (
    <motion.g
      style={{
        transformOrigin: `${CX}px ${CY}px`,
        rotate: rotation,
        filter: `drop-shadow(0 0 6px ${color})`,
      }}
    >
      <polygon
        points={`${CX + 96},${CY} ${CX},${CY - 8} ${CX + 75},${CY} ${CX},${CY + 8}`}
        fill={`hsl(200, 95%, 70%)`} opacity="0.9"
      />
      <motion.circle
        cx={CX + 96}
        cy={CY}
        r="3"
        fill="#ffffff"
        animate={isBoosting ? {
          scale: [1, 1.8, 1],
          fill: ['#ffffff', '#00ffcc', '#ffffff']
        } : { scale: 1, fill: '#ffffff' }}
        transition={{ repeat: Infinity, duration: 0.8, ease: "easeInOut" }}
        style={{ transformOrigin: `${CX + 96}px ${CY}px` }}
        filter="url(#acGlow)"
      />
    </motion.g>
  );
}

function Tick({ i, frac, color }: { i: number; frac: MotionValue<number>; color: string }) {
  const isMajor = i % 12 === 0;
  const isMid = i % 6 === 0 && !isMajor;
  const len = isMajor ? 12 : isMid ? 8 : 5;
  const a = START + (i / TICKS) * SWEEP;
  const p1 = polar(98, a);
  const p2 = polar(98 - len, a);
  
  const threshold = i / TICKS;
  const opacity = useTransform(frac, [threshold - 0.015, threshold], [0.15, 0.95]);
  const strokeColor = useTransform(
    frac,
    [threshold - 0.015, threshold],
    ['rgba(35, 45, 60, 0.3)', color]
  );
  const strokeWidth = isMajor ? 3.0 : isMid ? 2.0 : 1.0;

  return (
    <g>
      <line
        x1={p1.x}
        y1={p1.y}
        x2={p2.x}
        y2={p2.y}
        stroke="rgba(25, 32, 48, 0.6)"
        strokeWidth={strokeWidth}
        strokeLinecap="butt"
      />
      <motion.line
        x1={p1.x}
        y1={p1.y}
        x2={p2.x}
        y2={p2.y}
        stroke={strokeColor}
        strokeWidth={strokeWidth}
        strokeLinecap="butt"
        style={{ opacity }}
        filter={isMajor ? 'url(#acGlow)' : undefined}
      />
    </g>
  );
}

function Dot({ i, frac }: { i: number; frac: MotionValue<number> }) {
  const a = START + (i / 48) * SWEEP;
  const p = polar(102, a);
  const fill = useTransform(
    frac,
    [i / 48 - 0.03, i / 48],
    ['rgba(40,50,70,0.3)', 'rgba(60,160,240,0.9)']
  );
  const opacity = useTransform(frac, [i / 48 - 0.03, i / 48], [0.3, 0.9]);

  return (
    <motion.circle
      cx={p.x}
      cy={p.y}
      r={1.2}
      style={{ fill, opacity }}
    />
  );
}

export function CpuGauge({ value, maxValue = 5000, size = 340 }: CpuGaugeProps) {
  const rawFrac = Math.min(Math.max(value / maxValue, 0), 1);
  const frac = useSpring(rawFrac, { stiffness: 70, damping: 18, mass: 0.5 });
  useEffect(() => { frac.set(rawFrac); }, [rawFrac, frac]);

  const isBoosting = rawFrac > 0.75;
  const hue = 200;
  const color = `hsl(${hue}, 85%, 55%)`;
  const colorBright = `hsl(${hue}, 95%, 70%)`;

  const labelCount = 6;
  const scaleLabels = useMemo(() => {
    const nodes: React.ReactNode[] = [];
    for (let i = 0; i <= labelCount; i++) {
      const a = START + (i / labelCount) * SWEEP;
      const p = polar(108, a);
      const mhz = Math.round((i / labelCount) * maxValue);
      nodes.push(
        <text key={i} x={p.x} y={p.y} textAnchor="middle" dominantBaseline="central"
          fill="rgba(130,150,180,0.6)" fontSize="7" fontWeight="500"
          fontFamily="'Segoe UI', system-ui, sans-serif">
          {mhz >= 1000 ? `${(mhz / 1000).toFixed(mhz % 1000 === 0 ? 0 : 1)}k` : mhz}
        </text>
      );
    }
    return nodes;
  }, [maxValue]);

  const ticks = useMemo(() => {
    const nodes: React.ReactNode[] = [];
    for (let i = 0; i <= TICKS; i++) {
      nodes.push(<Tick key={i} i={i} frac={frac} color={color} />);
    }
    return nodes;
  }, [frac, color]);

  const dots = useMemo(() => {
    const nodes: React.ReactNode[] = [];
    for (let i = 0; i < 48; i++) {
      nodes.push(<Dot key={i} i={i} frac={frac} />);
    }
    return nodes;
  }, [frac]);

  const circuits = useMemo(() => {
    const nodes: React.ReactNode[] = [];
    [160, 195, 235, 280, 325, 365].forEach((a, i) => {
      nodes.push(
        <line key={`c${i}`} x1={polar(57, a).x} y1={polar(57, a).y}
          x2={polar(68, a).x} y2={polar(68, a).y}
          stroke="rgba(50,70,100,0.2)" strokeWidth="0.5" />,
        <circle key={`cn${i}`} cx={polar(68, a).x} cy={polar(68, a).y} r="1.2"
          fill="none" stroke="rgba(50,80,120,0.25)" strokeWidth="0.5" />
      );
    });
    nodes.push(
      <line key="ch1" x1={CX - 30} y1={CY + 22} x2={CX + 30} y2={CY + 22}
        stroke="rgba(50,70,100,0.12)" strokeWidth="0.4" />,
      <line key="ch2" x1={CX - 20} y1={CY + 26} x2={CX + 20} y2={CY + 26}
        stroke="rgba(50,70,100,0.08)" strokeWidth="0.4" />
    );
    return nodes;
  }, []);

  const textRef = useRef<SVGTextElement>(null);

  useEffect(() => {
    const unsubscribe = frac.on("change", (v) => {
      if (textRef.current) {
        textRef.current.textContent = Math.round(v * maxValue).toString();
      }
    });
    if (textRef.current) {
      textRef.current.textContent = Math.round(rawFrac * maxValue).toString();
    }
    return unsubscribe;
  }, [frac, maxValue, rawFrac]);

  return (
    <motion.div
      className="ac-gauge-container"
      style={{ width: size, height: size }}
      initial={{ opacity: 0, scale: 0.85 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.6, ease: [0.16, 1, 0.3, 1] }}
    >
      <svg viewBox={`0 0 ${V} ${V}`} width={size} height={size} style={{ overflow: 'visible' }}>
        <defs>
          <linearGradient id="acGrad" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#1a66cc" />
            <stop offset="35%" stopColor="#00ccdd" />
            <stop offset="60%" stopColor="#33cc88" />
            <stop offset="80%" stopColor="#ddaa22" />
            <stop offset="100%" stopColor="#cc2222" />
          </linearGradient>
          <filter id="acGlow">
            <feGaussianBlur stdDeviation="3" />
            <feMerge><feMergeNode /><feMergeNode in="SourceGraphic" /></feMerge>
          </filter>
          <filter id="acGlowStrong">
            <feGaussianBlur stdDeviation="5" />
            <feMerge><feMergeNode /><feMergeNode in="SourceGraphic" /></feMerge>
          </filter>
          <radialGradient id="acCenter" cx="50%" cy="40%" r="60%">
            <stop offset="0%" stopColor="#1a1e2e" />
            <stop offset="100%" stopColor="#0e1018" />
          </radialGradient>
        </defs>

        {/* Rings */}
        <path d={arcPath(100, START, START + SWEEP)} fill="none" stroke="rgba(30,40,60,0.3)" strokeWidth="1" />
        <path d={arcPath(90, START, START + SWEEP)} fill="none" stroke="rgba(25,30,45,0.6)" strokeWidth="7" strokeLinecap="round" />
        <path d={arcPath(82, START, START + SWEEP)} fill="none" stroke="rgba(20,28,40,0.4)" strokeWidth="2" />
        <path d={arcPath(72, START, START + SWEEP)} fill="none" stroke="rgba(25,35,50,0.3)" strokeWidth="1.5" />
        <circle cx={CX} cy={CY} r={55} fill="none" stroke="rgba(30,45,65,0.2)" strokeWidth="0.8" strokeDasharray="3 2" />

        {/* Circuit decorations */}
        {circuits}
        <polygon points={Array.from({ length: 6 }, (_, i) => {
          const a = 60 * i - 30;
          return `${polar(14, a).x},${CY - 36 + polar(14, a).y - CY}`;
        }).join(' ')} fill="none" stroke="rgba(40,60,90,0.15)" strokeWidth="0.6" />

        {/* Active arc — spring-driven pathLength, styled as segmented tachometer */}
        <motion.path d={FULL_ARC} fill="none" stroke="url(#acGrad)" strokeWidth="10"
          strokeLinecap="butt" strokeDasharray="4 2.5" style={{ pathLength: frac, opacity: 0.15 }} filter="url(#acGlowStrong)" />
        <motion.path d={FULL_ARC} fill="none" stroke="url(#acGrad)" strokeWidth="6"
          strokeLinecap="butt" strokeDasharray="4 2.5" style={{ pathLength: frac }} />
        <motion.path d={arcPath(87, START, START + SWEEP)} fill="none" stroke={colorBright}
          strokeWidth="0.8" strokeLinecap="butt" strokeDasharray="1 1" style={{ pathLength: frac, opacity: 0.4 }} />

        {/* Ticks + labels + dots */}
        {ticks}
        {scaleLabels}
        {dots}

        {/* Needle */}
        <Needle frac={frac} color={color} isBoosting={isBoosting} />

        {/* Center cap */}
        <motion.circle cx={CX} cy={CY} r="6" fill="#1a1e2e"
          animate={{ stroke: color }} transition={{ duration: 0.6 }} strokeWidth="1.5" />
        <circle cx={CX} cy={CY} r="2.5" fill={colorBright} opacity="0.7" />

        {/* Center readout */}
        <circle cx={CX} cy={CY} r={51} fill="url(#acCenter)" opacity="0.9" />

        {/* Animated value */}
        <motion.g
          animate={isBoosting ? {
            scale: [1, 1.05, 1],
          } : { scale: 1 }}
          transition={{ repeat: Infinity, duration: 1.0, ease: "easeInOut" }}
          style={{ transformOrigin: `${CX}px ${CY - 6}px` }}
        >
          <text ref={textRef} x={CX} y={CY - 6} textAnchor="middle" dominantBaseline="central"
            fill={isBoosting ? '#ffffff' : '#f0f2f8'} fontSize="36" fontWeight="900"
            fontFamily="'JetBrains Mono', 'Fira Code', monospace"
            style={{ filter: isBoosting ? `drop-shadow(0 0 12px ${colorBright})` : `drop-shadow(0 0 6px ${color})` }}
          />
        </motion.g>

        <text x={CX} y={CY + 16} textAnchor="middle" fill="rgba(140,160,190,0.8)"
          fontSize="11" fontWeight="600" fontFamily="'Segoe UI', system-ui, sans-serif" letterSpacing="1.5">MHz</text>
        <text x={CX} y={CY + 28} textAnchor="middle" fill="rgba(100,120,150,0.5)"
          fontSize="9" fontWeight="500" fontFamily="'Segoe UI', system-ui, sans-serif" letterSpacing="2">CPU</text>

        {/* Scale end labels */}
        <text x={polar(64, START + 12).x} y={polar(64, START + 12).y}
          textAnchor="middle" fill="rgba(100,130,170,0.5)" fontSize="8" fontWeight="600">0</text>
        <text x={polar(64, START + SWEEP - 12).x} y={polar(64, START + SWEEP - 12).y}
          textAnchor="middle" fill="rgba(100,130,170,0.5)" fontSize="8" fontWeight="600">Max</text>

        {/* Pulse ring */}
        <circle cx={CX} cy={CY} r={90} fill="none" stroke={color} strokeWidth="0.5" opacity="0.15">
          <animate attributeName="r" values="90;93;90" dur="2s" repeatCount="indefinite" />
          <animate attributeName="opacity" values="0.15;0.05;0.15" dur="2s" repeatCount="indefinite" />
        </circle>
      </svg>
    </motion.div>
  );
}
