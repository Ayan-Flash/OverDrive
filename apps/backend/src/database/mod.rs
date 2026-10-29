pub mod schema;
pub mod store;

use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

pub struct Database {
    conn: Mutex<Connection>,
    data_dir: PathBuf,
}

impl Database {
    pub fn open() -> Result<Self, rusqlite::Error> {
        let data_dir = match crate::utils::logging::log_dir().parent() {
            Some(p) => p.to_path_buf(),
            None => dirs_next::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("overdrive"),
        };

        std::fs::create_dir_all(&data_dir).ok();

        let db_path = data_dir.join("overdrive.db");
        let conn = Connection::open(&db_path)?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;

        schema::initialize(&conn)?;

        tracing::info!("Database opened at {:?}", db_path);

        Ok(Self {
            conn: Mutex::new(conn),
            data_dir,
        })
    }

    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        schema::initialize(&conn)?;
        tracing::warn!("Using in-memory database (disk persistence unavailable)");
        Ok(Self {
            conn: Mutex::new(conn),
            data_dir: PathBuf::from("."),
        })
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| {
            tracing::error!("Database mutex poisoned, recovering");
            e.into_inner()
        })
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }
}
