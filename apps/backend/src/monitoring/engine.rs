use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::RwLock;
use tracing::info;

use super::aggregator::{AggregatedMetrics, Aggregator};
use super::buffer::RingBuffer;
use super::collector::Collector;
use super::exporter::{ExportedSample, Exporter};
use crate::hardware::{GpuControlState, GpuIdentity, GpuProvider, GpuProviderError, GpuSample};

pub struct MonitoringEngine {
    collector: Collector,
    buffers: RwLock<HashMap<String, RingBuffer>>,
    running: AtomicBool,
    gpus: RwLock<Vec<GpuIdentity>>,
    sample_hooks: std::sync::Mutex<Vec<Box<dyn Fn(&GpuSample) + Send + Sync>>>,
}

impl MonitoringEngine {
    pub fn new(provider: Box<dyn GpuProvider>) -> Self {
        let collector = Collector::new(provider);
        let gpus = collector.enumerate_gpus();
        let mut buffers = HashMap::new();
        for gpu in &gpus {
            // Sized for ~1 hour of samples at the 250 ms collection cadence.
            buffers.insert(gpu.id.clone(), RingBuffer::new(14400));
        }

        info!("Monitoring engine initialized with {} GPU(s)", gpus.len());

        Self {
            collector,
            buffers: RwLock::new(buffers),
            running: AtomicBool::new(false),
            gpus: RwLock::new(gpus),
            sample_hooks: std::sync::Mutex::new(vec![]),
        }
    }

    pub fn add_sample_hook<F>(&self, hook: F)
    where
        F: Fn(&GpuSample) + Send + Sync + 'static,
    {
        if let Ok(mut hooks) = self.sample_hooks.lock() {
            hooks.push(Box::new(hook));
        }
    }

    pub fn gpus(&self) -> Vec<GpuIdentity> {
        self.gpus.blocking_read().clone()
    }

    pub fn provider_name(&self) -> &str {
        self.collector.provider_name()
    }

    pub fn collect_once(&self, gpu_id: &str) -> Option<GpuSample> {
        let sample = self.collector.collect_sample(gpu_id)?;
        let mut buffers = self.buffers.blocking_write();
        if let Some(buf) = buffers.get_mut(gpu_id) {
            buf.push(sample.clone());
        }
        if let Ok(hooks) = self.sample_hooks.lock() {
            for hook in hooks.iter() {
                hook(&sample);
            }
        }
        Some(sample)
    }

    pub fn collect_all_once(&self) -> Vec<GpuSample> {
        let gpus = self.gpus.blocking_read();
        let mut samples = Vec::with_capacity(gpus.len());
        for gpu in gpus.iter() {
            if let Some(sample) = self.collect_once(&gpu.id) {
                samples.push(sample);
            }
        }
        samples
    }

    pub fn get_latest(&self, gpu_id: &str) -> Option<GpuSample> {
        let buffers = self.buffers.blocking_read();
        buffers.get(gpu_id).and_then(|b| b.get_latest().cloned())
    }

    pub fn get_history(&self, gpu_id: &str, count: usize) -> Vec<GpuSample> {
        let buffers = self.buffers.blocking_read();
        buffers
            .get(gpu_id)
            .map(|b| b.get_range(count).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_exported_samples(&self, gpu_id: &str, count: usize) -> Vec<ExportedSample> {
        let samples = self.get_history(gpu_id, count);
        let refs: Vec<&GpuSample> = samples.iter().collect();
        Exporter::export_samples(&refs)
    }

    pub fn get_aggregated(&self, gpu_id: &str, count: usize) -> Option<AggregatedMetrics> {
        let buffers = self.buffers.blocking_read();
        let samples = buffers.get(gpu_id)?;
        let refs = samples.get_range(count);
        if refs.is_empty() {
            return None;
        }
        Some(Aggregator::aggregate(gpu_id, &refs))
    }

    pub fn get_control_state(&self, gpu_id: &str) -> Result<GpuControlState, GpuProviderError> {
        self.collector.provider_ref().read_control_state(gpu_id)
    }

    pub fn set_fan_speed(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        self.collector.provider_ref().set_fan_speed(gpu_id, percent)
    }

    pub fn set_core_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        self.collector
            .provider_ref()
            .set_core_clock_offset(gpu_id, offset_mhz)
    }

    pub fn set_memory_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        self.collector
            .provider_ref()
            .set_memory_clock_offset(gpu_id, offset_mhz)
    }

    pub fn set_power_limit(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        self.collector.provider_ref().set_power_limit(gpu_id, percent)
    }

    pub fn set_voltage_offset(&self, gpu_id: &str, offset_mv: i32) -> Result<(), GpuProviderError> {
        self.collector
            .provider_ref()
            .set_voltage_offset(gpu_id, offset_mv)
    }

    /// Hand the fans back to the driver's automatic policy.
    pub fn set_fan_control_automatic(&self, gpu_id: &str) -> Result<(), GpuProviderError> {
        self.collector
            .provider_ref()
            .set_fan_control_automatic(gpu_id)
    }

