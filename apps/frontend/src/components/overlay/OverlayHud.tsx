import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

/* ================================================================
   OverlayHud — the content rendered inside the transparent, always-
   on-top `overlay` window (see backend `overlay/window.rs`).

   It polls `get_overlay_data` once per second and renders the enabled
   metrics grouped by GPU. When the backend says the overlay should not
   be shown (disabled, or auto-hidden because no game is running) it
   renders nothing, leaving the window fully transparent.
   ================================================================ */

type OverlayTuple = [string, string, number]; // [gpuName, label, value]

interface OverlayPayload {
  running: boolean;
  should_show: boolean;
  data: OverlayTuple[];
}

const EMPTY: OverlayPayload = { running: false, should_show: false, data: [] };

/** Format a metric value compactly based on its label. */
function formatValue(label: string, value: number): string {
  const l = label.toLowerCase();
  if (l.includes('temp')) return `${Math.round(value)}°C`;
  if (l.includes('fan')) return `${Math.round(value)}%`;
  if (l.includes('power')) return `${Math.round(value)}W`;
  if (l.includes('volt')) return `${Math.round(value)}mV`;
  if (l.includes('core') || l.includes('clock') || l.includes('mem')) {
    // Utilisation (%) vs clock (MHz): clocks are large, utilisation <= 100.
    return value > 100 ? `${Math.round(value)}MHz` : `${Math.round(value)}%`;
  }
  if (l.includes('gpu') || l.includes('usage') || l.includes('util')) return `${Math.round(value)}%`;
  return `${Math.round(value)}`;
}

export function OverlayHud() {
  const [payload, setPayload] = useState<OverlayPayload>(EMPTY);

  useEffect(() => {
    // The overlay window must be see-through; strip any app background.
    document.documentElement.style.background = 'transparent';
    document.body.style.background = 'transparent';
    document.body.style.overflow = 'hidden';

    let cancelled = false;
    const poll = async () => {
      try {
        const data = await invoke<OverlayPayload>('get_overlay_data');
        if (!cancelled) setPayload(data);
      } catch {
        /* backend not reachable yet — keep last known state */
      }
    };
    poll();
    const id = setInterval(poll, 250);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const groups = useMemo(() => {
    const byGpu = new Map<string, { label: string; value: number }[]>();
    for (const [gpu, label, value] of payload.data) {
      if (!byGpu.has(gpu)) byGpu.set(gpu, []);
      byGpu.get(gpu)!.push({ label, value });
    }
    return Array.from(byGpu.entries());
  }, [payload.data]);

  if (!payload.should_show || groups.length === 0) return null;

  return (
    <div style={styles.wrap}>
      {groups.map(([gpu, metrics]) => (
        <div key={gpu} style={styles.card}>
          <div style={styles.title}>{gpu.replace(/^NVIDIA\s+/i, '')}</div>
          {metrics.map((m, i) => (
            <div key={`${m.label}-${i}`} style={styles.row}>
              <span style={styles.label}>{m.label}</span>
              <span style={styles.value}>{formatValue(m.label, m.value)}</span>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  wrap: {
    display: 'flex',
    flexDirection: 'column',
    gap: 8,
    padding: 8,
    fontFamily: "'Segoe UI', system-ui, sans-serif",
    userSelect: 'none',
  },
  card: {
    background: 'rgba(10, 12, 18, 0.72)',
    border: '1px solid rgba(80, 200, 255, 0.25)',
    borderRadius: 10,
    padding: '8px 12px',
    backdropFilter: 'blur(6px)',
    boxShadow: '0 4px 18px rgba(0,0,0,0.45)',
  },
  title: {
    color: '#7dd3fc',
    fontSize: 11,
    fontWeight: 700,
    letterSpacing: 0.5,
    textTransform: 'uppercase',
    marginBottom: 4,
  },
  row: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'baseline',
    gap: 16,
    lineHeight: 1.5,
  },
  label: { color: '#94a3b8', fontSize: 12 },
  value: {
    color: '#f1f5f9',
    fontSize: 13,
    fontWeight: 600,
    fontVariantNumeric: 'tabular-nums',
  },
};
