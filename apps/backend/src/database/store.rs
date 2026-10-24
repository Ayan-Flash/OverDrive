use crate::hardware::GpuSample;
use crate::utils::error::AppResult;
use rusqlite::params;

use super::Database;

impl Database {
    pub fn insert_sample(&self, sample: &GpuSample) -> AppResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO samples (gpu_id, timestamp, temperature_celsius, core_clock_mhz,
             memory_clock_mhz, memory_used_mb, memory_total_mb, fan_speed_percent, fan_rpm,
             power_watts, core_voltage_mv, core_utilization_percent, memory_utilization_percent)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                sample.gpu_id,
                sample.timestamp,
                sample.temperature_celsius,
                sample.core_clock_mhz,
                sample.memory_clock_mhz,
                sample.memory_used_mb,
                sample.memory_total_mb,
                sample.fan_speed_percent,
                sample.fan_rpm,
                sample.power_watts,
                sample.core_voltage_mv,
                sample.core_utilization_percent,
                sample.memory_utilization_percent,
            ],
        )?;
        Ok(())
    }

    pub fn get_samples(
        &self,
        gpu_id: &str,
        limit: usize,
        offset: usize,
    ) -> AppResult<Vec<GpuSample>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT gpu_id, timestamp, temperature_celsius, core_clock_mhz,
             memory_clock_mhz, memory_used_mb, memory_total_mb, fan_speed_percent, fan_rpm,
             power_watts, core_voltage_mv, core_utilization_percent, memory_utilization_percent
             FROM samples WHERE gpu_id = ?1 ORDER BY timestamp DESC LIMIT ?2 OFFSET ?3",
        )?;

        let samples = stmt
            .query_map(params![gpu_id, limit as i64, offset as i64], |row| {
                Ok(GpuSample {
                    gpu_id: row.get(0)?,
                    timestamp: row.get(1)?,
                    temperature_celsius: row.get(2)?,
                    core_clock_mhz: row.get(3)?,
                    memory_clock_mhz: row.get(4)?,
                    memory_used_mb: row.get(5)?,
                    memory_total_mb: row.get(6)?,
                    fan_speed_percent: row.get(7)?,
                    fan_rpm: row.get(8)?,
                    power_watts: row.get(9)?,
                    max_power_limit_watts: None,
                    core_voltage_mv: row.get(10)?,
                    core_utilization_percent: row.get(11)?,
                    memory_utilization_percent: row.get(12)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(samples)
    }

    pub fn prune_samples(&self, gpu_id: &str, keep_count: usize) -> AppResult<u64> {
        let conn = self.conn();
        let deleted = conn.execute(
            "DELETE FROM samples WHERE gpu_id = ?1 AND id NOT IN (
                SELECT id FROM samples WHERE gpu_id = ?1 ORDER BY timestamp DESC LIMIT ?2
             )",
            params![gpu_id, keep_count as i64],
        )?;
        Ok(deleted as u64)
    }

    pub fn insert_alert_event(
        &self,
        id: &str,
        gpu_id: &str,
        rule_id: &str,
        metric: &str,
        severity: &str,
        message: &str,
        value: f64,
        threshold: f64,
        timestamp: i64,
    ) -> AppResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR IGNORE INTO alert_events (id, gpu_id, rule_id, metric, severity,
             message, value, threshold, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![id, gpu_id, rule_id, metric, severity, message, value, threshold, timestamp],
        )?;
        Ok(())
    }

    pub fn get_alert_events(
        &self,
        gpu_id: &str,
        limit: usize,
    ) -> AppResult<Vec<serde_json::Value>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, gpu_id, rule_id, metric, severity, message, value, threshold,
             timestamp, acknowledged
             FROM alert_events WHERE gpu_id = ?1 ORDER BY timestamp DESC LIMIT ?2",
        )?;

        let events = stmt
            .query_map(params![gpu_id, limit as i64], |row| {
                let acknowledged: i32 = row.get(9)?;
                Ok(serde_json::json!({
                    "id": row.get::<_, String>(0)?,
                    "gpu_id": row.get::<_, String>(1)?,
                    "rule_id": row.get::<_, String>(2)?,
                    "metric": row.get::<_, String>(3)?,
                    "severity": row.get::<_, String>(4)?,
                    "message": row.get::<_, String>(5)?,
                    "value": row.get::<_, f64>(6)?,
                    "threshold": row.get::<_, f64>(7)?,
                    "timestamp": row.get::<_, i64>(8)?,
                    "acknowledged": acknowledged != 0,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    pub fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let conn = self.conn();
        let result = conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        );
        match result {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let conn = self.conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> AppResult<Vec<(String, String)>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn acknowledge_alert(&self, alert_id: &str) -> AppResult<bool> {
        let conn = self.conn();
        let rows = conn.execute(
            "UPDATE alert_events SET acknowledged = 1 WHERE id = ?1",
            params![alert_id],
        )?;
        Ok(rows > 0)
    }

    pub fn clear_alerts(&self, gpu_id: &str) -> AppResult<u64> {
        let conn = self.conn();
        let deleted = if gpu_id.is_empty() {
            conn.execute("DELETE FROM alert_events", [])?
        } else {
            conn.execute(
                "DELETE FROM alert_events WHERE gpu_id = ?1",
                params![gpu_id],
            )?
        };
        Ok(deleted as u64)
    }

    pub fn clear_samples(&self, before_timestamp: i64) -> AppResult<u64> {
        let conn = self.conn();
        let deleted = conn.execute(
            "DELETE FROM samples WHERE timestamp < ?1",
            params![before_timestamp],
        )?;
        Ok(deleted as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nullable_gpu_sensors_persist_as_null() {
        let db = Database::in_memory().unwrap();
        let sample = GpuSample {
            gpu_id: "nullable-gpu".into(),
            timestamp: 1,
            temperature_celsius: Some(0.0),
            core_clock_mhz: None,
            memory_clock_mhz: None,
            memory_used_mb: None,
            memory_total_mb: None,
            fan_speed_percent: None,
            fan_rpm: None,
            power_watts: None,
            max_power_limit_watts: None,
            core_voltage_mv: None,
            core_utilization_percent: Some(0.0),
            memory_utilization_percent: None,
        };
        db.insert_sample(&sample).unwrap();
        let restored = db.get_samples("nullable-gpu", 1, 0).unwrap().remove(0);
        assert_eq!(restored.temperature_celsius, Some(0.0));
        assert_eq!(restored.core_clock_mhz, None);
        assert_eq!(restored.power_watts, None);
    }
}
