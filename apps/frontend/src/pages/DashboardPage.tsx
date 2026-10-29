import { useEffect, useState } from 'react';
import type { ReactNode } from 'react';
import { alertService, monitoringService, overlayService } from '../services';
import { useAlertStore, useGpuStore, useUiStore } from '../stores';
import { useCpuData } from '../components/overdrive/useCpuData';
import { SegmentGauge } from '../components/overdrive/SegmentGauge';
import { systemTelemetryService, type RamTelemetry, type StorageTelemetry } from '../services/cpuService';

type MonitoringState = 'checking' | 'running' | 'stopped' | 'unavailable';

function reading(value: number | null | undefined, unit = '', precision = 0) {
  if (value == null || !Number.isFinite(value)) return 'Unavailable';
  return value.toFixed(precision) + unit;
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="overview-metric">
      <span>{label}</span>
      <strong>{value}</strong>
    </div>
  );
}

/** A trace of received samples only; missing data never becomes a zero reading. */
export function CpuActivity({ usage, sampledAt, available }: {
  usage: number;
  sampledAt: number | null;
  available: boolean;
}) {
  const [history, setHistory] = useState<{ at: number; value: number }[]>([]);
  const live = available && sampledAt != null && Number.isFinite(usage);
  useEffect(() => {
    if (!available || sampledAt == null || !Number.isFinite(usage)) {
      setHistory([]);
      return;
    }
    setHistory((previous) => {
      if (previous[previous.length - 1]?.at === sampledAt) return previous;
      const recent = previous.filter((point) => point.at > sampledAt - 60000 && point.at < sampledAt);
      return [...recent.slice(-119), { at: sampledAt, value: Math.min(100, Math.max(0, usage)) }];
    });
  }, [available, sampledAt, usage]);

  const points = live ? history : [];
  const end = points[points.length - 1]?.at ?? 0;
  const coordinates = points.map((point) => ({
    x: 300 - ((end - point.at) / 60000) * 300,
    y: 58 - (point.value / 100) * 54,
  }));
  const last = coordinates[coordinates.length - 1];
  const peak = points.length ? Math.max(...points.map((point) => point.value)) : null;

  return (
    <figure className="overview-cpu-activity">
      <figcaption>
        <span>CPU activity</span>
        <span className={'overview-cpu-activity__status' + (live ? ' is-live' : '')}>
          <i aria-hidden="true" />{live ? 'Live' : 'Waiting for readings'}
        </span>
      </figcaption>
      <div className="overview-cpu-activity__plot">
        <svg viewBox="0 0 304 62" preserveAspectRatio="none" role="img"
          aria-label={live ? `CPU utilization history, last 60 seconds. Current ${usage.toFixed(1)} percent. Peak ${peak?.toFixed(1) ?? '—'} percent.` : 'CPU utilization history unavailable'}>
          <path className="overview-cpu-activity__grid" d="M0 4H304M0 31H304M0 58H304M75 4V58M150 4V58M225 4V58" />
          {coordinates.length > 1 && <polyline className="overview-cpu-activity__line"
            points={coordinates.map(({ x, y }) => `${x},${y}`).join(' ')} />}
          {last && <circle className="overview-cpu-activity__point" cx={last.x} cy={last.y} r="2.5" />}
        </svg>
        {!live && <span className="overview-cpu-activity__empty">History resumes when telemetry reconnects</span>}
      </div>
      <div className="overview-cpu-activity__axis">
        <span>60 seconds</span><span>{peak == null ? '0–100%' : `Peak ${peak.toFixed(0)}%`} · Now</span>
      </div>
    </figure>
  );
}

function HardwareIcon({ kind }: { kind: 'cpu' | 'gpu' | 'ram' | 'storage' }) {
  const paths: Record<typeof kind, ReactNode> = {
    cpu: (
      <>
        <rect x="6" y="6" width="12" height="12" rx="2" />
        <rect x="9" y="9" width="6" height="6" rx="1" />
        <path d="M9 2v4m6-4v4M9 18v4m6-4v4M2 9h4m-4 6h4m12-6h4m-4 6h4" />
      </>
    ),
    gpu: (
      <>
        <rect x="3" y="5" width="18" height="13" rx="2" />
        <path d="M7 21h10m-5-3v3M7 9h8m-8 3h5" />
      </>
    ),
    ram: (
      <>
        <rect x="3" y="7" width="18" height="10" rx="2" />
        <path d="M7 7V4m5 3V4m5 3V4M7 17v3m5-3v3m5-3v3M7 11h2m3 0h2m3 0h1" />
      </>
    ),
    storage: (
      <>
        <ellipse cx="12" cy="5" rx="8" ry="3" />
        <path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" />
      </>
    ),
  };

  return (
    <span className="overview-device-icon" aria-hidden="true">
      <svg
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      >
        {paths[kind]}
      </svg>
    </span>
  );
}

