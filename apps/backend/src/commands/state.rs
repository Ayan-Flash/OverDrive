use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, RwLock};

use crate::database::Database;
use crate::monitoring::MonitoringEngine;
use crate::alerts::AlertEngine;
use crate::remote::RemoteServer;
use crate::overlay::OverlayController;

pub struct AppState {
    pub monitoring: Arc<MonitoringEngine>,
    pub alerts: Arc<AlertEngine>,
    pub remote: RwLock<Option<RemoteServer>>,
    pub overlay: RwLock<Option<OverlayController>>,
    pub obs_source: crate::integrations::obs::ObsSource,
    pub sync_server: crate::sync::SyncServer,
    pub smart_alerts: Arc<crate::ai::SmartAlertManager>,
    pub cpu: Arc<crate::hardware::cpu::CpuMonitor>,
    pub fan_curve: Arc<crate::hardware::FanCurveEngine>,
    pub db: Arc<Database>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

impl AppState {
    pub fn new() -> Self {
        let db = Arc::new(
            match Database::open() {
                Ok(db) => db,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to open database, using in-memory fallback");
                    Database::in_memory().unwrap_or_else(|e| {
                        tracing::error!(error = %e, "Failed to create in-memory database, exiting");
                        std::process::exit(1);
                    })
                }
            },
        );

        let provider = crate::hardware::create_provider();
        let monitoring = Arc::new(MonitoringEngine::new(provider));
        tracing::info!("AppState::new: MonitoringEngine created");
        let alerts = Arc::new(AlertEngine::new(1000));
        tracing::info!("AppState::new: AlertEngine created");

        tracing::info!("AppState::new: Retrieving GPUs");
        let gpus = monitoring.gpus();
        tracing::info!("AppState::new: GPUs retrieved: {}", gpus.len());
        for gpu in gpus {
            alerts.create_default_rules(&gpu.id);
        }
        tracing::info!("AppState::new: Default rules created");

        // The fan curve engine binds to the first detected GPU; users can
        // retarget it from the tuning page and it stays inert until enabled.
        let fan_curve_target = monitoring
            .gpus()
            .first()
            .map(|gpu| gpu.id.clone())
            .unwrap_or_default();
        let fan_curve = Arc::new(crate::hardware::FanCurveEngine::new(fan_curve_target));

        let overlay = OverlayController::new(monitoring.clone());

        let smart_alerts = Arc::new(crate::ai::SmartAlertManager::new());
        // The sample hook pushes every collected sample into the loop's own
        // consumers (smart alerts feed continuously), but the database record
        // is throttled back to ~1 Hz (1 in 4 ticks at the 250 ms loop cadence)
        // so the recorded history is unchanged while sampling runs faster.
        let sa = smart_alerts.clone();
        let record_every = crate::commands::monitoring_commands::RECORD_EVERY_TICKS;
        let record_tick = AtomicU32::new(0);
        let db_hook = db.clone();
        monitoring.add_sample_hook(move |sample| {
            sa.feed_sample(sample);
            let tick = record_tick.fetch_add(1, Ordering::Relaxed) + 1;
            if tick % record_every == 0 {
                if let Err(e) = db_hook.insert_sample(sample) {
                    tracing::error!(error = %e, "Failed to persist sample");
                }
            }
        });

        Self {
            monitoring,
            alerts,
            remote: RwLock::new(None),
            overlay: RwLock::new(Some(overlay)),
            obs_source: crate::integrations::obs::ObsSource::new(9877),
            sync_server: crate::sync::SyncServer::new(9878),
            smart_alerts,
            cpu: Arc::new(crate::hardware::cpu::CpuMonitor::new()),
            fan_curve,
            db,
        }
    }
}

pub type SharedState = Arc<AppState>;
