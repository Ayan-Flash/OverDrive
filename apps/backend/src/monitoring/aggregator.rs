use crate::hardware::GpuSample;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AggregatedMetrics {
    pub gpu_id: String,
    pub temperature: MetricStats,
    pub core_clock: MetricStats,
    pub memory_clock: MetricStats,
    pub fan_speed: MetricStats,
    pub power: MetricStats,
    pub core_util: MetricStats,
    pub mem_util: MetricStats,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricStats {
    pub current: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub avg: Option<f64>,
}

pub struct Aggregator;

impl Aggregator {
    pub fn aggregate(gpu_id: &str, samples: &[&GpuSample]) -> AggregatedMetrics {
        let temps: Vec<f64> = samples
            .iter()
            .filter_map(|s| s.temperature_celsius)
            .collect();
        let clocks: Vec<f64> = samples.iter().filter_map(|s| s.core_clock_mhz).collect();
        let mem_clocks: Vec<f64> = samples.iter().filter_map(|s| s.memory_clock_mhz).collect();
        let fans: Vec<f64> = samples.iter().filter_map(|s| s.fan_speed_percent).collect();
        let powers: Vec<f64> = samples.iter().filter_map(|s| s.power_watts).collect();
        let core_utils: Vec<f64> = samples
            .iter()
            .filter_map(|s| s.core_utilization_percent)
            .collect();
        let mem_utils: Vec<f64> = samples
            .iter()
            .filter_map(|s| s.memory_utilization_percent)
            .collect();

        AggregatedMetrics {
            gpu_id: gpu_id.to_string(),
            temperature: Self::stats(&temps, samples.last().and_then(|s| s.temperature_celsius)),
            core_clock: Self::stats(&clocks, samples.last().and_then(|s| s.core_clock_mhz)),
            memory_clock: Self::stats(&mem_clocks, samples.last().and_then(|s| s.memory_clock_mhz)),
            fan_speed: Self::stats(&fans, samples.last().and_then(|s| s.fan_speed_percent)),
            power: Self::stats(&powers, samples.last().and_then(|s| s.power_watts)),
            core_util: Self::stats(
                &core_utils,
                samples.last().and_then(|s| s.core_utilization_percent),
            ),
            mem_util: Self::stats(
                &mem_utils,
                samples.last().and_then(|s| s.memory_utilization_percent),
            ),
        }
    }

    fn stats(values: &[f64], current: Option<f64>) -> MetricStats {
        let min =
            (!values.is_empty()).then(|| values.iter().copied().fold(f64::INFINITY, f64::min));
        let max =
            (!values.is_empty()).then(|| values.iter().copied().fold(f64::NEG_INFINITY, f64::max));
        let avg = (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64);

        MetricStats {
            current,
            min,
            max,
            avg,
        }
    }

    pub fn smooth(values: &[f64], window: usize) -> Vec<f64> {
        if values.len() <= window {
            return values.to_vec();
        }

        let mut smoothed = Vec::with_capacity(values.len());
        for i in 0..values.len() {
            let start = i.saturating_sub(window / 2);
            let end = (i + window / 2).min(values.len());
            let slice = &values[start..end];
            let avg = slice.iter().sum::<f64>() / slice.len() as f64;
            smoothed.push(avg);
        }

        smoothed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(temperature_celsius: Option<f64>) -> GpuSample {
        GpuSample {
            gpu_id: "gpu".into(),
            timestamp: 0,
            temperature_celsius,
            core_clock_mhz: None,
            memory_clock_mhz: None,
            memory_used_mb: None,
            memory_total_mb: None,
            fan_speed_percent: None,
            fan_rpm: None,
            power_watts: None,
            max_power_limit_watts: None,
            core_voltage_mv: None,
            core_utilization_percent: None,
            memory_utilization_percent: None,
        }
    }
    #[test]
    fn latest_missing_sample_does_not_reuse_an_older_value() {
        let old = sample(Some(42.0));
        let latest = sample(None);
        let result = Aggregator::aggregate("gpu", &[&old, &latest]);
        assert_eq!(result.temperature.current, None);
        assert_eq!(result.temperature.avg, Some(42.0));
    }
}
