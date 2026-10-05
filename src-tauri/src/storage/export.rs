use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::Routine;

const APP: &str = "g-routine";
const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayItemRow {
    pub id: i64,
    pub day: String,
    pub routine_id: i64,
    pub title_snapshot: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub app: String,
    pub version: u32,
    pub exported_at: String,
    pub routines: Vec<Routine>,
    pub day_items: Vec<DayItemRow>,
    pub settings: Vec<(String, String)>,
}

fn all_routines(c: &Connection) -> AppResult<Vec<Routine>> {
    let mut ids = c.prepare("SELECT id FROM routines ORDER BY id")?;
    let ids: Vec<i64> = ids.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    ids.into_iter().map(|id| routines::get(c, id)).collect()
}

pub fn export(c: &Connection, now: &str) -> AppResult<BackupFile> {
    let mut st = c.prepare(
        "SELECT id, day, routine_id, title_snapshot, sort_order, completed_at FROM day_items ORDER BY id",
    )?;
    let day_items = st
        .query_map([], |r| {
            Ok(DayItemRow {
                id: r.get(0)?,
                day: r.get(1)?,
                routine_id: r.get(2)?,
                title_snapshot: r.get(3)?,
                sort_order: r.get(4)?,
                completed_at: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut st = c.prepare("SELECT key, value FROM settings WHERE key NOT LIKE 'window_%' ORDER BY key")?;
    let settings = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(BackupFile {
        app: APP.into(),
        version: VERSION,
        exported_at: now.into(),
        routines: all_routines(c)?,
        day_items,
        settings,
    })
}

/// 현재 데이터를 백업 내용으로 통째로 바꾼다 (창 위치 설정은 유지).
pub fn import(c: &Connection, b: &BackupFile) -> AppResult<()> {
    validate(b)?;
    let tx = c.unchecked_transaction()?;
    tx.execute("DELETE FROM day_items", [])?;
    tx.execute("DELETE FROM routines", [])?;
    tx.execute("DELETE FROM settings WHERE key NOT LIKE 'window_%'", [])?;
    for r in &b.routines {
        tx.execute(
            "INSERT INTO routines (id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                r.id,
                r.title,
                r.repeat_type.as_str(),
                i64::from(r.weekdays),
                r.once_date,
                r.due_time,
                r.link,
                r.sort_order,
                r.created_at,
                r.archived_at
            ],
        )?;
    }
    for d in &b.day_items {
        tx.execute(
            "INSERT INTO day_items (id, day, routine_id, title_snapshot, sort_order, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![d.id, d.day, d.routine_id, d.title_snapshot, d.sort_order, d.completed_at],
        )?;
    }
    for (k, v) in &b.settings {
        tx.execute("INSERT INTO settings (key, value) VALUES (?1, ?2)", params![k, v])?;
    }
    tx.commit()?;
    Ok(())
}

fn validate(b: &BackupFile) -> AppResult<()> {
    if b.app != APP || b.version != VERSION {
        return Err(AppError::invalid("G-routine 백업 파일이 아니에요"));
    }
    Ok(())
}

pub fn write_file(path: &Path, b: &BackupFile) -> AppResult<()> {
    fs::write(path, serde_json::to_string_pretty(b)?)?;
    Ok(())
}

pub fn read_file(path: &Path) -> AppResult<BackupFile> {
    let b: BackupFile = serde_json::from_str(&fs::read_to_string(path)?)?;
    validate(&b)?;
    Ok(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{open_in_memory, settings};
    use crate::service;
    use crate::test_util::{at, input_daily};

    #[test]
    fn export_then_import_round_trips_into_fresh_db() {
        let src = open_in_memory().unwrap();
        service::create_routine(&src, input_daily("출결 확인"), at("2026-10-05 09:00")).unwrap();
        let v = service::get_today(&src, at("2026-10-05 09:00")).unwrap();
        service::set_done(&src, v.pending[0].id, true, at("2026-10-05 09:10")).unwrap();
        settings::apply(&src, "theme", "mint").unwrap();
        settings::set_window_pos(&src, 10, 10).unwrap();
        let backup = export(&src, "2026-10-05T10:00:00").unwrap();
        assert_eq!(backup.settings, vec![("theme".to_string(), "mint".to_string())]);

        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("b.json");
        write_file(&path, &backup).unwrap();

        let dst = open_in_memory().unwrap();
        service::create_routine(&dst, input_daily("지워질 루틴"), at("2026-10-05 09:00")).unwrap();
        import(&dst, &read_file(&path).unwrap()).unwrap();
        let history = service::history_day(&dst, "2026-10-05").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].title, "출결 확인");
        assert_eq!(history[0].completed_at.as_deref(), Some("2026-10-05T09:10:00"));
        assert_eq!(settings::load(&dst).unwrap().theme, "mint");
    }

    #[test]
    fn rejects_foreign_json() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("x.json");
        fs::write(&path, r#"{"app":"other","version":1,"exportedAt":"","routines":[],"dayItems":[],"settings":[]}"#).unwrap();
        assert!(read_file(&path).is_err());
        fs::write(&path, "not json").unwrap();
        assert!(read_file(&path).is_err());
    }
}
