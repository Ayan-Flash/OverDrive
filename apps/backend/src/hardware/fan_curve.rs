//! Custom GPU fan curve engine — the OverDrive equivalent of the classic
//! "custom fan curve": a list of (temperature, duty%) keyframes is applied
//! continuously while enabled. The background task drives ticks (see
//! `tick()` wiring in `commands`); this module stays a pure engine so it can
//! be unit-tested without a GPU or a provider.

use std::sync::{Mutex, MutexGuard};

use serde::{Deserialize, Serialize};

use super::provider::GpuProviderError;

/// How often the background task re-evaluates the curve (seconds).
pub const APPLY_INTERVAL_SECS: u64 = 1;
/// Re-apply only when the interpolated duty differs from the last applied
/// value by at least this much, avoiding pointless driver calls on flat
/// sections of the curve.
pub const DUTY_CHANGE_EPSILON: f64 = 1.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FanCurvePoint {
    /// GPU temperature in celsius.
    pub temperature_celsius: f64,
    /// Fan duty in percent (0–100).
    pub fan_percent: f64,
}

/// Sensible initial curve shown in the editor and used when the user enables
/// the curve without importing their own keyframes.
pub const DEFAULT_FAN_CURVE: &[FanCurvePoint] = &[
    FanCurvePoint { temperature_celsius: 30.0, fan_percent: 25.0 },
    FanCurvePoint { temperature_celsius: 40.0, fan_percent: 35.0 },
    FanCurvePoint { temperature_celsius: 50.0, fan_percent: 45.0 },
    FanCurvePoint { temperature_celsius: 60.0, fan_percent: 60.0 },
    FanCurvePoint { temperature_celsius: 70.0, fan_percent: 80.0 },
    FanCurvePoint { temperature_celsius: 80.0, fan_percent: 100.0 },
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FanCurveState {
    pub gpu_id: String,
    pub enabled: bool,
    pub points: Vec<FanCurvePoint>,
}

#[derive(Debug, Clone)]
struct EngineState {
    gpu_id: String,
    enabled: bool,
    points: Vec<FanCurvePoint>,
    last_applied_percent: Option<f64>,
}

pub struct FanCurveEngine {
    state: Mutex<EngineState>,
}

impl FanCurveEngine {
    pub fn new(gpu_id: impl Into<String>) -> Self {
        Self {
            state: Mutex::new(EngineState {
                gpu_id: gpu_id.into(),
                enabled: false,
                points: DEFAULT_FAN_CURVE.to_vec(),
                last_applied_percent: None,
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, EngineState> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn get_state(&self, gpu_id: &str) -> FanCurveState {
        let state = self.state();
        FanCurveState {
            gpu_id: gpu_id.to_string(),
            enabled: state.enabled && state.gpu_id == gpu_id,
            points: if state.gpu_id == gpu_id {
                state.points.clone()
            } else {
                // When the caller queries a different card, report the
                // template without claiming that card uses it.
                DEFAULT_FAN_CURVE.to_vec()
            },
        }
    }

    pub fn set_curve(&self, gpu_id: &str, points: Vec<FanCurvePoint>) -> Result<(), GpuProviderError> {
        validate_curve(&points)?;
        let mut state = self.state();
        // Authoring a curve for a different GPU takes the engine out of its
        // currently-engaged application loop; the user re-enables it there.
        if state.enabled && state.gpu_id != gpu_id {
            state.enabled = false;
        }
        state.gpu_id = gpu_id.to_string();
        state.points = normalize_points(points);
        state.last_applied_percent = None;
        Ok(())
    }

    pub fn enabled(&self) -> bool {
        self.state().enabled
    }

    pub fn curve_gpu_id(&self) -> Option<String> {
        let state = self.state();
        state.enabled.then(|| state.gpu_id.clone())
    }

    /// Enables the curve for a GPU. The first tick after enabling applies the
    /// interpolated duty for the current temperature.
    pub fn enable(&self, gpu_id: &str) {
        let mut state = self.state();
        state.gpu_id = gpu_id.to_string();
        state.enabled = true;
        state.last_applied_percent = None;
    }

    /// Disables the curve. Returns the GPU id that had been under custom
    /// control, so the caller can hand the fan back to the driver's automatic
    /// policy.
    pub fn disable(&self) -> Option<String> {
        let mut state = self.state();
        state.enabled = false;
        state.last_applied_percent = None;
        (!state.gpu_id.is_empty()).then(|| state.gpu_id.clone())
    }

    /// Interpolated duty for `temperature`, or `None` when no duty should be
    /// written now (curve disabled, belongs to another GPU, or the duty has
    /// not moved past the change epsilon since the last application).
    pub fn target_duty(&self, gpu_id: &str, temperature: f64) -> Option<f64> {
        let needs_write;
        let duty;
        {
            let state = self.state();
            if !state.enabled || state.gpu_id != gpu_id {
                return None;
            }
            duty = interpolate_fan_percent(&state.points, temperature)
                .ok_or_else(|| GpuProviderError::ControlError("fan curve has no usable points".into()))
                .ok()?;
            needs_write = state
                .last_applied_percent
                .map(|last| (last - duty).abs() >= DUTY_CHANGE_EPSILON)
                .unwrap_or(true);
        }
        needs_write.then_some(duty)
    }

    /// Records a successfully applied duty so subsequent ticks can skip
    /// redundant driver writes.
    pub fn note_applied(&self, gpu_id: &str, percent: f64) {
        let mut state = self.state();
        if state.gpu_id == gpu_id {
            state.last_applied_percent = Some(percent);
        }
    }
}

/// Piecewise-linear fan-curve interpolation.
///
/// Points must be sorted ascending by temperature (normalize_points does
/// this); out-of-range queries clamp to the nearest keyframe, matching the
/// rubber behavior of classic fan-curve editors.
pub fn interpolate_fan_percent(points: &[FanCurvePoint], temperature_celsius: f64) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    let mut sorted: Vec<&FanCurvePoint> = points.iter().collect();
    sorted.sort_by(|a, b| {
        a.temperature_celsius
            .partial_cmp(&b.temperature_celsius)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let first = sorted.first()?;
    let last = sorted.last()?;

    if temperature_celsius <= first.temperature_celsius {
        return Some(clamp_percent(first.fan_percent));
    }
    if temperature_celsius >= last.temperature_celsius {
        return Some(clamp_percent(last.fan_percent));
    }

    let mut lower: &FanCurvePoint = first;
    let mut upper: &FanCurvePoint = last;
    for window in sorted.windows(2) {
        let (a, b) = (window[0], window[1]);
        if temperature_celsius >= a.temperature_celsius && temperature_celsius <= b.temperature_celsius {
            lower = a;
            upper = b;
            break;
        }
    }

    let span = upper.temperature_celsius - lower.temperature_celsius;
    if span <= f64::EPSILON {
        return Some(clamp_percent(upper.fan_percent));
    }
    let t = (temperature_celsius - lower.temperature_celsius) / span;
    Some(clamp_percent(lower.fan_percent + t * (upper.fan_percent - lower.fan_percent)))
}

fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

/// Sort points by temperature and collapse duplicate temperatures (last wins).
pub fn normalize_points(mut points: Vec<FanCurvePoint>) -> Vec<FanCurvePoint> {
    points.sort_by(|a, b| {
        a.temperature_celsius
            .partial_cmp(&b.temperature_celsius)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut normalized: Vec<FanCurvePoint> = Vec::with_capacity(points.len());
    for point in points {
        let same_temperature = normalized
            .last()
            .map(|last| (last.temperature_celsius - point.temperature_celsius).abs() <= f64::EPSILON)
            .unwrap_or(false);
        if same_temperature {
            // Later keyframe replaces the earlier one at the same temperature.
            if let Some(last) = normalized.last_mut() {
                *last = point;
            }
        } else {
            normalized.push(point);
        }
    }
    normalized
}

pub fn validate_curve(points: &[FanCurvePoint]) -> Result<(), GpuProviderError> {
    if points.len() < 2 {
        return Err(GpuProviderError::ControlError(
            "a fan curve needs at least 2 keyframes".into(),
        ));
    }
    if points.len() > 20 {
        return Err(GpuProviderError::ControlError(
            "a fan curve supports at most 20 keyframes".into(),
        ));
    }
    for point in points {
        if !(0.0..=100.0).contains(&point.fan_percent) {
            return Err(GpuProviderError::ControlError(format!(
                "fan duty {} is outside 0-100%",
                point.fan_percent
            )));
        }
        if !(-30.0..=150.0).contains(&point.temperature_celsius) {
            return Err(GpuProviderError::ControlError(format!(
                "temperature {} is outside the sensible -30..150 °C range",
                point.temperature_celsius
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(pairs: &[(f64, f64)]) -> Vec<FanCurvePoint> {
        pairs
            .iter()
            .map(|(t, f)| FanCurvePoint { temperature_celsius: *t, fan_percent: *f })
            .collect()
    }

    #[test]
    fn interpolates_between_keyframes() {
        let curve = points(&[(30.0, 25.0), (50.0, 45.0), (70.0, 85.0)]);
        assert_eq!(interpolate_fan_percent(&curve, 30.0), Some(25.0));
        assert_eq!(interpolate_fan_percent(&curve, 40.0), Some(35.0));
        assert_eq!(interpolate_fan_percent(&curve, 50.0), Some(45.0));
        assert_eq!(interpolate_fan_percent(&curve, 60.0), Some(65.0));
        assert_eq!(interpolate_fan_percent(&curve, 70.0), Some(85.0));
    }

    #[test]
    fn clamps_below_first_and_above_last_keyframe() {
        let curve = points(&[(40.0, 30.0), (80.0, 100.0)]);
        assert_eq!(interpolate_fan_percent(&curve, 20.0), Some(30.0));
        assert_eq!(interpolate_fan_percent(&curve, 200.0), Some(100.0));
    }

    #[test]
    fn empty_curve_is_none() {
        assert_eq!(interpolate_fan_percent(&[], 50.0), None);
    }

    #[test]
    fn interpolates_monotonically_even_if_input_unsorted() {
        // The engine normalizes curves before storage; interpolation itself
        // must still be stable if callers hand over unsorted points.
        let curve = points(&[(70.0, 85.0), (30.0, 25.0)]);
        assert_eq!(interpolate_fan_percent(&curve, 50.0), Some(55.0));
    }

    #[test]
    fn duplicate_temperatures_last_wins_after_normalization() {
        let normalized = normalize_points(points(&[(40.0, 30.0), (40.0, 60.0), (60.0, 80.0)]));
        assert_eq!(normalized.len(), 2);
        assert_eq!(normalized[0].fan_percent, 60.0);
        assert_eq!(normalized[1].temperature_celsius, 60.0);
    }

    #[test]
    fn validate_rejects_single_point_and_out_of_range() {
        assert!(validate_curve(&points(&[(40.0, 50.0)])).is_err());
        assert!(validate_curve(&points(&[(40.0, 150.0), (60.0, 80.0)])).is_err());
        assert!(validate_curve(&points(&[(2000.0, 50.0), (60.0, 80.0)])).is_err());
        assert!(validate_curve(&points(&[(40.0, 50.0), (60.0, 80.0)])).is_ok());
    }

    #[test]
    fn engine_set_curve_normalizes_and_get_state_reports_template_for_other_gpu() {
        let engine = FanCurveEngine::new("nvidia-0");
        engine
            .set_curve("nvidia-0", points(&[(60.0, 80.0), (40.0, 30.0), (40.0, 55.0)]))
            .unwrap();
        let state = engine.get_state("nvidia-0");
        assert_eq!(state.points.len(), 2);
        assert_eq!(state.points[0].fan_percent, 55.0);
        assert!(!state.enabled);

        let other = engine.get_state("nvidia-1");
        assert!(!other.enabled);
        assert_eq!(other.points, DEFAULT_FAN_CURVE.to_vec());
    }

    #[test]
    fn engine_skips_redundant_duty_writes() {
        let engine = FanCurveEngine::new("nvidia-0");
        engine.set_curve("nvidia-0", points(&[(30.0, 20.0), (90.0, 100.0)])).unwrap();
        engine.enable("nvidia-0");

        // First tick must write (nothing applied yet). 50 °C sits 1/3 into a
        // 20→100% ramp over 30→90 °C, so the duty is 20 + ⅓·80 = 46⅔%.
        let first = engine.target_duty("nvidia-0", 50.0);
        assert!(first.is_some());
        let duty = first.unwrap();
        assert!((duty - 46.667).abs() < 0.01, "unexpected duty {duty}");
        engine.note_applied("nvidia-0", duty);
        // Same temp → skip.
        assert_eq!(engine.target_duty("nvidia-0", 50.0), None);
        // Small drift → skip (below epsilon).
        assert_eq!(engine.target_duty("nvidia-0", 50.2), None);
        // Large drift → write again (60 °C is midpoint → exactly 60%).
        assert_eq!(engine.target_duty("nvidia-0", 60.0), Some(60.0));

        // Disabling stops all writes and reports the GPU for policy restore.
        let gpu = engine.disable();
        assert_eq!(gpu.as_deref(), Some("nvidia-0"));
        assert_eq!(engine.target_duty("nvidia-0", 60.0), None);
    }
}
