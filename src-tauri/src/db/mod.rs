pub mod day_items;
pub mod routines;
pub mod settings;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::error::AppResult;

pub const DB_FILE: &str = "g-routine.db";

const SCHEMA_V1: &str = "
CREATE TABLE routines (
  id           INTEGER PRIMARY KEY,
  title        TEXT    NOT NULL,
  repeat_type  TEXT    NOT NULL CHECK (repeat_type IN ('daily','weekdays','once')),
  weekdays     INTEGER NOT NULL DEFAULT 0,
  once_date    TEXT,
  due_time     TEXT,
  link         TEXT,
  sort_order   INTEGER NOT NULL,
  created_at   TEXT    NOT NULL,
  archived_at  TEXT
);
CREATE TABLE day_items (
  id             INTEGER PRIMARY KEY,
  day            TEXT    NOT NULL,
  routine_id     INTEGER NOT NULL REFERENCES routines(id),
  title_snapshot TEXT    NOT NULL,
  sort_order     INTEGER NOT NULL,
  completed_at   TEXT,
  UNIQUE (day, routine_id)
);
CREATE INDEX idx_day_items_day ON day_items(day);
CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    prepare(&conn)?;
    Ok(conn)
}

pub fn open_in_memory() -> AppResult<Connection> {
    let conn = Connection::open_in_memory()?;
    prepare(&conn)?;
    Ok(conn)
}

fn prepare(conn: &Connection) -> AppResult<()> {
    conn.busy_timeout(Duration::from_secs(3))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "FULL")?;
    migrate(conn)
}

fn migrate(conn: &Connection) -> AppResult<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(SCHEMA_V1)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    }
    Ok(())
}

pub fn integrity_ok(conn: &Connection) -> AppResult<bool> {
    let result: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    Ok(result == "ok")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn migration_creates_tables_once_and_sets_version() {
        let conn = open_in_memory().unwrap();
        migrate(&conn).unwrap(); // 두 번째 호출은 아무것도 하지 않아야 한다
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 1);
        let tables: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('routines','day_items','settings')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tables, 3);
        assert!(integrity_ok(&conn).unwrap());
    }

    #[test]
    fn file_backed_migration_survives_reopen() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // First open: creates schema
        {
            let conn = open(&db_path).unwrap();
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
            assert_eq!(version, 1);
            assert!(integrity_ok(&conn).unwrap());
        }

        // Reopen: should find existing schema and not error
        {
            let conn = open(&db_path).unwrap();
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
            assert_eq!(version, 1);
            let tables: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('routines','day_items','settings')",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(tables, 3);
            assert!(integrity_ok(&conn).unwrap());
        }
    }
}
