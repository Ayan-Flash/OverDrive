import { useGpuStore, useUiStore } from '../stores';
import { formatClockSpeed } from '@common/utils';

export function ControlPage() {
  const { currentData, gpus } = useGpuStore();
  const { selectedGpuId } = useUiStore();
  const data = selectedGpuId ? currentData.get(selectedGpuId) : null;
  const selectedGpu = gpus.find((gpu) => gpu.id === selectedGpuId);

  return (
    <div className="ac-page ac-page--wide">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <span className="ac-page-header__title">GPU Tuning</span>
          <span className="ac-page-header__desc">Hardware telemetry and supported controls</span>
        </div>
        {selectedGpuId && (
          <div className="ac-page-header__right">
            <span className="ac-badge ac-badge--blue">{selectedGpu?.name ?? selectedGpuId}</span>
          </div>
        )}
      </div>

      {selectedGpuId && data && (
        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">
              <svg className="ac-page-card__title-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
              </svg>
              Current Status
            </span>
          </div>
          <div className="ac-page-card__body">
            <div className="ac-grid-4">
              {[
                { label: 'Core Clock', value: data.core_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.core_clock_mhz), color: '#60a5fa' },
                { label: 'Memory Clock', value: data.memory_clock_mhz == null ? 'Unavailable' : formatClockSpeed(data.memory_clock_mhz), color: '#22d3ee' },
                { label: 'Temperature', value: data.temperature_celsius == null ? 'Unavailable' : `${data.temperature_celsius.toFixed(1)}°C`, color: data.temperature_celsius != null && data.temperature_celsius > 80 ? '#f55' : '#fbbf24' },
                { label: 'Voltage', value: data.core_voltage_mv == null ? 'Unavailable' : `${data.core_voltage_mv.toFixed(0)} mV`, color: '#c084fc' },
              ].map((s) => (
                <div key={s.label} style={{ textAlign: 'center' }}>
                  <div style={{ fontSize: 10, color: 'var(--ac-text-muted)', textTransform: 'uppercase', letterSpacing: '0.5px', marginBottom: 4, fontWeight: 500 }}>{s.label}</div>
                  <div className="ac-metric" style={{ fontSize: 18, color: s.color }}>{s.value}</div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}
      <section className="ac-page-card" aria-labelledby="tuning-availability-title">
        <div className="ac-page-card__header">
          <span className="ac-page-card__title" id="tuning-availability-title">Tuning controls unavailable</span>
        </div>
        <div className="ac-page-card__body">
          <p>
            {selectedGpu
              ? `Live readings for ${selectedGpu.name} are shown above. This build does not yet have a supported driver API for changing its fan curve, clocks, voltage, or power limit.`
              : 'Select a detected GPU to view its live readings. Fan, clock, voltage, and power controls will appear only after a supported vendor driver API is available.'}
          </p>
          <p>
            NVIDIA monitoring uses NVML when the NVIDIA driver exposes it. AMD and Intel GPU monitoring and all GPU tuning controls still need vendor API integrations; Windows WMI does not provide those readings here.
          </p>
        </div>
      </section>
    </div>
  );
}
