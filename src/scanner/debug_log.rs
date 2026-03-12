use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};

/// Global debug database at ~/.cache/diskstat/diskstat.db
///
/// Tables:
///   metadata(key TEXT PK, value TEXT)
///   action_log(id INTEGER PK, ts TEXT, action TEXT, details TEXT)
pub struct DebugLog {
    conn: Connection,
}

impl DebugLog {
    pub fn open() -> Result<Self, rusqlite::Error> {
        let db_path = global_db_path();
        let _ = std::fs::create_dir_all(db_path.parent().unwrap());

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             CREATE TABLE IF NOT EXISTS metadata (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS action_log (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 ts TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now','localtime')),
                 action TEXT NOT NULL,
                 details TEXT NOT NULL DEFAULT '{}'
             );",
        )?;
        Ok(DebugLog { conn })
    }

    // ── Metadata ────────────────────────────────────────────────────────────

    pub fn set_last_scanned(&self, path: &Path) {
        let _ = self.conn.execute(
            "INSERT OR REPLACE INTO metadata (key, value) VALUES ('last_scanned', ?1)",
            params![path.to_string_lossy().as_ref()],
        );
    }

    pub fn get_last_scanned(&self) -> Option<PathBuf> {
        self.conn
            .query_row(
                "SELECT value FROM metadata WHERE key = 'last_scanned'",
                [],
                |row| row.get::<_, String>(0),
            )
            .ok()
            .map(PathBuf::from)
    }

    // ── Action log ──────────────────────────────────────────────────────────

    /// Log an action with JSON details.
    /// `details` should be a valid JSON string (or plain text).
    pub fn log(&self, action: &str, details: &str) {
        let _ = self.conn.execute(
            "INSERT INTO action_log (action, details) VALUES (?1, ?2)",
            params![action, details],
        );
    }

    /// Convenience: log with key-value pairs formatted as JSON.
    pub fn log_json(&self, action: &str, kvs: &[(&str, &str)]) {
        let mut json = String::from("{");
        for (i, (k, v)) in kvs.iter().enumerate() {
            if i > 0 {
                json.push_str(", ");
            }
            json.push('"');
            json.push_str(&escape_json(k));
            json.push_str("\": \"");
            json.push_str(&escape_json(v));
            json.push('"');
        }
        json.push('}');
        self.log(action, &json);
    }

    /// Trim old log entries, keeping the last N.
    pub fn trim(&self, keep: u32) {
        let _ = self.conn.execute(
            "DELETE FROM action_log WHERE id NOT IN (SELECT id FROM action_log ORDER BY id DESC LIMIT ?1)",
            params![keep],
        );
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn global_db_path() -> PathBuf {
    let cache_dir = if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".cache").join("diskstat")
    } else {
        PathBuf::from("/tmp").join("diskstat")
    };
    cache_dir.join("diskstat.db")
}
