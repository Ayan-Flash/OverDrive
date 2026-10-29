import { useCallback, useEffect, useMemo, useState } from 'react';
import { useGpuStore, useUiStore } from '../stores';
import { controlService, fanCurveService, gpuService, profileService } from '../services';
import type { ControlSnapshot, FanCurvePoint, FanCurveView, GpuControlState, QuickSlotSummary, SnapshotFanMode } from '../services';
import { formatClockSpeed, clamp } from '@common/utils';
import { FanCurveEditor } from '../components/overdrive/FanCurveEditor';

/* ================================================================
   ControlPage — OverDrive Tuning. NVIDIA-style GPU control surfaces:
   fan mode (automatic / custom curve / fixed), core & memory clock
   offsets, power limit, plus a driver-defaults reset. Availability
   follows the capabilities the backend probes live from the driver.
   Sliders apply on release; applied state reloads after each write.
   ================================================================ */

type FanMode = 'automatic' | 'curve' | 'fixed';

/** Mirrors the backend's DEFAULT_FAN_CURVE; only used if the curve view
 *  could not be loaded (e.g. backend unreachable) — the resulting driver
 *  error surfaces through the action banner either way. */
const FALLBACK_CURVE: FanCurvePoint[] = [
  { temperature_celsius: 30, fan_percent: 25 },
  { temperature_celsius: 40, fan_percent: 35 },
  { temperature_celsius: 50, fan_percent: 45 },
  { temperature_celsius: 60, fan_percent: 60 },
  { temperature_celsius: 70, fan_percent: 80 },
  { temperature_celsius: 80, fan_percent: 100 },
];

interface ActionResult {
  ok: boolean;
  message: string;
}

