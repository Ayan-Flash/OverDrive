import { useEffect, useState } from 'react';
import { useGpuStore, useUiStore } from '../stores';
import { FanControl } from '../components/feature/FanControl';
import { formatClockSpeed } from '@common/utils';
import { controlService } from '../services';

export function ControlPage() {
  const { currentData, controlStates, fetchControlState } = useGpuStore();
  const { selectedGpuId } = useUiStore();
  const [coreOffset, setCoreOffset] = useState(0);
  const [memOffset, setMemOffset] = useState(0);
  const [powerLimit, setPowerLimit] = useState(100);
  const [voltageOffset, setVoltageOffset] = useState(0);

  const data = selectedGpuId ? currentData.get(selectedGpuId) : null;
  const control = selectedGpuId ? controlStates.get(selectedGpuId) : null;

  useEffect(() => {
    if (selectedGpuId) {
      fetchControlState(selectedGpuId);
    }
  }, [selectedGpuId, fetchControlState]);

  useEffect(() => {
    if (control) {
      setCoreOffset(control.core_clock_offset_mhz);
      setMemOffset(control.memory_clock_offset_mhz);
      setPowerLimit(control.power_limit_percent ?? 100);
      setVoltageOffset(control.voltage_offset_mv);
    }
  }, [control]);

  const handleCoreOffset = async (value: number) => {
    setCoreOffset(value);
    if (selectedGpuId) {
      await controlService.setCoreClockOffset(selectedGpuId, value);
    }
  };

  const handleMemOffset = async (value: number) => {
    setMemOffset(value);
    if (selectedGpuId) {
      await controlService.setMemoryClockOffset(selectedGpuId, value);
    }
  };

  return (
    <div className="ac-page ac-page--wide">
      <div className="ac-page-header">
        <div className="ac-page-header__left">
          <span className="ac-page-header__title">GPU Control</span>
          <span className="ac-page-header__desc">Overclocking, fan control, and power management</span>
        </div>
        {selectedGpuId && (
          <div className="ac-page-header__right">
            <span className="ac-badge ac-badge--blue">{selectedGpuId}</span>
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

      {selectedGpuId && data?.fan_speed_percent != null && (
        <FanControl gpuId={selectedGpuId} currentSpeed={data.fan_speed_percent} />
      )}

      <div className="ac-grid-2">
        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">Core Clock Offset</span>
            <span className={`ac-metric ${coreOffset >= 0 ? '' : ''}`}
              style={{ fontSize: 14, color: coreOffset >= 0 ? '#34d399' : '#f55' }}>
              {coreOffset >= 0 ? '+' : ''}{coreOffset} MHz
            </span>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input type="range" min={-500} max={500} step={5} value={coreOffset}
              onChange={(e) => handleCoreOffset(Number(e.target.value))}
              className="ac-slider" />
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--ac-text-dim)' }}>
              <span>-500</span><span>0</span><span>+500</span>
            </div>
          </div>
        </div>

        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">Memory Clock Offset</span>
            <span className="ac-metric" style={{ fontSize: 14, color: memOffset >= 0 ? '#34d399' : '#f55' }}>
              {memOffset >= 0 ? '+' : ''}{memOffset} MHz
            </span>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input type="range" min={-2000} max={2000} step={10} value={memOffset}
              onChange={(e) => handleMemOffset(Number(e.target.value))}
              className="ac-slider" />
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--ac-text-dim)' }}>
              <span>-2000</span><span>0</span><span>+2000</span>
            </div>
          </div>
        </div>

        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">Power Limit</span>
            <span className="ac-metric" style={{ fontSize: 14, color: '#fbbf24' }}>{powerLimit}%</span>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input type="range" min={50} max={150} step={1} value={powerLimit}
              onChange={(e) => {
                setPowerLimit(Number(e.target.value));
                if (selectedGpuId) controlService.setPowerLimit(selectedGpuId, Number(e.target.value));
              }}
              className="ac-slider" />
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--ac-text-dim)' }}>
              <span>50%</span><span>100%</span><span>150%</span>
            </div>
          </div>
        </div>

        <div className="ac-page-card">
          <div className="ac-page-card__header">
            <span className="ac-page-card__title">Voltage Offset</span>
            <span className="ac-metric" style={{ fontSize: 14, color: voltageOffset >= 0 ? '#34d399' : '#f55' }}>
              {voltageOffset >= 0 ? '+' : ''}{voltageOffset} mV
            </span>
          </div>
          <div className="ac-page-card__body" style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            <input type="range" min={-200} max={200} step={5} value={voltageOffset}
              onChange={(e) => {
                setVoltageOffset(Number(e.target.value));
                if (selectedGpuId) controlService.setVoltageOffset(selectedGpuId, Number(e.target.value));
              }}
              className="ac-slider" />
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 10, color: 'var(--ac-text-dim)' }}>
              <span>-200</span><span>0</span><span>+200</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
