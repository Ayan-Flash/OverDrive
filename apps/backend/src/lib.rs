// GPUControl Pro is under active development: many modules expose API that is
// wired incrementally (backup/marketplace/enterprise/AI tuning surfaces). Allow
// dead_code so that in-progress API doesn't bury actionable warnings. The large
// command signatures and boxed trait maps are inherent to the IPC design.
#![allow(dead_code, clippy::too_many_arguments, clippy::type_complexity)]

pub mod ai;
pub mod alerts;
pub mod automation;
pub mod commands;
pub mod database;
pub mod enterprise;
pub mod hardware;
pub mod sync;
pub mod integrations;
pub mod monitoring;
pub mod reporting;
pub mod overlay;
pub mod plugins;
pub mod remote;
pub mod stores;
pub mod utils;
pub mod backup;
pub mod marketplace;
pub mod updater;

use std::sync::Arc;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};

    tracing::info!("setup_tray: Creating menu items");
    let show_hide = MenuItemBuilder::with_id("show_hide", "Show/Hide").build(app)?;
    let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    tracing::info!("setup_tray: Building menu");
    let menu = MenuBuilder::new(app)
        .item(&show_hide)
        .item(&separator)
        .item(&quit)
        .build()?;

    let icon = app.default_window_icon().cloned();
    tracing::info!("setup_tray: default_window_icon exists: {}", icon.is_some());

    tracing::info!("setup_tray: Building tray icon");
    let mut tray = TrayIconBuilder::new();
    if let Some(icon) = icon {
        tray = tray.icon(icon);
    }
    let _tray = tray
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show_hide" => {
                if let Some(window) = app.get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    tracing::info!("setup_tray: Tray icon built successfully");
    Ok(())
}

#[tauri::command]
fn get_app_info() -> serde_json::Value {
    serde_json::json!({
        "name": "GPUControl Pro",
        "version": env!("CARGO_PKG_VERSION"),
        "platform": std::env::consts::OS,
    })
}

/// Entry point for GPUControl Pro. Lives in the library crate so the binary
/// stays a thin passthrough (see `main.rs`) — this avoids compiling every
/// module twice and keeps the app mobile-entry-point compatible.
pub fn run() {
    let _log_guard = utils::logging::init_logging();

    let app_state = Arc::new(commands::AppState::new());
    tracing::info!("AppState initialized successfully");

    tracing::info!("Starting tauri::Builder");
    tauri::Builder::default()
        // Single-instance MUST be the first plugin registered. When a second
        // launch is attempted, this callback runs in the ALREADY-RUNNING
        // process and the new process exits immediately, so we never spawn
        // duplicate GPUControl Pro processes. Instead we surface the existing
        // window (restoring it from the tray / minimized state).
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tracing::info!("Second instance attempted; focusing existing window");
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(app_state)
        .manage(updater::UpdaterState::new())
        .setup(|app| {
            let handle = app.handle().clone();
            updater::commands::spawn_auto_check(handle);

            setup_tray(app)?;

            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                tracing::info!("GPUControl Pro started (PID: {})", std::process::id());
            });

            // Explicitly show and focus the main window
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Minimize to tray on close
            if let Some(window) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        if let Some(win) = handle.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                });
            }

            tracing::info!("GPUControl Pro initialized");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_info,
            commands::get_cpu_info,
            commands::get_cpu_sample,
            commands::get_ram_telemetry,
            commands::get_storage_telemetry,
            commands::list_gpus,
            commands::get_gpu_data,
            commands::get_gpu_history,
            commands::get_gpu_control_state,
            commands::set_fan_speed,
            commands::set_core_clock_offset,
            commands::set_memory_clock_offset,
            commands::set_power_limit,
            commands::set_voltage_offset,
            commands::start_monitoring,
            commands::stop_monitoring,
            commands::is_monitoring_running,
            commands::export_csv,
            commands::save_profile,
            commands::load_profiles,
            commands::delete_profile,
            commands::apply_profile,
            commands::get_alert_rules,
            commands::get_alert_rules_for_gpu,
            commands::add_alert_rule,
            commands::remove_alert_rule,
            commands::update_alert_rule,
            commands::get_alert_history,
            commands::acknowledge_alert,
            commands::clear_alert_history,
            commands::start_remote_server,
            commands::stop_remote_server,
            commands::get_remote_server_status,
            commands::generate_api_key,
            commands::start_overlay,
            commands::stop_overlay,
            commands::is_overlay_running,
            commands::get_overlay_config,
            commands::update_overlay_config,
            commands::get_detected_games,
            commands::is_game_running,
            commands::get_overlay_data,
            commands::get_automation_rules,
            commands::create_automation_rule,
            commands::update_automation_rule,
            commands::delete_automation_rule,
            commands::toggle_automation_rule,
            commands::add_automation_condition,
            commands::add_automation_action,
            commands::start_automation_engine,
            commands::stop_automation_engine,
            commands::get_integration_config,
            commands::save_integration_config,
            commands::test_discord_webhook,
            commands::send_discord_alert,
            commands::send_discord_report,
            commands::start_obs_source,
            commands::stop_obs_source,
            commands::is_obs_running,
            commands::generate_report,
            commands::list_reports,
            commands::get_report,
            commands::delete_report,
            commands::export_report_csv,
            commands::get_enterprise_config,
            commands::save_enterprise_config,
            commands::list_group_policies,
            commands::create_group_policy,
            commands::update_group_policy,
            commands::delete_group_policy,
            commands::toggle_group_policy,
            commands::get_sync_status,
            commands::register_device,
            commands::unregister_device,
            commands::sync_now,
            commands::start_sync_client,
            commands::stop_sync_client,
            commands::update_sync_settings,
            commands::start_sync_server,
            commands::stop_sync_server,
            commands::is_sync_server_running,
            commands::get_ai_anomalies,
            commands::get_ai_suggestions,
            commands::clear_ai_anomalies,
            commands::dismiss_ai_suggestion,
            commands::run_ai_analysis,
            commands::predict_gpu_temperature,
            commands::predict_gpu_utilization,
            commands::tune_fan_curve,
            commands::tune_clock_offsets,
            commands::tune_power_limit,
            commands::get_tuning_profiles,
            commands::save_tuning_profile,
            commands::apply_tuning_profile,
            commands::get_smart_alert_status,
            commands::get_smart_baselines,
            commands::get_smart_context,
            commands::get_smart_suppressed,
            commands::update_smart_config,
            commands::reset_smart_baselines,
            commands::create_backup,
            commands::restore_backup,
            commands::list_backups,
            commands::delete_backup,
            commands::export_backup,
            commands::import_backup,
            commands::list_marketplace_profiles,
            commands::get_marketplace_profile,
            commands::publish_marketplace_profile,
            commands::delete_marketplace_profile,
            commands::rate_marketplace_profile,
            commands::download_marketplace_profile,
            commands::import_marketplace_profile,
            commands::export_marketplace_profile,
            commands::get_setting,
            commands::set_setting,
            commands::get_all_settings,
            updater::commands::check_update,
            updater::commands::start_update,
            updater::commands::get_update_status,
            updater::commands::set_auto_check,
            updater::commands::set_check_interval,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            tracing::error!(error = %e, "Tauri application exited with error");
        });
}
