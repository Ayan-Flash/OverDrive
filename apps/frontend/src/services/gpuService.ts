import { invoke } from '@tauri-apps/api/core';

export interface GPUInfo {
  id: string;
  name: string;
  vendor: string;
  index: number;
  memory_total_mb?: number | null;
}

export interface GPUData {
  gpu_id: string;
  timestamp: number;
  temperature_celsius: number | null;
  core_clock_mhz: number | null;
  memory_clock_mhz: number | null;
  memory_used_mb: number | null;
  memory_total_mb: number | null;
  fan_speed_percent: number | null;
  fan_rpm: number | null;
  power_watts: number | null;
  max_power_limit_watts?: number | null;
  core_voltage_mv: number | null;
  core_utilization_percent: number | null;
  memory_utilization_percent: number | null;
}

export interface GpuDataResponse {
  sample: GPUData;
  alerts: unknown[];
}

export interface GpuHistoryResponse {
  samples: ExportedSample[];
  aggregated: AggregatedMetrics | null;
}

export interface ExportedSample {
  t: number;
  temp: number | null;
  core: number | null;
  mem: number | null;
  fan: number | null;
  power: number | null;
  core_util: number | null;
  mem_util: number | null;
  mem_used_mb: number | null;
  mem_total_mb: number | null;
}

export interface AggregatedMetrics {
  gpu_id: string;
  temperature: MetricStats;
  core_clock: MetricStats;
  memory_clock: MetricStats;
  fan_speed: MetricStats;
  power: MetricStats;
  core_util: MetricStats;
  mem_util: MetricStats;
}

export interface MetricStats {
  current: number | null;
  min: number | null;
  max: number | null;
  avg: number | null;
}

export interface GpuControlCapabilities {
  fan_control: boolean;
  core_clock_offset: boolean;
  memory_clock_offset: boolean;
  power_limit: boolean;
  voltage_offset: boolean;
}

export interface GpuControlState {
  gpu_id: string;
  target_fan_speed: number | null;
  fan_control_automatic: boolean | null;
  fan_count: number | null;
  core_clock_offset_mhz: number;
  memory_clock_offset_mhz: number;
  power_limit_percent: number | null;
  power_limit_min_percent?: number | null;
  power_limit_max_percent?: number | null;
  voltage_offset_mv: number;
  core_clock_offset_limit_mhz?: number;
  memory_clock_offset_limit_mhz?: number;
  capabilities?: GpuControlCapabilities;
}

export interface FanCurvePoint {
  temperature_celsius: number;
  fan_percent: number;
}

export interface FanCurveView {
  gpu_id: string;
  enabled: boolean;
  points: FanCurvePoint[];
}

export interface AlertRule {
  id: string;
  gpu_id: string;
  metric: string;
  condition: string;
  threshold: number;
  enabled: boolean;
}

export interface AlertEvent {
  id: string;
  rule_id: string;
  gpu_id: string;
  metric: string;
  severity: 'Info' | 'Warning' | 'Critical';
  message: string;
  value: number;
  threshold: number;
  timestamp: number;
  acknowledged: boolean;
}

export const gpuService = {
  list: () => invoke<GPUInfo[]>('list_gpus'),

  getData: (gpuId: string) => invoke<GpuDataResponse>('get_gpu_data', { gpuId }),

  /** Live-sample-only fast path (no recording, no alert evaluation). */
  getSample: (gpuId: string) => invoke<GPUData>('get_gpu_sample', { gpuId }),

  getHistory: (gpuId: string, count: number) =>
    invoke<GpuHistoryResponse>('get_gpu_history', { gpuId, count }),

  getControlState: (gpuId: string) => invoke<GpuControlState>('get_gpu_control_state', { gpuId }),
};

export const controlService = {
  setFanSpeed: (gpuId: string, percent: number) =>
    invoke<void>('set_fan_speed', { gpuId, percent }),

  setCoreClockOffset: (gpuId: string, offsetMhz: number) =>
    invoke<void>('set_core_clock_offset', { gpuId, offsetMhz }),

  setMemoryClockOffset: (gpuId: string, offsetMhz: number) =>
    invoke<void>('set_memory_clock_offset', { gpuId, offsetMhz }),

  setPowerLimit: (gpuId: string, percent: number) =>
    invoke<void>('set_power_limit', { gpuId, percent }),

  setVoltageOffset: (gpuId: string, offsetMv: number) =>
    invoke<void>('set_voltage_offset', { gpuId, offsetMv }),

  restoreFanControlAutomatic: (gpuId: string) =>
    invoke<void>('set_fan_control_automatic', { gpuId }),

  resetControls: (gpuId: string) => invoke<void>('reset_gpu_controls', { gpuId }),
};

export const fanCurveService = {
  get: (gpuId: string) => invoke<FanCurveView>('get_fan_curve', { gpuId }),

  set: (gpuId: string, points: FanCurvePoint[]) =>
    invoke<void>('set_fan_curve', { gpuId, points }),

  setEnabled: (gpuId: string, enabled: boolean) =>
    invoke<FanCurveView>('set_fan_curve_enabled', { gpuId, enabled }),
};

