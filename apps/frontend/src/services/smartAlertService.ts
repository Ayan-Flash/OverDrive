import { invoke } from '@tauri-apps/api/core';

export interface SmartAlertConfig {
  enabled: boolean;
  learning_window_minutes: number;
  adaptive_sensitivity: number;
  suppress_duplicates: boolean;
  duplicate_window_secs: number;
  context_aware: boolean;
  gaming_relaxation_factor: number;
  idle_strictness_factor: number;
  auto_reset_baselines: boolean;
  baseline_reset_hours: number;
}

export interface MetricBaseline {
  metric: string;
  mean: number;
  std_dev: number;
  min: number;
  max: number;
  sample_count: number;
  last_updated: number;
  hourly_patterns: number[];
  load_based_means: Record<string, number>;
}

export interface ContextInfo {
  context: 'Idle' | 'Normal' | 'Gaming' | 'Benchmarking' | 'Unknown';
  confidence: number;
  avg_utilization: number | null;
  avg_temperature: number | null;
  avg_power: number | null;
  duration_secs: number;
}

export interface SuppressedAlert {
  rule_id: string;
  metric: string;
  message: string;
  original_timestamp: number;
  suppress_count: number;
  last_suppressed: number;
}

export const smartAlertService = {
  getStatus: () => invoke<SmartAlertConfig>('get_smart_alert_status'),

  getBaselines: () => invoke<MetricBaseline[]>('get_smart_baselines'),

  getContext: () => invoke<ContextInfo>('get_smart_context'),

  getSuppressed: () => invoke<SuppressedAlert[]>('get_smart_suppressed'),

  updateConfig: (config: SmartAlertConfig) =>
    invoke<void>('update_smart_config', { config }),

  resetBaselines: () => invoke<void>('reset_smart_baselines'),
};