export function ControlPage() {
  const { currentData, gpus } = useGpuStore();
  const { selectedGpuId } = useUiStore();
  const data = selectedGpuId ? currentData.get(selectedGpuId) : null;
  const selectedGpu = gpus.find((gpu) => gpu.id === selectedGpuId);

  const [control, setControl] = useState<GpuControlState | null>(null);
  const [curve, setCurve] = useState<FanCurveView | null>(null);
  const [curveDraft, setCurveDraft] = useState<FanCurvePoint[] | null>(null);
  const [fixedFan, setFixedFan] = useState<number>(50);
  const [coreOffset, setCoreOffset] = useState(0);
  const [memoryOffset, setMemoryOffset] = useState(0);
  const [powerPercent, setPowerPercent] = useState<number | null>(null);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [actionResult, setActionResult] = useState<Record<string, ActionResult>>({});
  const [slots, setSlots] = useState<(QuickSlotSummary | null)[]>([]);
  const [activeSlot, setActiveSlot] = useState<number | null>(null);

  const capabilities = control?.capabilities;
  const caps = {
    fanControl: capabilities?.fan_control ?? false,
    coreOffset: capabilities?.core_clock_offset ?? false,
    memoryOffset: capabilities?.memory_clock_offset ?? false,
    powerLimit: capabilities?.power_limit ?? false,
    voltageOffset: capabilities?.voltage_offset ?? false,
  };

  const reloadControl = useCallback(async () => {
    if (!selectedGpuId) return;
    try {
      const state = await gpuService.getControlState(selectedGpuId);
      setControl(state);
      setCoreOffset(state.core_clock_offset_mhz ?? 0);
      setMemoryOffset(state.memory_clock_offset_mhz ?? 0);
      setPowerPercent(state.power_limit_percent ?? null);
    } catch {
      setControl(null);
    }
  }, [selectedGpuId]);

  const reloadCurve = useCallback(async () => {
    if (!selectedGpuId) return;
    try {
      const state = await fanCurveService.get(selectedGpuId);
      setCurve(state);
      setCurveDraft(null);
      if (state.enabled) {
        setFixedFan(Math.round(state.points[0]?.fan_percent ?? 50));
      } else if (control?.target_fan_speed != null) {
        setFixedFan(Math.round(control.target_fan_speed));
      }
    } catch {
      setCurve(null);
    }
  }, [selectedGpuId, control?.target_fan_speed]);

  const reloadSlots = useCallback(async () => {
    try {
      setSlots(await profileService.slots());
    } catch {
      setSlots([]);
    }
  }, []);

  useEffect(() => {
    reloadControl();
    reloadCurve();
    reloadSlots();
  }, [reloadControl, reloadCurve, reloadSlots]);

  // Live telemetry strip — polls the lightweight live-sample command at
  // ~10 Hz; each fetch is a single sub-millisecond driver probe. Keep the
  // last data when a poll fails intermittently.
  useEffect(() => {
    if (!selectedGpuId) return;
    const poll = async () => {
      try {
        await useGpuStore.getState().fetchLiveSample(selectedGpuId);
      } catch {
        /* keep previous sample */
      }
    };
    poll();
    const id = setInterval(poll, 100);
    return () => clearInterval(id);
  }, [selectedGpuId]);

  const runAction = useCallback(
    async (key: string, action: () => Promise<void>, result: string) => {
      setBusyAction(key);
      try {
        await action();
        setActionResult((prev) => ({ ...prev, [key]: { ok: true, message: result } }));
      } catch (error) {
        const message = humanizeTauriError(error);
        setActionResult((prev) => ({ ...prev, [key]: { ok: false, message } }));
      } finally {
        setBusyAction(null);
        reloadControl();
      }
    },
    [reloadControl],
  );

  const fanMode: FanMode = useMemo(() => {
    if (curve?.enabled) return 'curve';
    if (control?.fan_control_automatic === false) return 'fixed';
    return 'automatic';
  }, [curve?.enabled, control?.fan_control_automatic]);

  const coreLimit = control?.core_clock_offset_limit_mhz ?? 500;
  const memoryLimit = control?.memory_clock_offset_limit_mhz ?? 2000;
  const powerMin = clamp(control?.power_limit_min_percent ?? 50, 1, 100);
  const powerMax = clamp(control?.power_limit_max_percent ?? 100, powerMin, 150);
  const points = curveDraft ?? curve?.points ?? [];
  const curveDirty = curveDraft != null;

  const setFanMode = useCallback(
    (mode: FanMode) => {
      if (!selectedGpuId) return;
      if (mode === 'automatic') {
        runAction(
          'fan-mode',
          async () => {
            await controlService.restoreFanControlAutomatic(selectedGpuId);
            if (curve?.enabled) {
              try {
                await fanCurveService.setEnabled(selectedGpuId, false);
              } catch {
                /* engine may already be off; restore above succeeded */
              }
            }
          },
          'Fan control returned to the driver',
        ).then(() => reloadCurve());
      } else if (mode === 'curve') {
        if (!curve?.enabled) {
          runAction(
            'fan-mode',
            async () => {
              await fanCurveService.set(selectedGpuId, curveDraft ?? curve?.points ?? FALLBACK_CURVE);
              await fanCurveService.setEnabled(selectedGpuId, true);
            },
            'Custom fan curve engaged',
          ).then(() => reloadCurve());
        }
      } else if (mode === 'fixed') {
        // Entering fixed mode applies the slider value once.
        runAction(
          'fan-mode',
          async () => {
            if (curve?.enabled) {
              await fanCurveService.setEnabled(selectedGpuId, false);
            }
            await controlService.setFanSpeed(selectedGpuId, fixedFan);
          },
          `Fan fixed at ${fixedFan}%`,
        ).then(() => reloadCurve());
      }
    },
    [selectedGpuId, curve, curveDraft, fixedFan, runAction, reloadCurve],
  );

  const applyCurve = () => {
    if (!selectedGpuId || !curveDirty) return;
    runAction(
      'curve-apply',
      async () => {
        await fanCurveService.set(selectedGpuId, curveDraft!);
        if (curve?.enabled) {
          await fanCurveService.setEnabled(selectedGpuId, true);
        }
      },
      curve?.enabled ? 'Fan curve applied and running' : 'Fan curve saved',
    ).then(() => reloadCurve());
  };

  const resetCurve = () => {
    if (!curve) return;
    setCurveDraft(null);
    reloadCurve();
  };

  const resetAll = () => {
    if (!selectedGpuId) return;
    if (!window.confirm('Reset all OverDrive tuning to driver defaults? Clock offsets, power limit, and fan control return to their factory state.')) {
      return;
    }
    runAction('reset-all', () => controlService.resetControls(selectedGpuId), 'Controls restored to driver defaults')
      .then(() => {
        setActiveSlot(null);
        reloadCurve();
      });
  };

  /** Capture the currently dialed-in control state as one portable snapshot
   *  (the MSI Afterburner profile model). Unsupported controls serialize as
   *  null and the backend skips them on apply. */
  const buildSnapshot = useCallback((): ControlSnapshot => {
    let fan_mode: SnapshotFanMode = 'automatic';
    let fixedFanPercent: number | null = null;
    let fanCurve: FanCurvePoint[] | null = null;
    if (curve?.enabled) {
      fan_mode = 'curve';
      fanCurve = curveDraft ?? curve.points;
    } else if (control?.fan_control_automatic === false) {
      fan_mode = 'fixed';
      fixedFanPercent = fixedFan;
    }
    return {
      core_offset_mhz: Math.round(coreOffset),
      memory_offset_mhz: Math.round(memoryOffset),
      voltage_offset_mv: 0,
      power_limit_percent: powerPercent == null ? null : Math.round(powerPercent),
      fan_mode,
      fixed_fan_percent: fixedFanPercent == null ? null : Math.round(fixedFanPercent),
      fan_curve: fanCurve,
    };
  }, [curve, curveDraft, control?.fan_control_automatic, fixedFan, coreOffset, memoryOffset, powerPercent]);

  const handleSlotSave = (slot: number) => {
    if (!selectedGpuId) return;
    runAction(
      'slot-save',
      () => profileService.saveSlot(selectedGpuId, slot, buildSnapshot()),
      `Settings captured to slot P${slot}`,
    ).then(() => {
      setActiveSlot(slot);
      reloadSlots();
    });
  };

  const handleSlotLoad = (slot: number) => {
    if (!selectedGpuId) return;
    runAction(
      'slot-load',
      () => profileService.loadSlot(selectedGpuId, slot),
      `Slot P${slot} applied`,
    ).then(() => {
      setActiveSlot(slot);
      reloadCurve();
    });
  };

  const handleSlotClear = (slot: number) => {
    runAction('slot-clear', () => profileService.clearSlot(slot), `Slot P${slot} erased`).then(() => {
      if (activeSlot === slot) setActiveSlot(null);
      reloadSlots();
    });
  };

  const name = selectedGpu?.name ?? selectedGpuId;

  return (
    <div className="ac-page ac-page--wide">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <span className="ac-page-header__title">OverDrive Tuning</span>
          <span className="ac-page-header__desc">{name} — GPU fan, clock, and power control</span>
        </div>
        <div className="ac-page-header__right">
          <span className="ac-badge ac-badge--blue">{name}</span>
          <button
            className="ac-btn ac-btn--danger ac-btn--sm"
            onClick={resetAll}
            disabled={busyAction !== null || selectedGpuId == null}
            title="Restore all controls to driver defaults"
          >
            {busyAction === 'reset-all' ? 'Resetting…' : 'Reset All'}
          </button>
        </div>
      </div>

      {!selectedGpuId && (
        <div className="ac-banner ac-banner--warning">No GPU selected. Launch with an NVIDIA driver installed to list GPUs.</div>
      )}

      <ActionResultLine state={actionResult['reset-all']} />

      {selectedGpuId && (
        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">Live telemetry</span>
            <span className="od-live-pill"><i className="od-live-pill__dot" aria-hidden />LIVE · 10 Hz</span>
          </div>
          <div className="ac-page-card__body">
            <div className="ac-grid-4 od-metrics-grid">
              <TelemetryTile label="GPU Temp" value={data?.temperature_celsius == null ? 'Unavailable' : `${data.temperature_celsius.toFixed(1)}°C`} color="#fbbf24" />
              <TelemetryTile label="Core Clock" value={data?.core_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.core_clock_mhz)} color="#60a5fa" />
              <TelemetryTile label="Memory Clock" value={data?.memory_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.memory_clock_mhz)} color="#22d3ee" />
              <TelemetryTile label="Fan" value={data?.fan_speed_percent == null ? 'Unavailable' : `${Math.round(data.fan_speed_percent)}%`} color="#40dc9e" />
              <TelemetryTile label="Power" value={data?.power_watts == null ? 'Unavailable' : `${data.power_watts.toFixed(1)} W`} color="#f97316" />
              <TelemetryTile label="GPU Load" value={data?.core_utilization_percent == null ? 'Unavailable' : `${Math.round(data.core_utilization_percent)}%`} color="#c084fc" />
              <TelemetryTile label="VRAM Used" value={data?.memory_used_mb == null ? 'Unavailable' : `${data.memory_used_mb} MB`} color="#94a3b8" />
              <TelemetryTile label="Voltage" value={data?.core_voltage_mv == null ? 'Unavailable' : `${data.core_voltage_mv.toFixed(0)} mV`} color="#a78bfa" />
            </div>
          </div>
        </div>
      )}

      {selectedGpuId && (
        <QuickSlotsCard
          slots={slots}
          activeSlot={activeSlot}
          result={actionResult['slot-load'] ?? actionResult['slot-save'] ?? actionResult['slot-clear']}
          busy={busyAction !== null}
          onLoad={handleSlotLoad}
          onSave={handleSlotSave}
          onClear={handleSlotClear}
        />
      )}

      {selectedGpuId && control && (
        <div className="od-tuning-layout">
          {/* -------- Fan control -------- */}
          <section className="ac-page-card od-tuning-col" aria-label="Fan control">
            <div className="ac-page-card__header">
              <span className="ac-page-card__title">Fan Control</span>
              <span className="ac-badge ac-badge--blue">{fanMode === 'curve' ? 'Custom curve' : fanMode === 'fixed' ? 'Fixed speed' : 'Automatic'}</span>
            </div>
            <div className="ac-page-card__body">
              {!caps.fanControl && (
                <div className="ac-banner ac-banner--warning">
                  This driver does not expose fan control through the supported interfaces. Telemetry still reads live.
                </div>
              )}
              <div className="od-fan-modes" role="tablist" aria-label="Fan mode">
                <ModeCard
                  id="automatic"
                  label="Automatic"
                  hint="Driver's own thermal policy"
                  active={fanMode === 'automatic'}
                  disabled={busyAction !== null || !caps.fanControl}
                  onClick={() => setFanMode('automatic')}
                />
                <ModeCard
                  id="curve"
                  label="Custom curve"
                  hint="Duty % vs temperature"
                  active={fanMode === 'curve'}
                  disabled={busyAction !== null || !caps.fanControl}
                  onClick={() => setFanMode('curve')}
                />
                <ModeCard
                  id="fixed"
                  label="Fixed"
                  hint="Constant duty target"
                  active={fanMode === 'fixed'}
                  disabled={busyAction !== null || !caps.fanControl}
                  onClick={() => setFanMode('fixed')}
                />
              </div>

              {fanMode === 'fixed' && (
                <div className="od-control-block">
                  <label className="ac-label" htmlFor="fixed-fan-slider">Fixed fan target — {fixedFan}%</label>
                  <input
                    id="fixed-fan-slider"
                    className="ac-slider"
                    type="range"
                    min={0}
                    max={100}
                    step={1}
                    value={fixedFan}
                    disabled={!caps.fanControl || busyAction !== null}
                    onChange={(e) => setFixedFan(Number(e.target.value))}
                    onPointerUp={() =>
                      selectedGpuId &&
                      runAction('fixed-fan', () => controlService.setFanSpeed(selectedGpuId, fixedFan), `Fan set to ${fixedFan}%`)
                    }
                  />
                </div>
              )}

              {fanMode !== 'fixed' && (
                <>
                  <div className="od-control-block">
                    <FanCurveEditor
                      points={points}
                      onChange={(nextPoints) => setCurveDraft(nextPoints)}
                      currentTemperature={data?.temperature_celsius ?? null}
                      disabled={!caps.fanControl}
                    />
                  </div>
                  <div className="ac-row od-control-actions">
                    <span className="ac-label">
                      {curve?.enabled
                        ? `Curve is engaged and re-applies every second${curveDirty ? ' (unsaved edits)' : ''}`
                        : 'Curve is saved but not engaged'}
                    </span>
                    <div className="od-action-row">
                      <button className="ac-btn ac-btn--secondary ac-btn--sm" onClick={resetCurve} disabled={busyAction !== null || !curveDirty}>
                        Discard
                      </button>
                      <button
                        className="ac-btn ac-btn--secondary ac-btn--sm"
                        onClick={() => curve && setCurveDraft(clonePoints(curve.points))}
                        disabled={busyAction !== null}
                        title="Load the saved curve into the editor again"
                      >
                        Reload
                      </button>
                      {curve?.enabled ? (
                        <button
                          className="ac-btn ac-btn--danger ac-btn--sm"
                          disabled={busyAction !== null}
                          onClick={() => {
                            if (!selectedGpuId) return;
                            runAction(
                              'curve-toggle',
                              async () => {
                                const view = await fanCurveService.setEnabled(selectedGpuId, false);
                                setCurve(view);
                                setCurveDraft(null);
                              },
                              'Custom curve disabled; driver fan policy restored',
                            );
                          }}
                        >
                          {busyAction === 'curve-toggle' ? 'Disabling…' : 'Disable Curve'}
                        </button>
                      ) : (
                        <button
                          className="ac-btn ac-btn--primary ac-btn--sm"
                          disabled={busyAction !== null || !!curveDirty}
                          onClick={() => setFanMode('curve')}
                          title={curveDirty ? 'Apply the edited curve first' : 'Engage the custom curve'}
                        >
                          Enable Curve
                        </button>
                      )}
                      {(!curve?.enabled || curveDirty) && (
                        <button className="ac-btn ac-btn--success ac-btn--sm" onClick={applyCurve} disabled={busyAction !== null || !curveDirty}>
                          {busyAction === 'curve-apply' ? 'Applying…' : curve?.enabled ? 'Apply Edits' : 'Engage Curve'}
                        </button>
                      )}
                    </div>
                  </div>
                </>
              )}
              <ActionResultLine state={actionResult['fan-mode'] ?? actionResult['fixed-fan'] ?? actionResult['curve-toggle'] ?? actionResult['curve-apply']} />
            </div>
          </section>

          {/* -------- Clock / power -------- */}
          <section className="ac-page-card od-tuning-col" aria-label="Clock and power tuning">
            <div className="ac-page-card__header">
              <span className="ac-page-card__title">Core &amp; Memory</span>
              <span className="ac-badge ac-badge--blue">{caps.coreOffset || caps.memoryOffset ? 'Offset control' : 'Read-only'}</span>
            </div>
            <div className="ac-page-card__body">
              <OffsetSlider
                id="core-offset"
                label="Core clock offset"
                value={coreOffset}
                limit={coreLimit}
                disabled={!caps.coreOffset || busyAction !== null}
                onCommit={(value) => selectedGpuId && runAction('core', () => controlService.setCoreClockOffset(selectedGpuId, value), `Core offset ${value > 0 ? '+' : ''}${value} MHz applied`)}
                onChange={setCoreOffset}
              />
              <OffsetSlider
                id="memory-offset"
                label="Memory clock offset"
                value={memoryOffset}
                limit={memoryLimit}
                disabled={!caps.memoryOffset || busyAction !== null}
                onCommit={(value) => selectedGpuId && runAction('memory', () => controlService.setMemoryClockOffset(selectedGpuId, value), `Memory offset ${value > 0 ? '+' : ''}${value} MHz applied`)}
                onChange={setMemoryOffset}
              />
              <div className="od-control-block">
                {!caps.coreOffset && !caps.memoryOffset && (
                  <div className="ac-banner ac-banner--info">
                    This GPU/driver build does not expose clock offset control. OverDrive keeps the sliders read-only and shows live clocks above.
                  </div>
                )}
              </div>

              <div style={{ height: 8 }} />

              <div className="ac-page-card">
                <div className="ac-page-card__header">
                  <span className="ac-page-card__title">Power Limit</span>
                  {control.power_limit_percent != null && (
                    <span className="ac-badge ac-badge--blue">{control.power_limit_percent.toFixed(0)}% of default</span>
                  )}
                </div>
                <div className="ac-page-card__body">
                  {caps.powerLimit && control.power_limit_percent != null ? (
                    <>
                      <label className="ac-label" htmlFor="power-slider">Power limit — {Math.round(powerPercent ?? control.power_limit_percent)}%</label>
                      <input
                        id="power-slider"
                        className="ac-slider"
                        type="range"
                        min={Math.round(powerMin)}
                        max={Math.round(powerMax)}
                        step={1}
                        value={Math.round(clamp(powerPercent ?? control.power_limit_percent, powerMin, powerMax))}
                        disabled={busyAction !== null}
                        onChange={(e) => setPowerPercent(Number(e.target.value))}
                        onPointerUp={() =>
                          selectedGpuId &&
                          powerPercent != null &&
                          runAction(
                            'power',
                            () => controlService.setPowerLimit(selectedGpuId, powerPercent),
                            `Power limit set to ${Math.round(powerPercent)}%`,
                          )
                        }
                      />
                      <span className="od-slider-bounds">{Math.round(powerMin)}% — {Math.round(powerMax)}%</span>
                    </>
                  ) : (
                    <div className="ac-banner ac-banner--warning">
                      Power limit data is {control.power_limit_percent == null ? 'unavailable' : 'read-only'} for this driver.
                    </div>
                  )}
                  <ActionResultLine state={actionResult['power']} />
                </div>
              </div>

              <div style={{ height: 8 }} />

              <div className="od-control-block">
                <label className="ac-label ac-label--muted">Voltage offset</label>
                <input
                  className="ac-slider"
                  type="range"
                  min={0}
                  max={200}
                  value={0}
                  disabled
                  aria-label="Voltage offset (not available)"
                  onChange={() => {}}
                />
                {!caps.voltageOffset && (
                  <span className="od-slider-bounds">Not exposed by this build&apos;s NVIDIA interface</span>
                )}
                <ActionResultLine state={actionResult['core'] ?? actionResult['memory']} />
              </div>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}

function TelemetryTile({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="od-tile">
      <span className="od-tile__label">{label}</span>
      <span className="od-tile__value" style={{ color }}>{value}</span>
    </div>
  );
}

function ModeCard({ id, label, hint, active, disabled, onClick }: {
  id: string;
  label: string;
  hint: string;
  active: boolean;
  disabled: boolean;
  onClick: () => void;
}) {
  return (
    <button
      id={`fan-mode-${id}`}
      role="tab"
      aria-selected={active}
      className={`od-mode-card ${active ? 'od-mode-card--active' : ''}`}
      disabled={disabled}
      onClick={onClick}
    >
      <span className="od-mode-card__label">{label}</span>
      <span className="od-mode-card__hint">{hint}</span>
    </button>
  );
}

function OffsetSlider({
  id,
  label,
  value,
  limit,
  disabled,
  onChange,
  onCommit,
}: {
  id: string;
  label: string;
  value: number;
  limit: number;
  disabled: boolean;
  onChange: (value: number) => void;
  onCommit: (value: number) => void;
}) {
  return (
    <div className="od-control-block">
      <label className="ac-label" htmlFor={id}>{label} — {value > 0 ? '+' : ''}{value} MHz</label>
      <input
        id={id}
        className="ac-slider"
        type="range"
        min={-limit}
        max={limit}
        step={5}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        onPointerUp={() => onCommit(value)}
        onKeyUp={() => onCommit(value)}
      />
      <span className="od-slider-bounds">-{limit} — +{limit} MHz (0 dB none)</span>
    </div>
  );
}

function ActionResultLine({ state }: { state?: ActionResult | undefined }) {
  if (!state) return null;
  return (
    <div className={state.ok ? 'ac-banner ac-banner--success' : 'ac-banner ac-banner--error'} role="status">
      {state.message}
    </div>
  );
}

/** P1–P5 quick-slot strip, the MSI Afterburner profile model: clicking an
 *  occupied slot applies its whole control snapshot (fan mode included);
 *  Save stores everything currently dialed in; ✕ erases the slot. The
 *  "applied" marker is session-local, like Afterburner's active profile. */
function QuickSlotsCard({ slots, activeSlot, result, busy, onLoad, onSave, onClear }: {
  slots: (QuickSlotSummary | null)[];
  activeSlot: number | null;
  result: ActionResult | undefined;
  busy: boolean;
  onLoad: (slot: number) => void;
  onSave: (slot: number) => void;
  onClear: (slot: number) => void;
}) {
  const list = Array.from({ length: 5 }, (_, i) => slots[i] ?? null);
  return (
    <div className="ac-page-card" aria-label="Quick slots">
      <div className="ac-page-card__header">
        <span className="ac-page-card__title">Quick Slots</span>
        <span className="ac-badge ac-badge--blue">P1–P5 · fan, clocks &amp; power in one unit</span>
      </div>
      <div className="ac-page-card__body">
        <span className="ac-label ac-label--muted">
          Click a filled slot to apply it; Save stores everything currently dialed in.
        </span>
        <div className="od-profile-strip">
          {list.map((entry, index) => {
            const slot = index + 1;
            return (
              <div
                key={slot}
                className={[
                  'od-slot-chip',
                  entry != null ? 'od-slot-chip--filled' : '',
                  activeSlot === slot ? 'od-slot-chip--active' : '',
                ].filter(Boolean).join(' ')}
              >
                <button
                  className="od-slot-chip__main"
                  disabled={busy}
                  onClick={() => (entry != null ? onLoad(slot) : onSave(slot))}
                  title={entry != null ? `Apply slot P${slot} (${entry.name})` : `Save the current settings to slot P${slot}`}
                >
                  <span className="od-slot-chip__label">P{slot}</span>
                  <span className="od-slot-chip__mode">{entry != null ? modeShort(entry.snapshot.fan_mode) : 'EMPTY'}</span>
                  <span className="od-slot-chip__desc">{entry != null ? describeSlot(entry.snapshot) : 'Save current settings here'}</span>
                  {activeSlot === slot && <span className="od-slot-chip__tag">APPLIED</span>}
                </button>
                <div className="od-slot-chip__ops">
                  <button className="od-slot-chip__op" disabled={busy} onClick={() => onSave(slot)} title={`Overwrite slot P${slot} with the current settings`}>
                    Save
                  </button>
                  {entry != null && (
                    <button className="od-slot-chip__op" disabled={busy} onClick={() => onLoad(slot)} title={`Apply slot P${slot}`}>
                      Load
                    </button>
                  )}
                  {entry != null && (
                    <button
                      className="od-slot-chip__op od-slot-chip__op--danger"
                      disabled={busy}
                      onClick={() => onClear(slot)}
                      title={`Erase slot P${slot}`}
                      aria-label={`Erase slot P${slot}`}
                    >
                      ✕
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <ActionResultLine state={result} />
      </div>
    </div>
  );
}

/** Compact label for a snapshot's fan mode, shown on the slot chip. */
function modeShort(mode: SnapshotFanMode | undefined): string {
  switch (mode) {
    case 'fixed':
      return 'FIXED';
    case 'curve':
      return 'CURVE';
    default:
      return 'AUTO';
  }
}

/** One-line human summary of a stored snapshot for its slot chip. */
function describeSlot(snapshot: ControlSnapshot): string {
  const offsets =
    snapshot.core_offset_mhz === 0 && snapshot.memory_offset_mhz === 0
      ? '±0 MHz'
      : `${signed(snapshot.core_offset_mhz)}/${signed(snapshot.memory_offset_mhz)} MHz`;
  const parts = [offsets];
  if (snapshot.power_limit_percent != null) {
    parts.push(`${Math.round(snapshot.power_limit_percent)}% power`);
  }
  if (snapshot.fan_mode === 'curve') {
    parts.push('fan curve');
  } else if (snapshot.fan_mode === 'fixed') {
    parts.push(snapshot.fixed_fan_percent != null ? `fan ${Math.round(snapshot.fixed_fan_percent)}%` : 'fan fixed');
  } else {
    parts.push('fan auto');
  }
  return parts.join(' · ');
}

function signed(value: number): string {
  return value > 0 ? `+${value}` : `${value}`;
}

function clonePoints(points: FanCurvePoint[]): FanCurvePoint[] {
  return points.map((p) => ({ ...p }));
}

/** NVML/backend errors arrive as long strings; show the informative tail. */
function humanizeTauriError(error: unknown): string {
  let text = String(error).trim();
  // Tauri prefixes invocation failures like "Error setting fan speed: <msg>".
  const knownMarkers = ['failed to set control', 'failed to read data', 'provider not available', 'gpu not found'];
  for (const marker of knownMarkers) {
    const index = text.toLowerCase().indexOf(marker);
    if (index >= 0) {
      text = text.slice(index);
      break;
    }
  }
  return text.length > 220 ? `${text.slice(0, 220)}…` : text;
}