export const monitoringService = {
  start: () => invoke<void>('start_monitoring'),
  stop: () => invoke<void>('stop_monitoring'),
  isRunning: () => invoke<boolean>('is_monitoring_running'),
  exportCsv: (gpuId: string, count: number) => invoke<string>('export_csv', { gpuId, count }),
};

export type SnapshotFanMode = 'automatic' | 'fixed' | 'curve';

/** Full portable control state, captured or applied as one unit (the
 *  MSI Afterburner profile model). Field names match the backend's serde
 *  field names (snake_case over the wire). */
export interface ControlSnapshot {
  core_offset_mhz: number;
  memory_offset_mhz: number;
  voltage_offset_mv: number;
  power_limit_percent?: number | null;
  fan_mode?: SnapshotFanMode;
  fixed_fan_percent?: number | null;
  fan_curve?: FanCurvePoint[] | null;
}

export interface QuickSlotSummary {
  slot: number;
  name: string;
  gpu_id: string;
  saved_at: string;
  snapshot: ControlSnapshot;
}

export const profileService = {
  save: (name: string, gpuId: string, coreOffset: number, memOffset: number, voltageOffset: number, fanSpeed: number, powerLimit: number) =>
    invoke<string>('save_profile', { name, gpuId, coreOffset, memOffset, voltageOffset, fanSpeed, powerLimit }),

  load: () => invoke<unknown[]>('load_profiles'),

  delete: (filename: string) => invoke<void>('delete_profile', { filename }),

  apply: (gpuId: string, coreOffset: number, memOffset: number, voltageOffset: number, fanSpeed: number, powerLimit: number) =>
    invoke<void>('apply_profile', { gpuId, coreOffset, memOffset, voltageOffset, fanSpeed, powerLimit }),

  /** Apply a full control snapshot (offsets + power + fan mode) to a GPU. */
  applySnapshot: (gpuId: string, snapshot: ControlSnapshot) =>
    invoke<void>('apply_control_snapshot', { gpuId, snapshot }),

  slots: () => invoke<(QuickSlotSummary | null)[]>('get_quick_slots'),

  saveSlot: (gpuId: string, slot: number, snapshot: ControlSnapshot, name?: string) =>
    invoke<void>('quick_save_slot', { gpuId, slot, snapshot, name }),

  loadSlot: (gpuId: string, slot: number) => invoke<void>('quick_load_slot', { gpuId, slot }),

  clearSlot: (slot: number) => invoke<void>('quick_delete_slot', { slot }),
};

export const alertService = {
  getRules: () => invoke<AlertRule[]>('get_alert_rules'),
  getRulesForGpu: (gpuId: string) => invoke<AlertRule[]>('get_alert_rules_for_gpu', { gpuId }),
  addRule: (rule: AlertRule) => invoke<void>('add_alert_rule', { rule }),
  removeRule: (ruleId: string) => invoke<boolean>('remove_alert_rule', { ruleId }),
  updateRule: (rule: AlertRule) => invoke<boolean>('update_alert_rule', { rule }),
  getHistory: (limit: number) => invoke<AlertEvent[]>('get_alert_history', { limit }),
  acknowledge: (alertId: string) => invoke<boolean>('acknowledge_alert', { alertId }),
  clearHistory: () => invoke<void>('clear_alert_history'),
};

export const remoteService = {
  start: (port: number, apiKey?: string) =>
    invoke<string>('start_remote_server', { port, apiKey }),
  stop: () => invoke<void>('stop_remote_server'),
  getStatus: () => invoke<{ running: boolean; port?: number; url?: string }>('get_remote_server_status'),
  generateKey: () => invoke<string>('generate_api_key'),
};

export interface OverlayMetric {
  metric: string;
  label: string;
  color: string;
  enabled: boolean;
}

export interface OverlayConfig {
  enabled: boolean;
  metrics: OverlayMetric[];
  position: 'TopLeft' | 'TopRight' | 'BottomLeft' | 'BottomRight';
  opacity: number;
  auto_hide_no_game: boolean;
  scale: number;
}

export const overlayService = {
  start: () => invoke<void>('start_overlay'),
  stop: () => invoke<void>('stop_overlay'),
  isRunning: () => invoke<boolean>('is_overlay_running'),
  getConfig: () => invoke<OverlayConfig>('get_overlay_config'),
  updateConfig: (config: OverlayConfig) => invoke<void>('update_overlay_config', { config }),
  getDetectedGames: () => invoke<string[]>('get_detected_games'),
  isGameRunning: () => invoke<boolean>('is_game_running'),
  getData: () => invoke<{ running: boolean; should_show: boolean; data: [string, string, number][] }>('get_overlay_data'),
};

export const appService = {
  getInfo: () => invoke<{ name: string; version: string; platform: string }>('get_app_info'),
};
