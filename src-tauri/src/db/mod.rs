pub mod day_items;
pub mod routines;
pub mod settings;

use std::path::Path;
use std::time::Duration;

use rusqlite::Connection;

use crate::error::{AppError, AppResult};

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

/// v2: 루틴의 시간대(조회 전 · 수업 중 · 방과 후). 예전 버전 앱도 이 열을 모르는 채로 그대로 읽고 쓸 수 있다.
const SCHEMA_V2: &str = "
ALTER TABLE routines ADD COLUMN slot TEXT CHECK (slot IN ('morning','class','after'));
";

/// v3: (1) 반복에 '매월'(month_day: 1–31일, 0은 마지막 날 — 주말 · 공휴일이면 바로 앞 평일로)과
/// '기간 한정'(active_from ~ active_until)을 더한다. repeat_type의 CHECK를 바꿔야 해서 routines 표를 다시 만든다.
/// (2) 지난 기록이 루틴을 고쳐도 그대로 남도록 day_items에 반복 종류 · 마감 시각 · 시간대를 함께 적는다.
/// 예전 버전 앱(복원 프로그램으로 C드라이브가 돌아간 PC)도 새 열을 모르는 채로 그대로 읽고 쓸 수 있다.
const SCHEMA_V3: &str = "
CREATE TABLE routines_v3 (
  id           INTEGER PRIMARY KEY,
  title        TEXT    NOT NULL,
  repeat_type  TEXT    NOT NULL CHECK (repeat_type IN ('daily','weekdays','once','monthly')),
  weekdays     INTEGER NOT NULL DEFAULT 0,
  once_date    TEXT,
  due_time     TEXT,
  link         TEXT,
  sort_order   INTEGER NOT NULL,
  created_at   TEXT    NOT NULL,
  archived_at  TEXT,
  slot         TEXT CHECK (slot IN ('morning','class','after')),
  month_day    INTEGER CHECK (month_day BETWEEN 0 AND 31),
  active_from  TEXT,
  active_until TEXT
);
INSERT INTO routines_v3 (id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at, slot)
  SELECT id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at, slot FROM routines;
DROP TABLE routines;
ALTER TABLE routines_v3 RENAME TO routines;
ALTER TABLE day_items ADD COLUMN repeat_type_snapshot TEXT;
ALTER TABLE day_items ADD COLUMN due_time_snapshot TEXT;
ALTER TABLE day_items ADD COLUMN slot_snapshot TEXT;
UPDATE day_items SET
  repeat_type_snapshot = (SELECT r.repeat_type FROM routines r WHERE r.id = day_items.routine_id),
  due_time_snapshot    = (SELECT r.due_time    FROM routines r WHERE r.id = day_items.routine_id),
  slot_snapshot        = (SELECT r.slot        FROM routines r WHERE r.id = day_items.routine_id);
";

pub fn open(path: &Path) -> AppResult<Connection> {
    let conn = Connection::open(path)?;
    prepare(&conn)?;
    Ok(conn)
}

#[cfg(test)]
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
    if version < 2 {
        let tx = conn.unchecked_transaction()?;
        tx.execute_batch(SCHEMA_V2)?;
        tx.pragma_update(None, "user_version", 2)?;
        tx.commit()?;
    }
    if version < 3 {
        if version >= 1 {
            keep_copy_before(conn, "pre-v3-g-routine.db");
        }
        // 표를 다시 만드는 동안에는 외래 키 검사를 끈다 (트랜잭션 밖에서만 바꿀 수 있다)
        conn.pragma_update(None, "foreign_keys", "OFF")?;
        let result = (|| -> AppResult<()> {
            let fk_broken = |c: &Connection| -> rusqlite::Result<i64> {
                c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| r.get(0))
            };
            // 예전부터 있던 어긋남 때문에 영영 열리지 않게 되지 않도록, 바꾸는 동안 새로 생긴 것만 본다
            let before = fk_broken(conn)?;
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(SCHEMA_V3)?;
            if fk_broken(&tx)? > before {
                return Err(AppError::invalid("데이터 구조를 바꾸지 못했어요"));
            }
            tx.pragma_update(None, "user_version", 3)?;
            tx.commit()?;
            Ok(())
        })();
        conn.pragma_update(None, "foreign_keys", "ON")?;
        result?;
    }
    Ok(())
}

