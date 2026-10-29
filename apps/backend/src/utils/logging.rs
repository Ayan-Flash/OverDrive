use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Default log directory, computed without requiring logging to be initialized.
/// Used both by `init_logging` and as the fallback for `log_dir` so that code
/// paths which resolve paths (e.g. the database) never depend on init order.
fn default_log_dir() -> PathBuf {
    dirs_next::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("overdrive")
        .join("logs")
}

/// Returns the active log directory. Falls back to the default location when
/// logging has not been initialized (e.g. in unit tests) instead of panicking.
pub fn log_dir() -> PathBuf {
    LOG_DIR.get().cloned().unwrap_or_else(default_log_dir)
}

pub fn init_logging() -> Option<WorkerGuard> {
    let data_dir = default_log_dir();

    let _ = LOG_DIR.set(data_dir.clone());
    fs::create_dir_all(&data_dir).ok()?;

    let file_appender = tracing_appender::rolling::daily(&data_dir, "overdrive.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into());

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    std::panic::set_hook(Box::new(move |panic_info| {
        let msg = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        tracing::error!(target: "panic", "PANIC: {} at {}", msg, location);

        let crash_path = data_dir.join("crash.log");
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
        {
            use std::io::Write;
            let _ = writeln!(
                f,
                "[{}] PANIC: {} at {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                msg,
                location
            );
        }
    }));

    Some(guard)
}