function ActionIcon({ kind }: { kind: 'analysis' | 'optimize' | 'health' | 'overlay' }) {
  const paths: Record<typeof kind, ReactNode> = {
    analysis: (
      <>
        <path d="M4 19V9m5 10V5m5 14v-7m5 7V3" />
      </>
    ),
    optimize: (
      <>
        <path d="M13 2 4 13h7l-1 9 10-12h-7l1-8Z" />
      </>
    ),
    health: (
      <>
        <path d="M20.8 8.7c0 5.2-8.8 11-8.8 11s-8.8-5.8-8.8-11A4.7 4.7 0 0 1 12 6a4.7 4.7 0 0 1 8.8 2.7Z" />
      </>
    ),
    overlay: (
      <>
        <rect x="3" y="4" width="18" height="14" rx="2" />
        <path d="M8 21h8m-4-3v3M7 9h3v3H7zm7 0h3m-3 4h3" />
      </>
    ),
  };
  return (
    <svg
      className="overview-action-icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {paths[kind]}
    </svg>
  );
}

export function DashboardPage() {
  const gpus = useGpuStore((state) => state.gpus);
  const currentData = useGpuStore((state) => state.currentData);
  const error = useGpuStore((state) => state.error);
  const fetchData = useGpuStore((state) => state.fetchData);
  const fetchLiveSample = useGpuStore((state) => state.fetchLiveSample);
  const selectedGpuId = useUiStore((state) => state.selectedGpuId);
  const setSelectedGpu = useUiStore((state) => state.setSelectedGpu);
  const navigateTo = useUiStore((state) => state.navigateTo);
  const setAlerts = useAlertStore((state) => state.setAlerts);

  const cpu = useCpuData();
  const [monitoringState, setMonitoringState] = useState<MonitoringState>('checking');
  const [alertsLoaded, setAlertsLoaded] = useState(false);
  const [games, setGames] = useState<string[]>([]);
  const [gamesLoaded, setGamesLoaded] = useState(false);
  const [gamesAvailable, setGamesAvailable] = useState(true);
  const [ram, setRam] = useState<RamTelemetry | null>(null);
  const [storage, setStorage] = useState<StorageTelemetry[] | null>(null);

  useEffect(() => {
    let active = true;
    const readRam = () => void systemTelemetryService.getRam().then((v) => active && setRam(v)).catch(() => active && setRam(null));
    const readStorage = () => void systemTelemetryService.getStorage().then((v) => active && setStorage(v)).catch(() => active && setStorage(null));
    readRam(); readStorage();
    const ramTimer = setInterval(readRam, 1000);
    const storageTimer = setInterval(readStorage, 60000);
    return () => { active = false; clearInterval(ramTimer); clearInterval(storageTimer); };
  }, []);

  useEffect(() => {
    if (!selectedGpuId) return;
    void fetchLiveSample(selectedGpuId);
    // ~10 Hz display refresh; each fetch is a sub-millisecond cached/live
    // probe on the backend, so this adds effectively no battery/CPU load.
    const interval = setInterval(() => void fetchLiveSample(selectedGpuId), 100);
    return () => clearInterval(interval);
  }, [selectedGpuId, fetchLiveSample]);


  useEffect(() => {
    let active = true;
    void monitoringService
      .isRunning()
      .then((running) => {
        if (active) setMonitoringState(running ? 'running' : 'stopped');
      })
      .catch(() => {
        if (active) setMonitoringState('unavailable');
      });
    void alertService
      .getHistory(100)
      .then((items) => {
        if (active) {
          setAlerts(items);
          setAlertsLoaded(true);
        }
      })
      .catch(() => {
        if (active) setAlertsLoaded(false);
      });
    void overlayService
      .getDetectedGames()
      .then((items) => {
        if (active) {
          setGames(items);
          setGamesLoaded(true);
        }
      })
      .catch(() => {
        if (active) {
          setGamesLoaded(true);
          setGamesAvailable(false);
        }
      });

    return () => {
      active = false;
    };
  }, [setAlerts]);

  const cpuDetected = cpu.model !== 'CPU';
  const cpuHasData = cpu.telemetryAvailable;
  const gpu = selectedGpuId ? currentData.get(selectedGpuId) : undefined;
  const selectedGpu = gpus.find((item) => item.id === selectedGpuId);
  const cpuTemperature = cpuHasData && Number.isFinite(cpu.temperature) ? cpu.temperature : null;
  const gpuTemperature =
    gpu?.temperature_celsius != null && Number.isFinite(gpu.temperature_celsius) ? gpu.temperature_celsius : null;
  const gpuUtilization =
    gpu?.core_utilization_percent != null && Number.isFinite(gpu.core_utilization_percent)
      ? gpu.core_utilization_percent
      : null;
  const gpuVram =
    gpu && gpu.memory_used_mb != null && gpu.memory_total_mb != null &&
    Number.isFinite(gpu.memory_used_mb) &&
    Number.isFinite(gpu.memory_total_mb) &&
    gpu.memory_total_mb > 0
      ? (gpu.memory_used_mb / 1024).toFixed(1) +
        ' / ' +
        (gpu.memory_total_mb / 1024).toFixed(1) +
        ' GB'
      : 'Unavailable';
  const activeWarnings = useAlertStore(
    (state) =>
      state.alerts.filter(
        (alert) =>
          !alert.acknowledged && (alert.severity === 'Warning' || alert.severity === 'Critical'),
      ).length,
  );
  const monitoringLabel =
    monitoringState === 'running'
      ? 'Background monitor active'
      : monitoringState === 'stopped'
        ? 'Background monitor paused'
        : monitoringState === 'checking'
          ? 'Checking monitor service…'
          : 'Monitor status unavailable';

  return (
    <main className="ac-overview">
      <div className="overview-shell">
        <header className="overview-welcome">
          <div className="overview-welcome__copy">
            <span className="overview-eyebrow">HOME</span>
            <h1>Welcome back</h1>
            <p>Here’s how your PC is running today.</p>
            <div className="overview-welcome__hardware">
              <span>
                <i className="overview-welcome__dot overview-welcome__dot--cpu" />
                {cpuDetected ? cpu.model : cpuHasData ? 'Processor' : 'CPU data unavailable'}
              </span>
              <span>
                <i className="overview-welcome__dot" />
                {selectedGpu?.name ?? 'GPU not selected'}
              </span>
            </div>
          </div>
          <div className="overview-welcome__actions">
            <button className="overview-button" onClick={() => navigateTo('settings')}>
              Settings <span aria-hidden="true">›</span>
            </button>
            <button
              className="overview-button overview-button--primary"
              onClick={() => navigateTo('control')}
            >
              My PC <span aria-hidden="true">›</span>
            </button>
          </div>
        </header>

        <div className="overview-workspace">
          <aside className="overview-health" aria-labelledby="overview-health-title">
            <div className="overview-health__heading">
              <span className="overview-health__heart" aria-hidden="true">
                ♡
              </span>
              <h2 id="overview-health-title">System health</h2>
            </div>
            <div className="overview-health__status">
              <span
                className={
                  'overview-health__status-dot ' +
                  (monitoringState === 'running' ? 'overview-health__status-dot--live' : '')
                }
              />
              <span>{monitoringLabel}</span>
            </div>
            <div className="overview-health__visual" aria-hidden="true">
              <div className="overview-health__bar" />
              <div className="overview-health__ticks">
                {Array.from({ length: 22 }, (_, index) => (
                  <i key={index} />
                ))}
              </div>
            </div>
            <div className="overview-health__warning">
              <strong>{alertsLoaded ? activeWarnings : '—'}</strong>
              <span>Unacknowledged warnings</span>
            </div>
            <button className="overview-health__link" onClick={() => navigateTo('alerts')}>
              Review alerts <span aria-hidden="true">›</span>
            </button>
            <p className="overview-health__note">
              The color bar is decorative and does not represent a health score.
            </p>
          </aside>

          <div className="overview-main">
            <section className="overview-hardware-grid" aria-label="Hardware status">
              <article className="overview-hardware-card">
                <div className="overview-hardware-card__top">
                  <HardwareIcon kind="cpu" />
                  <div className="overview-hardware-card__identity">
                    <h2>CPU</h2>
                    <p>
                      {cpuDetected
                        ? cpu.model
                        : cpuHasData
                          ? 'Processor'
                          : 'Processor details unavailable'}
                    </p>
                  </div>
                </div>
                <SegmentGauge
                  value={cpuTemperature}
                  min={0}
                  max={100}
                  unit="°C"
                  label="Temperature"
                  precision={1}
                />
                <div className="overview-hardware-card__metrics">
                  <Metric label="Utilization" value={reading(cpuHasData ? cpu.usage : null, '%')} />
                  <Metric
                    label="Fastest core clock"
                    value={reading(cpuHasData ? cpu.frequency : null, ' MHz')}
                  />
                  <Metric
                    label="Cores / threads"
                    value={
                      cpuHasData && cpu.logicalCores > 0
                        ? cpu.physicalCores + ' / ' + cpu.logicalCores
                        : 'Unavailable'
                    }
                  />
                </div>
                <CpuActivity usage={cpu.usage} sampledAt={cpu.sampledAt} available={cpuHasData} />
                {cpuHasData && cpuTemperature == null && (
                  <p className="overview-cpu-sensor-note">CPU usage is live. Temperature is waiting for hardware sensor access.</p>
                )}
                {Boolean(cpuHasData && cpu.coreTemperatures && cpu.coreTemperatures.length > 0) && (
                  <div className="overview-cpu-core-temperatures" style={{ marginTop: 14 }}>
                    <div style={{ marginBottom: 8, color: 'var(--ac-text-secondary)', fontSize: 11, fontWeight: 600, letterSpacing: '0.05em', textTransform: 'uppercase' }}>
                      CPU core temperatures
                    </div>
                    <div className="overview-hardware-card__metrics">
                      {cpu.coreTemperatures?.map((sensor) => (
                        <Metric
                          key={sensor.name}
                          label={sensor.name}
                          value={reading(sensor.temperature, '°C', 1)}
                        />
                      ))}
                    </div>
                  </div>
                )}
              </article>

              <article className="overview-hardware-card">
                <div className="overview-hardware-card__top">
                  <HardwareIcon kind="gpu" />
                  <div className="overview-hardware-card__identity">
                    <h2>GPU</h2>
                    <p>{selectedGpu?.name ?? 'Graphics card not selected'}</p>
                  </div>
                  {gpus.length > 1 && (
                    <label className="overview-select-label">
                      <span className="sr-only">Select graphics card</span>
                      <select
                        value={selectedGpuId ?? ''}
                        onChange={(event) => setSelectedGpu(event.target.value)}
                        aria-label="Select graphics card"
                      >
                        {gpus.map((item) => (
                          <option key={item.id} value={item.id}>
                            {item.name}
                          </option>
                        ))}
                      </select>
                    </label>
                  )}
                </div>
                {gpu ? (
                  <>
                    <SegmentGauge
                      value={gpuTemperature}
                      min={0}
                      max={100}
                      unit="°C"
                      label="Temperature"
                      precision={1}
                    />
                    <div className="overview-hardware-card__metrics overview-hardware-card__metrics--gpu">
                      <Metric label="Utilization" value={reading(gpuUtilization, '%')} />
                      <Metric label="VRAM" value={gpuVram} />
                      <Metric label="Core clock" value={reading(gpu.core_clock_mhz, ' MHz')} />
                      <Metric label="Power" value={reading(gpu.power_watts, ' W')} />
                      <Metric label="Fan" value={reading(gpu.fan_speed_percent, '%')} />
                    </div>
                    <button
                      className="overview-hardware-card__more"
                      onClick={() => navigateTo('reports')}
                    >
                      GPU history <span aria-hidden="true">›</span>
                    </button>
                  </>
                ) : (
                  <div className="overview-hardware-card__empty">
                    <SegmentGauge value={null} min={0} max={100} unit="°C" label="Temperature" />
                    <p>
                      {gpus.length
                        ? 'Waiting for the first GPU reading.'
                        : 'No supported GPU telemetry provider was found. NVIDIA needs NVML from its driver; AMD and Intel GPU support is not implemented yet.'}
                    </p>
                    {error && (
                      <small className="overview-error">
                        {selectedGpuId
                          ? 'GPU telemetry could not be retrieved.'
                          : 'GPU detection could not be reached.'}
                      </small>
                    )}
                    {selectedGpuId && (
                      <button onClick={() => void fetchData(selectedGpuId)}>Retry reading</button>
                    )}
                  </div>
                )}
              </article>

              <article className="overview-hardware-card">
                <div className="overview-hardware-card__top">
                  <HardwareIcon kind="ram" />
                  <div className="overview-hardware-card__identity">
                    <h2>Memory</h2>
                    <p>System RAM</p>
                  </div>
                </div>
                {ram ? <div className="overview-hardware-card__metrics">
                  <Metric label="Used" value={(ram.used_bytes / 1073741824).toFixed(1) + ' GB'} />
                  <Metric label="Available" value={(ram.available_bytes / 1073741824).toFixed(1) + ' GB'} />
                  <Metric label="Total" value={(ram.total_bytes / 1073741824).toFixed(1) + ' GB'} />
                  <Metric label="Utilization" value={reading(ram.utilization_percent, '%', 1)} />
                </div> : <div className="overview-unavailable-state"><span>System memory telemetry isn’t available.</span><small>Windows did not return a physical memory reading.</small></div>}
              </article>

              <article className="overview-hardware-card">
                <div className="overview-hardware-card__top">
                  <HardwareIcon kind="storage" />
                  <div className="overview-hardware-card__identity">
                    <h2>Storage</h2>
                    <p>Disk capacity and usage</p>
                  </div>
                </div>
                {storage?.length ? <div className="overview-hardware-card__metrics">{storage.map((disk) => <div className="overview-metric" key={disk.volume}>
                  <span>{disk.volume} · {(disk.used_bytes / 1073741824).toFixed(1)} / {(disk.total_bytes / 1073741824).toFixed(1)} GB used · {(disk.utilization_percent).toFixed(1)}%</span>
                  <strong>{(disk.user_available_bytes / 1073741824).toFixed(1)} GB available to you</strong>
                </div>)}</div> : <div className="overview-unavailable-state"><span>Storage telemetry isn’t available.</span><small>{storage === null ? 'Volume capacity could not be read.' : 'No accessible volumes were found.'}</small></div>}
              </article>
            </section>

            <section
              className="overview-quick-actions"
              aria-labelledby="overview-quick-actions-title"
            >
              <h2 id="overview-quick-actions-title">Quick actions</h2>
              <div className="overview-quick-actions__grid">
                <button className="overview-quick-action" onClick={() => navigateTo('reports')}>
                  <ActionIcon kind="analysis" />
                  <span className="overview-quick-action__copy">
                    <strong>Game Analysis</strong>
                    <small>
                      {!gamesLoaded
                        ? 'Checking detected games…'
                        : !gamesAvailable
                          ? 'Game detection unavailable'
                          : games.length
                            ? games.slice(0, 2).join(' · ')
                            : 'No games detected'}
                    </small>
                  </span>
                  <span className="overview-quick-action__arrow" aria-hidden="true">
                    ›
                  </span>
                </button>
                <button className="overview-quick-action" onClick={() => navigateTo('ai')}>
                  <ActionIcon kind="optimize" />
                  <span className="overview-quick-action__copy">
                    <strong>Optimize PC</strong>
                    <small>Insights and supported tuning suggestions</small>
                  </span>
                  <span className="overview-quick-action__arrow" aria-hidden="true">
                    ›
                  </span>
                </button>
                <button className="overview-quick-action" onClick={() => navigateTo('alerts')}>
                  <ActionIcon kind="health" />
                  <span className="overview-quick-action__copy">
                    <strong>System Health</strong>
                    <small>Alerts, thresholds, and history</small>
                  </span>
                  <span className="overview-quick-action__arrow" aria-hidden="true">
                    ›
                  </span>
                </button>
                <button className="overview-quick-action" onClick={() => navigateTo('overlay')}>
                  <ActionIcon kind="overlay" />
                  <span className="overview-quick-action__copy">
                    <strong>Gaming Overlay</strong>
                    <small>Configure on-screen hardware metrics</small>
                  </span>
                  <span className="overview-quick-action__arrow" aria-hidden="true">
                    ›
                  </span>
                </button>
              </div>
            </section>
          </div>
        </div>
      </div>
    </main>
  );
}