/// 구조를 바꾸기 전 DB를 backups 폴더에 한 번 복사해 둔다 (매일 백업 7개 순환에 섞이지 않는 이름).
/// 실패해도 구조 바꾸기는 그대로 한다 (이 복사는 만일을 위한 것).
fn keep_copy_before(conn: &Connection, name: &str) {
    let Some(db) = conn.path().filter(|p| !p.is_empty()).map(Path::new) else { return };
    let Some(dir) = db.parent().map(|d| d.join("backups")) else { return };
    let target = dir.join(name);
    if target.exists() || std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    match conn.execute("VACUUM INTO ?1", [target.to_string_lossy()]) {
        Ok(_) => log::info!("구조를 바꾸기 전 DB를 {name}(으)로 보관했어요"),
        Err(e) => log::warn!("구조를 바꾸기 전 DB를 보관하지 못했어요: {e}"),
    }
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
        assert_eq!(version, 3);
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
    fn upgrading_a_v1_database_keeps_routines_and_adds_slot() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.pragma_update(None, "user_version", 1).unwrap();
        conn.execute(
            "INSERT INTO routines (title, repeat_type, sort_order, created_at) VALUES ('출결 확인', 'daily', 0, '2026-10-01T09:00:00')",
            [],
        )
        .unwrap();

        migrate(&conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);
        let list = routines::list_unarchived(&conn).unwrap();
        assert_eq!((list[0].title.as_str(), list[0].slot), ("출결 확인", None));
        // 정해진 값 말고는 넣을 수 없다
        assert!(conn.execute("UPDATE routines SET slot = 'lunch'", []).is_err());
        // 복원 프로그램으로 돌아간 예전 버전(v0.2.0)은 slot 없이 넣는다. 그래도 된다
        conn.execute(
            "INSERT INTO routines (title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at)
             VALUES ('공문 확인', 'daily', 0, NULL, NULL, NULL, 1, '2026-10-02T09:00:00')",
            [],
        )
        .unwrap();
        assert_eq!(routines::list_unarchived(&conn).unwrap()[1].slot, None);
    }

    #[test]
    fn upgrading_a_v2_database_rebuilds_routines_and_fills_snapshots() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        conn.execute(
            "INSERT INTO routines (title, repeat_type, due_time, slot, sort_order, created_at)
             VALUES ('출결 확인', 'daily', '09:00', 'morning', 0, '2026-10-01T09:00:00')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO day_items (day, routine_id, title_snapshot, sort_order, completed_at)
             VALUES ('2026-10-12', 1, '출결 확인', 0, '2026-10-12T08:40:00')",
            [],
        )
        .unwrap();

        migrate(&conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);
        let list = routines::list_unarchived(&conn).unwrap();
        assert_eq!((list[0].title.as_str(), list[0].due_time.as_deref()), ("출결 확인", Some("09:00")));
        let snap: (String, String, String) = conn
            .query_row("SELECT repeat_type_snapshot, due_time_snapshot, slot_snapshot FROM day_items", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .unwrap();
        assert_eq!(snap, ("daily".into(), "09:00".into(), "morning".into()));
        // 이제 '매월'을 넣을 수 있고, 외래 키 검사도 다시 켜져 있다
        conn.execute(
            "INSERT INTO routines (title, repeat_type, month_day, sort_order, created_at) VALUES ('월말 정리', 'monthly', 0, 1, '2026-10-02')",
            [],
        )
        .unwrap();
        assert!(conn
            .execute("INSERT INTO day_items (day, routine_id, title_snapshot, sort_order) VALUES ('2026-10-12', 99, 'x', 0)", [])
            .is_err());
        // 복원 프로그램으로 돌아간 예전 버전은 새 열 없이 넣는다. 그래도 된다
        conn.execute(
            "INSERT INTO day_items (day, routine_id, title_snapshot, sort_order) VALUES ('2026-10-13', 1, '출결 확인', 0)",
            [],
        )
        .unwrap();
        assert!(integrity_ok(&conn).unwrap());
    }

    fn v2_database(path: &Path) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.execute_batch(SCHEMA_V1).unwrap();
        conn.execute_batch(SCHEMA_V2).unwrap();
        conn.pragma_update(None, "user_version", 2).unwrap();
        conn.execute(
            "INSERT INTO routines (title, repeat_type, sort_order, created_at) VALUES ('출결 확인', 'daily', 0, '2026-10-01T09:00:00')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_copy_is_kept_before_the_v3_change() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join(DB_FILE);
        drop(v2_database(&db_path));
        let conn = open(&db_path).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 3);
        let copy = Connection::open(dir.path().join("backups").join("pre-v3-g-routine.db")).unwrap();
        let old: i64 = copy.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(old, 2, "바꾸기 전 모습 그대로");
        let titles: String = copy.query_row("SELECT title FROM routines", [], |r| r.get(0)).unwrap();
        assert_eq!(titles, "출결 확인");
    }

    #[test]
    fn a_failed_v3_change_leaves_the_v2_database_as_it_was() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join(DB_FILE);
        let conn = v2_database(&db_path);
        // 이름이 겹치는 표를 미리 만들어 구조 바꾸기가 도중에 실패하게 한다
        conn.execute_batch("CREATE TABLE routines_v3 (x INTEGER)").unwrap();
        assert!(migrate(&conn).is_err());
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 2);
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM routines", [], |r| r.get(0)).unwrap();
        assert_eq!(n, 1);
        let fk: i64 = conn.query_row("PRAGMA foreign_keys", [], |r| r.get(0)).unwrap();
        assert_eq!(fk, 1, "외래 키 검사가 다시 켜져 있어야 한다");
    }

    #[test]
    fn file_backed_migration_survives_reopen() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        // First open: creates schema
        {
            let conn = open(&db_path).unwrap();
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
            assert_eq!(version, 3);
            assert!(integrity_ok(&conn).unwrap());
        }

        // Reopen: should find existing schema and not error
        {
            let conn = open(&db_path).unwrap();
            let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
            assert_eq!(version, 3);
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
