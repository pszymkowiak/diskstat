use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::{Connection, params};

use super::walk::DirChild;

/// In-memory cache entry for a directory.
struct CachedDir {
    mtime_sec: i64,
    mtime_nsec: i64,
    children: Vec<DirChild>,
}

/// SQLite-backed scan cache with in-memory read path.
///
/// Strategy:
/// 1. On open: load ALL cached data from SQLite into a HashMap (fast bulk read)
/// 2. During parallel scan: lookup from HashMap (no locks, no SQLite queries)
/// 3. After scan: batch-write all new/updated dirs to SQLite in one transaction
pub struct ScanCache {
    db_path: PathBuf,
    /// In-memory index loaded at startup.
    index: HashMap<String, CachedDir>,
    /// New/updated dirs to write back after scan.
    pending_writes: Vec<(String, i64, i64, Vec<DirChild>)>,
}

impl ScanCache {
    /// Open cache and load existing data into memory.
    pub fn open(root: &Path) -> Result<Self, rusqlite::Error> {
        let cache_dir = dirs_cache().join("diskstat");
        let _ = std::fs::create_dir_all(&cache_dir);

        let hash = blake3::hash(root.to_string_lossy().as_bytes());
        let db_name = format!("{}.db", &hash.to_hex()[..16]);
        let db_path = cache_dir.join(&db_name);

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-8000;
             CREATE TABLE IF NOT EXISTS dirs (
                 path TEXT PRIMARY KEY,
                 mtime_sec INTEGER NOT NULL,
                 mtime_nsec INTEGER NOT NULL
             );
             CREATE TABLE IF NOT EXISTS entries (
                 dir_path TEXT NOT NULL,
                 name TEXT NOT NULL,
                 is_dir INTEGER NOT NULL,
                 size INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_entries_dir ON entries(dir_path);",
        )?;

        // Load everything into memory
        let mut index: HashMap<String, CachedDir> = HashMap::new();

        {
            // Load dir metadata
            let mut stmt = conn.prepare("SELECT path, mtime_sec, mtime_nsec FROM dirs")?;
            let dir_rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?;
            for (path, sec, nsec) in dir_rows.flatten() {
                index.insert(
                    path,
                    CachedDir {
                        mtime_sec: sec,
                        mtime_nsec: nsec,
                        children: Vec::new(),
                    },
                );
            }
        }

        {
            // Load children into their parent dirs
            let mut stmt = conn.prepare("SELECT dir_path, name, is_dir, size FROM entries")?;
            let entry_rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i32>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })?;
            for (dir_path, name, is_dir, size) in entry_rows.flatten() {
                if let Some(cached) = index.get_mut(&dir_path) {
                    cached.children.push(DirChild {
                        name,
                        is_dir: is_dir != 0,
                        size: size as u64,
                    });
                }
            }
        }

        Ok(ScanCache {
            db_path,
            index,
            pending_writes: Vec::new(),
        })
    }

    /// Lookup a directory from the in-memory cache.
    /// Returns cached children if the directory mtime hasn't changed.
    /// This is lock-free and safe to call from any thread (takes &self).
    pub fn lookup_dir(&self, dir_path: &Path) -> Option<Vec<DirChild>> {
        let (cur_sec, cur_nsec) = dir_mtime_ns(dir_path)?;
        let path_str = dir_path.to_string_lossy();

        let cached = self.index.get(path_str.as_ref())?;

        if cached.mtime_sec != cur_sec || cached.mtime_nsec != cur_nsec {
            return None;
        }

        if cached.children.is_empty() {
            return None;
        }

        Some(cached.children.clone())
    }

    /// Queue a directory for writing to the cache after the scan.
    pub fn queue_store(&mut self, dir_path: &Path, children: &[DirChild]) {
        let (mtime_sec, mtime_nsec) = match dir_mtime_ns(dir_path) {
            Some(m) => m,
            None => return,
        };
        let path_str = dir_path.to_string_lossy().to_string();
        let children_clone: Vec<DirChild> = children
            .iter()
            .map(|c| DirChild {
                name: c.name.clone(),
                is_dir: c.is_dir,
                size: c.size,
            })
            .collect();
        self.pending_writes
            .push((path_str, mtime_sec, mtime_nsec, children_clone));
    }

    /// Flush all pending writes to SQLite in one batch transaction.
    pub fn flush(&mut self) -> Result<(), rusqlite::Error> {
        if self.pending_writes.is_empty() {
            return Ok(());
        }

        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        conn.execute_batch("BEGIN")?;

        let mut dir_stmt = conn.prepare(
            "INSERT OR REPLACE INTO dirs (path, mtime_sec, mtime_nsec) VALUES (?1, ?2, ?3)",
        )?;
        let mut del_stmt = conn.prepare("DELETE FROM entries WHERE dir_path = ?1")?;
        let mut ins_stmt = conn.prepare(
            "INSERT INTO entries (dir_path, name, is_dir, size) VALUES (?1, ?2, ?3, ?4)",
        )?;

        for (path, sec, nsec, children) in &self.pending_writes {
            dir_stmt.execute(params![path, sec, nsec])?;
            del_stmt.execute(params![path])?;
            for child in children {
                ins_stmt.execute(params![path, &child.name, child.is_dir as i32, child.size as i64])?;
            }
        }

        drop(dir_stmt);
        drop(del_stmt);
        drop(ins_stmt);

        conn.execute_batch("COMMIT")?;
        self.pending_writes.clear();

        Ok(())
    }

    /// Invalidate the entire cache (forced rescan).
    pub fn invalidate_all(&mut self) -> Result<(), rusqlite::Error> {
        self.index.clear();
        self.pending_writes.clear();
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("DELETE FROM entries; DELETE FROM dirs;")?;
        Ok(())
    }
}

/// Get directory mtime as (seconds, nanoseconds) since epoch.
fn dir_mtime_ns(path: &Path) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta.modified().ok()?;
    let duration = mtime.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    Some((duration.as_secs() as i64, duration.subsec_nanos() as i64))
}

fn dirs_cache() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        PathBuf::from("/tmp")
    }
}
