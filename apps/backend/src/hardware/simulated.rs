use std::sync::Mutex;
use rand::Rng;

use super::provider::*;

pub struct SimulatedGpuProvider {
    gpus: Mutex<Vec<SimulatedGpuState>>,
}

struct SimulatedGpuState {
    identity: GpuIdentity,
    base_temp: f64,
    base_core_clock: f64,
    base_memory_clock: f64,
    base_power: f64,
    base_voltage: f64,
    fan_speed: f64,
    fan_rpm: u32,
    core_offset: i32,
    mem_offset: i32,
    power_limit: f64,
    voltage_offset: i32,
    time: f64,
    core_util: f64,
    mem_util: f64,
    mem_used: u64,
}

impl Default for SimulatedGpuProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulatedGpuProvider {
    pub fn new() -> Self {
        let gpus = vec![
            SimulatedGpuState {
                identity: GpuIdentity {
                    id: "gpu-sim-0".into(),
                    name: "NVIDIA GeForce RTX 4090 (Simulated)".into(),
                    vendor: GpuVendor::Nvidia,
                    index: 0,
                    memory_total_mb: Some(24576),
                },
                base_temp: 45.0,
                base_core_clock: 2235.0,
                base_memory_clock: 10501.0,
                base_power: 120.0,
                base_voltage: 1050.0,
                fan_speed: 30.0,
                fan_rpm: 1200,
                core_offset: 0,
                mem_offset: 0,
                power_limit: 100.0,
                voltage_offset: 0,
                time: 0.0,
                core_util: 25.0,
                mem_util: 30.0,
                mem_used: 6144,
            },
            SimulatedGpuState {
                identity: GpuIdentity {
                    id: "gpu-sim-1".into(),
                    name: "AMD Radeon RX 7900 XTX (Simulated)".into(),
                    vendor: GpuVendor::Amd,
                    index: 1,
                    memory_total_mb: Some(24576),
                },
                base_temp: 42.0,
                base_core_clock: 2500.0,
                base_memory_clock: 6250.0,
                base_power: 110.0,
                base_voltage: 1060.0,
                fan_speed: 28.0,
                fan_rpm: 1100,
                core_offset: 0,
                mem_offset: 0,
                power_limit: 100.0,
                voltage_offset: 0,
                time: 0.0,
                core_util: 20.0,
                mem_util: 25.0,
                mem_used: 8192,
            },
        ];

        Self {
            gpus: Mutex::new(gpus),
        }
    }

    fn simulate_sample(state: &mut SimulatedGpuState) -> GpuSample {
        let mut rng = rand::thread_rng();
        state.time += 0.1;

        let util_wobble = rng.gen_range(-5.0..5.0);
        state.core_util = (state.core_util + util_wobble).clamp(0.0, 100.0);
        state.mem_util = (state.mem_util + rng.gen_range(-3.0..3.0)).clamp(0.0, 100.0);

        let temp_offset = (state.core_util / 100.0) * 35.0 + rng.gen_range(-1.0..1.0);
        let load_factor = state.core_util / 100.0;

        let mem_total = state.identity.memory_total_mb.unwrap_or(0);
        state.mem_used = (((mem_total as f64)
            * ((state.mem_util / 100.0) + rng.gen_range(-0.02..0.02)))
            as u64)
            .clamp(0, mem_total);

        let clock_boost_mhz = (state.core_offset as f64) + (load_factor * 150.0) + rng.gen_range(-10.0..10.0);

        GpuSample {
            gpu_id: state.identity.id.clone(),
            timestamp: chrono::Utc::now().timestamp_millis(),
            temperature_celsius: Some(state.base_temp + temp_offset),
            core_clock_mhz: Some(state.base_core_clock + clock_boost_mhz),
            memory_clock_mhz: Some(state.base_memory_clock + state.mem_offset as f64 + rng.gen_range(-5.0..5.0)),
            memory_used_mb: Some(state.mem_used),
            memory_total_mb: state.identity.memory_total_mb,
            fan_speed_percent: Some(state.fan_speed + (load_factor * 15.0) + rng.gen_range(-1.0..1.0)),
            fan_rpm: Some((state.fan_rpm as f64 * (state.fan_speed / 30.0)) as u32 + rng.gen_range(0..50)),
            power_watts: Some(state.base_power * (0.3 + load_factor * 0.7) + rng.gen_range(-2.0..2.0)),
            max_power_limit_watts: None,
            core_voltage_mv: Some(state.base_voltage + state.voltage_offset as f64 + rng.gen_range(-5.0..5.0)),
            core_utilization_percent: Some(state.core_util),
            memory_utilization_percent: Some(state.mem_util),
        }
    }
}

impl GpuProvider for SimulatedGpuProvider {
    fn name(&self) -> &str {
        "simulated"
    }

    fn enumerate(&self) -> Result<Vec<GpuIdentity>, GpuProviderError> {
        let gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        Ok(gpus.iter().map(|g| g.identity.clone()).collect())
    }

    fn read_sample(&self, gpu_id: &str) -> Result<GpuSample, GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        Ok(Self::simulate_sample(state))
    }

    fn read_control_state(&self, gpu_id: &str) -> Result<GpuControlState, GpuProviderError> {
        let gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        Ok(GpuControlState {
            gpu_id: state.identity.id.clone(),
            target_fan_speed: Some(state.fan_speed),
            core_clock_offset_mhz: state.core_offset,
            memory_clock_offset_mhz: state.mem_offset,
            power_limit_percent: Some(state.power_limit),
            voltage_offset_mv: state.voltage_offset,
        })
    }

    fn set_fan_speed(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        state.fan_speed = percent.clamp(0.0, 100.0);
        state.fan_rpm = (1200.0 * (state.fan_speed / 30.0)) as u32;
        Ok(())
    }

    fn set_core_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        state.core_offset = offset_mhz.clamp(-500, 500);
        Ok(())
    }

    fn set_memory_clock_offset(&self, gpu_id: &str, offset_mhz: i32) -> Result<(), GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        state.mem_offset = offset_mhz.clamp(-2000, 2000);
        Ok(())
    }

    fn set_power_limit(&self, gpu_id: &str, percent: f64) -> Result<(), GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        state.power_limit = percent.clamp(50.0, 150.0);
        Ok(())
    }

    fn set_voltage_offset(&self, gpu_id: &str, offset_mv: i32) -> Result<(), GpuProviderError> {
        let mut gpus = self.gpus.lock().unwrap_or_else(|e| e.into_inner());
        let state = gpus
            .iter_mut()
            .find(|g| g.identity.id == gpu_id)
            .ok_or_else(|| GpuProviderError::GpuNotFound(gpu_id.to_string()))?;

        state.voltage_offset = offset_mv.clamp(-200, 200);
        Ok(())
    }
}
