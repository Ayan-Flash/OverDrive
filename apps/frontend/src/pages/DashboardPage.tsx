import { useEffect, useState } from 'react';
import { useGpuStore, useUiStore } from '../stores';
import { useCpuData } from '../components/armoury/useCpuData';
import { FanPanel, GameLauncher } from '../components/armoury';
import { RealtimeChart } from '../components/feature/RealtimeChart';

function display(value: number | null | undefined, unit = '', decimals = 0) {
  if (value == null || !Number.isFinite(value)) return '—';
  return value.toFixed(decimals) + unit;
}

function clampPercent(value: number | null | undefined) {
  if (value == null || !Number.isFinite(value)) return 0;
  return Math.min(100, Math.max(0, value));
}

function Metric({ label, value, unit }: { label: string; value: string; unit?: string }) {
  return (
    <div className="overview-metric">
      <span>{label}</span>
      <strong>{value}{unit && <small>{unit}</small>}</strong>
    </div>
  );
}

export function DashboardPage() {
  const gpus = useGpuStore((state) => state.gpus);
  const currentData = useGpuStore((state) => state.currentData);
  const history = useGpuStore((state) => state.history);
  const error = useGpuStore((state) => state.error);
  const fetchData = useGpuStore((state) => state.fetchData);
  const fetchHistory = useGpuStore((state) => state.fetchHistory);
  const setFanSpeed = useGpuStore((state) => state.setFanSpeed);
  const selectedGpuId = useUiStore((state) => state.selectedGpuId);
  const setSelectedGpu = useUiStore((state) => state.setSelectedGpu);
  const navigateTo = useUiStore((state) => state.navigateTo);
  const cpu = useCpuData();
  const [fanMode, setFanMode] = useState<'silence' | 'standard' | 'turbo' | 'full'>('standard');

  useEffect(() => {
    if (!selectedGpuId) return;
    void fetchData(selectedGpuId);
    const interval = setInterval(() => void fetchData(selectedGpuId), 1000);
    return () => clearInterval(interval);
  }, [selectedGpuId, fetchData]);

  useEffect(() => {
    if (selectedGpuId) void fetchHistory(selectedGpuId, 48);
  }, [selectedGpuId, fetchHistory]);

  const handleFanModeChange = async (mode: string) => {
    if (mode !== 'silence' && mode !== 'standard' && mode !== 'turbo' && mode !== 'full') return;
    setFanMode(mode);
    if (!selectedGpuId) return;
    const speed = mode === 'silence' ? 0 : mode === 'standard' ? 35 : mode === 'turbo' ? 70 : 100;
    await setFanSpeed(selectedGpuId, speed);
  };

  const gpu = selectedGpuId ? currentData.get(selectedGpuId) : undefined;
  const selectedGpu = gpus.find((item) => item.id === selectedGpuId);
  const gpuHistory = selectedGpuId ? history.get(selectedGpuId) ?? [] : [];
  const hasCpu = cpu.model !== 'CPU';
  const hasHardwareData = hasCpu || Boolean(gpu);

  return (
    <main className="ac-overview">
      <section className="overview-grid" aria-label="System overview">
        <article className="overview-card overview-card--system">
          <div className="overview-card__topline">
            <span className="overview-kicker">SYSTEM SNAPSHOT</span>
            <span className={'overview-status ' + (hasHardwareData ? 'overview-status--online' : '')}>
              <span className="overview-status__dot" />
              {hasHardwareData ? 'Telemetry available' : 'Waiting for hardware'}
            </span>
          </div>
          <div className="overview-system__body">
            <div>
              <h2>Your system</h2>
              <p>Live hardware readings from this PC.</p>
            </div>
            <div className="overview-system__identity">
              <div className="overview-identity-row">
                <span className="overview-identity-icon" aria-hidden="true">C</span>
                <span><small>PROCESSOR</small><strong>{hasCpu ? cpu.model : 'Detecting processor…'}</strong></span>
              </div>
              <div className="overview-identity-row">
                <span className="overview-identity-icon overview-identity-icon--teal" aria-hidden="true">G</span>
                <span><small>GRAPHICS</small><strong>{selectedGpu?.name ?? (gpus.length ? 'Select a GPU' : 'No GPU detected')}</strong></span>
              </div>
            </div>
          </div>
          <button className="overview-action" onClick={() => navigateTo('control')}>
            Open hardware controls <span aria-hidden="true">↗</span>
          </button>
        </article>

        <article className="overview-card overview-card--cpu">
          <div className="overview-card__heading">
            <div>
              <span className="overview-kicker">PROCESSOR</span>
              <h2>{hasCpu ? cpu.model : 'CPU'}</h2>
            </div>
            <span className="overview-chip">CPU</span>
          </div>
          <div className="overview-primary-reading">
            <strong>{display(hasCpu ? cpu.usage : null, '%')}</strong>
            <span>Utilization</span>
          </div>
          <div className="overview-progress" role="img" aria-label={'CPU utilization ' + display(hasCpu ? cpu.usage : null, '%')}>
            <span style={{ width: clampPercent(hasCpu ? cpu.usage : null) + '%' }} />
          </div>
          <div className="overview-metrics">
            <Metric label="Temperature" value={display(cpu.temperature, '°')} />
            <Metric label="Frequency" value={display(cpu.frequency, ' MHz')} />
            <Metric label="Cores / threads" value={cpu.logicalCores ? String(cpu.physicalCores || '—') + ' / ' + cpu.logicalCores : '—'} />
          </div>
        </article>

        <article className="overview-card overview-card--gpu">
          <div className="overview-card__heading">
            <div>
              <span className="overview-kicker">GRAPHICS</span>
              <h2>{selectedGpu?.name ?? 'GPU telemetry'}</h2>
            </div>
            {gpus.length > 1 && (
              <label className="overview-select-label">
                <span className="sr-only">Select graphics card</span>
                <select value={selectedGpuId ?? ''} onChange={(event) => setSelectedGpu(event.target.value)} aria-label="Select graphics card">
                  {gpus.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}
                </select>
              </label>
            )}
          </div>
          {gpu ? (
            <>
              <div className="overview-primary-reading">
                <strong>{display(gpu.temperature_celsius, '°')}</strong>
                <span>GPU temperature</span>
              </div>
              <div className="overview-progress overview-progress--cyan" role="img" aria-label={'GPU utilization ' + display(gpu.core_utilization_available === false ? null : gpu.core_utilization_percent, '%')}>
                <span style={{ width: clampPercent(gpu.core_utilization_available === false ? null : gpu.core_utilization_percent) + '%' }} />
              </div>
              <div className="overview-metrics overview-metrics--gpu">
                <Metric label="Utilization" value={display(gpu.core_utilization_available === false ? null : gpu.core_utilization_percent, '%')} />
                <Metric label="VRAM" value={gpu.memory_total_mb ? (gpu.memory_used_mb / 1024).toFixed(1) + ' / ' + (gpu.memory_total_mb / 1024).toFixed(1) : '—'} unit=" GB" />
                <Metric label="Core clock" value={display(gpu.core_clock_mhz, ' MHz')} />
                <Metric label="Board power" value={display(gpu.power_watts, ' W')} />
                <Metric label="Fan speed" value={display(gpu.fan_speed_percent, '%')} />
              </div>
            </>
          ) : (
            <div className="overview-unavailable">
              <span>{gpus.length ? 'Waiting for the first GPU reading…' : 'No compatible GPU was detected.'}</span>
              {error && <span className="overview-error">{error}</span>}
              {selectedGpuId && <button onClick={() => void fetchData(selectedGpuId)}>Retry reading</button>}
            </div>
          )}
        </article>

        <article className="overview-card overview-card--history">
          <div className="overview-card__heading">
            <div>
              <span className="overview-kicker">RECENT HISTORY</span>
              <h2>GPU temperature</h2>
            </div>
            <span className="overview-chart-unit">°C</span>
          </div>
          <div className="overview-history-chart">
            <RealtimeChart data={gpuHistory} metric="temp" color="#b78af7" label="Temperature" unit="°C" height={112} />
          </div>
          <p className="overview-card__note">Historical readings provided by the existing GPU monitor.</p>
        </article>

        <section className="overview-card overview-card--shortcuts" aria-labelledby="overview-shortcuts-heading">
          <div className="overview-card__heading">
            <div>
              <span className="overview-kicker">TOOLS</span>
              <h2 id="overview-shortcuts-heading">Quick access</h2>
            </div>
          </div>
          <div className="overview-shortcut-list">
            <button onClick={() => navigateTo('reports')}><span>Performance Lab</span><small>Saved reports and analysis</small><b aria-hidden="true">↗</b></button>
            <button onClick={() => navigateTo('ai')}><span>AI Insights</span><small>Anomalies and tuning suggestions</small><b aria-hidden="true">↗</b></button>
            <button onClick={() => navigateTo('alerts')}><span>System alerts</span><small>Thresholds and alert history</small><b aria-hidden="true">↗</b></button>
            <button onClick={() => navigateTo('overlay')}><span>Gaming overlay</span><small>On-screen hardware metrics</small><b aria-hidden="true">↗</b></button>
          </div>
        </section>

        <div className="overview-tool-slot overview-tool-slot--fan">
          <FanPanel
            activeMode={fanMode}
            onModeChange={handleFanModeChange}
            rpm={gpu && gpu.fan_rpm > 0 ? gpu.fan_rpm : null}
            percent={gpu ? gpu.fan_speed_percent : null}
          />
        </div>
        <div className="overview-tool-slot overview-tool-slot--games">
          <GameLauncher />
        </div>
      </section>
    </main>
  );
}

