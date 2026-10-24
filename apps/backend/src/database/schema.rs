use rusqlite::Connection;

pub fn initialize(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch("PRAGMA journal_mode=WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            gpu_id TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            temperature_celsius REAL,
            core_clock_mhz REAL,
            memory_clock_mhz REAL,
            memory_used_mb INTEGER,
            memory_total_mb INTEGER,
            fan_speed_percent REAL,
            fan_rpm INTEGER,
            power_watts REAL,
            core_voltage_mv REAL,
            core_utilization_percent REAL,
            memory_utilization_percent REAL
        );

        CREATE INDEX IF NOT EXISTS idx_samples_gpu_time
            ON samples(gpu_id, timestamp DESC);

        CREATE TABLE IF NOT EXISTS alert_events (
            id TEXT PRIMARY KEY,
            gpu_id TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            metric TEXT NOT NULL,
            severity TEXT NOT NULL,
            message TEXT NOT NULL,
            value REAL NOT NULL,
            threshold REAL NOT NULL,
            timestamp INTEGER NOT NULL,
            acknowledged INTEGER NOT NULL DEFAULT 0
        );

        CREATE INDEX IF NOT EXISTS idx_alerts_gpu_time
            ON alert_events(gpu_id, timestamp DESC);

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS profiles (
            name TEXT NOT NULL,
            gpu_id TEXT NOT NULL,
            core_offset INTEGER NOT NULL DEFAULT 0,
            mem_offset INTEGER NOT NULL DEFAULT 0,
            voltage_offset INTEGER NOT NULL DEFAULT 0,
            fan_speed REAL NOT NULL DEFAULT 0,
            power_limit REAL NOT NULL DEFAULT 100,
            created_at TEXT NOT NULL,
            PRIMARY KEY (name, gpu_id)
        );

        CREATE TABLE IF NOT EXISTS automation_rules (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS automation_conditions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_id TEXT NOT NULL,
            metric TEXT NOT NULL,
            operator TEXT NOT NULL,
            value REAL NOT NULL,
            FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS automation_actions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_id TEXT NOT NULL,
            action_type TEXT NOT NULL,
            target TEXT NOT NULL,
            value REAL NOT NULL,
            FOREIGN KEY (rule_id) REFERENCES automation_rules(id) ON DELETE CASCADE
        );
        ",
    )?;

    let version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if version < 1 {
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
    }

    // Rebuild the legacy samples table once so unsupported GPU sensors can be stored as SQL NULL.
    if version < 2 {
        conn.execute_batch("BEGIN IMMEDIATE;
            DROP INDEX IF EXISTS idx_samples_gpu_time;
            CREATE TABLE samples_nullable (
                id INTEGER PRIMARY KEY AUTOINCREMENT, gpu_id TEXT NOT NULL, timestamp INTEGER NOT NULL,
                temperature_celsius REAL, core_clock_mhz REAL, memory_clock_mhz REAL,
                memory_used_mb INTEGER, memory_total_mb INTEGER, fan_speed_percent REAL, fan_rpm INTEGER,
                power_watts REAL, core_voltage_mv REAL, core_utilization_percent REAL, memory_utilization_percent REAL
            );
            INSERT INTO samples_nullable SELECT id,gpu_id,timestamp,temperature_celsius,core_clock_mhz,
                memory_clock_mhz,memory_used_mb,memory_total_mb,fan_speed_percent,fan_rpm,power_watts,
                core_voltage_mv,core_utilization_percent,memory_utilization_percent FROM samples;
            DROP TABLE samples;
            ALTER TABLE samples_nullable RENAME TO samples;
            CREATE INDEX idx_samples_gpu_time ON samples(gpu_id, timestamp DESC);
            INSERT INTO schema_version (version) VALUES (2);
            COMMIT;")?;
    }

    Ok(())
}