    /// Restore driver defaults (zero offsets, default power limit, automatic
    /// fan control).
    pub fn reset_controls(&self, gpu_id: &str) -> Result<(), GpuProviderError> {
        self.collector.provider_ref().reset_controls(gpu_id)
    }

    /// Reads a full sample without touching the sample buffers, alert hooks,
    /// or the persistence layer. Powers the fast UI polling path (`get_gpu_sample`):
    /// the display fetches live data at its own rate while recording stays
    /// owned by the background loop.
    pub fn probe_sample(&self, gpu_id: &str) -> Option<GpuSample> {
        self.collector.provider_ref().read_sample(gpu_id).ok()
    }

    /// Reads a temperature without touching the sample buffers, alert hooks,
    /// or the persistence layer. Used by the fan curve task so curve ticks
    /// never distort the recorded monitoring history.
    pub fn probe_temperature(&self, gpu_id: &str) -> Option<f64> {
        self.collector
            .provider_ref()
            .read_sample(gpu_id)
            .ok()
            .and_then(|sample| sample.temperature_celsius)
    }

    pub fn export_csv(&self, gpu_id: &str, count: usize) -> String {
        let samples = self.get_history(gpu_id, count);
        let refs: Vec<&GpuSample> = samples.iter().collect();
        Exporter::export_to_csv(&refs)
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::Relaxed);
        info!("Monitoring engine started");
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        info!("Monitoring engine stopped");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::GpuProviderError;
    use std::sync::atomic::AtomicU32;
    use std::sync::Arc;

    /// Deterministic provider reporting one GPU with a fixed sample, and it
    /// counts how many times `read_sample` is invoked.
    struct FakeProvider {
        reads: AtomicU32,
    }

    impl FakeProvider {
        fn sample(gpu_id: &str) -> GpuSample {
            GpuSample {
                gpu_id: gpu_id.to_string(),
                timestamp: 0,
                temperature_celsius: Some(55.0),
                core_clock_mhz: Some(3000.0),
                memory_clock_mhz: Some(8000.0),
                memory_used_mb: Some(1024),
                memory_total_mb: Some(8192),
                fan_speed_percent: Some(40.0),
                fan_rpm: None,
                power_watts: Some(120.0),
                max_power_limit_watts: None,
                core_voltage_mv: None,
                core_utilization_percent: Some(30.0),
                memory_utilization_percent: None,
            }
        }
    }

    impl GpuProvider for FakeProvider {
        fn name(&self) -> &str {
            "fake"
        }

        fn enumerate(&self) -> Result<Vec<GpuIdentity>, GpuProviderError> {
            Ok(vec![GpuIdentity {
                id: "gpu-0".into(),
                name: "Fake GPU".into(),
                vendor: crate::hardware::GpuVendor::Unknown,
                index: 0,
                memory_total_mb: Some(8192),
            }])
        }

        fn read_sample(&self, gpu_id: &str) -> Result<GpuSample, GpuProviderError> {
            self.reads.fetch_add(1, Ordering::Relaxed);
            Ok(Self::sample(gpu_id))
        }

        fn read_control_state(&self, _gpu_id: &str) -> Result<GpuControlState, GpuProviderError> {
            Ok(GpuControlState::unavailable("gpu-0"))
        }

        fn set_fan_speed(&self, _gpu_id: &str, _percent: f64) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn set_core_clock_offset(&self, _gpu_id: &str, _offset_mhz: i32) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn set_memory_clock_offset(&self, _gpu_id: &str, _offset_mhz: i32) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn set_power_limit(&self, _gpu_id: &str, _percent: f64) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn set_voltage_offset(&self, _gpu_id: &str, _offset_mv: i32) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn set_fan_control_automatic(&self, _gpu_id: &str) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }

        fn reset_controls(&self, _gpu_id: &str) -> Result<(), GpuProviderError> {
            Err(GpuProviderError::NotAvailable("fake provider".into()))
        }
    }

    #[test]
    fn engine_probe_sample_is_side_effect_free_but_collect_once_records() {
        let engine = MonitoringEngine::new(Box::new(FakeProvider {
            reads: AtomicU32::new(0),
        }));
        let hook_calls = Arc::new(AtomicU32::new(0));

        // The fast-path probe returns the value, triggers hooks exactly zero
        // times and does not extend the buffer history.
        let sample = engine
            .probe_sample("gpu-0")
            .expect("probe returns the fake sample");
        assert_eq!(sample.temperature_celsius, Some(55.0));
        assert!(
            engine.get_latest("gpu-0").is_none(),
            "probe must not push into the ring buffer"
        );
        assert_eq!(hook_calls.load(Ordering::Relaxed), 0);

        // A recorded collection triggers the hook chain and the buffer.
        let counted = Arc::clone(&hook_calls);
        engine.add_sample_hook(move |_sample| {
            counted.fetch_add(1, Ordering::Relaxed);
        });
        engine.collect_once("gpu-0");
        assert_eq!(
            hook_calls.load(Ordering::Relaxed),
            1,
            "collect_once must feed the persistence hooks"
        );
        assert!(
            engine.get_latest("gpu-0").is_some(),
            "collect_once must push into the ring buffer"
        );
    }
}
