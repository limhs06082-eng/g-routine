use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::{routines, settings};
use crate::domain::day::{fmt_day, parse_day};
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
    let mut st = c.prepare("SELECT key, value FROM settings WHERE key NOT LIKE 'window_%' AND key NOT IN ('synced_day', 'holidays_cache', 'seen_version') ORDER BY key")?;
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
    // 창 위치와 '바뀐 점' 안내를 본 버전은 이 PC의 것이라 그대로 둔다
    tx.execute("DELETE FROM settings WHERE key NOT LIKE 'window_%' AND key <> 'seen_version'", [])?;
    for r in &b.routines {
        tx.execute(
            "INSERT INTO routines (id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at, slot)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
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
                r.archived_at,
                r.slot.map(|s| s.as_str())
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
        // Skip window position settings to preserve UI state
        if k.starts_with("window_") || k == settings::SEEN_VERSION {
            continue;
        }
        // Use settings::apply for validation and upserting; silently skip invalid pairs
        let _ = settings::apply(&tx, k, v);
    }
    // 방학 기간은 두 날짜가 모두 올바르고 순서가 맞을 때만 함께 되살린다
    let find = |key: &str| b.settings.iter().find(|(k, _)| k == key).and_then(|(_, v)| parse_day(v));
    if let (Some(start), Some(end)) = (find("vacation_start"), find("vacation_end")) {
        if start <= end {
            settings::set(&tx, "vacation_start", &fmt_day(start))?;
            settings::set(&tx, "vacation_end", &fmt_day(end))?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn validate(b: &BackupFile) -> AppResult<()> {
    if b.app != APP || b.version != VERSION {
        return Err(AppError::invalid("G-routine 백업 파일이 아니에요"));
    }

    // Validate routines
    let mut routine_ids = HashSet::new();
    for r in &b.routines {
        // Check weekdays is valid (0-127)
        if r.weekdays > 0x7F {
            return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
        }
        // Check once_date format if present
        if let Some(ref date) = r.once_date {
            if parse_day(date).is_none() {
                return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
            }
        }
        // Check due_time format if present (must be exactly 5 chars and parse as HH:MM)
        if let Some(ref time) = r.due_time {
            if time.len() != 5 || chrono::NaiveTime::parse_from_str(time, "%H:%M").is_err() {
                return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
            }
        }
        if !routine_ids.insert(r.id) {
            return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
        }
    }

    // Validate day items
    let mut day_item_ids = HashSet::new();
    for d in &b.day_items {
        // Check day format
        if parse_day(&d.day).is_none() {
            return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
        }
        // Check routine_id refers to a routine in the file
        if !routine_ids.contains(&d.routine_id) {
            return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
        }
        // Check day item ids are unique
        if !day_item_ids.insert(d.id) {
            return Err(AppError::invalid("백업 파일 내용이 올바르지 않아요"));
        }
    }

    Ok(())
}

pub fn write_file(path: &Path, b: &BackupFile) -> AppResult<()> {
    fs::write(path, serde_json::to_string_pretty(b)?)?;
    Ok(())
}

pub fn read_file(path: &Path) -> AppResult<BackupFile> {
    let text = fs::read_to_string(path)?;
    // 메모장 등이 붙이는 UTF-8 BOM은 JSON 파서가 받지 않으므로 떼어 낸다
    let b: BackupFile = serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text))?;
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
        service::create_routine(&src, input_daily("출결 확인"), at("2026-10-12 09:00")).unwrap();
        let v = service::get_today(&src, at("2026-10-12 09:00")).unwrap();
        service::set_done(&src, v.pending[0].id, true, at("2026-10-12 09:10")).unwrap();
        settings::apply(&src, "theme", "mint").unwrap();
        settings::set_window_pos(&src, 10, 10).unwrap();
        settings::set_vacation(&src, Some(("2026-12-24", "2027-02-28"))).unwrap();
        let backup = export(&src, "2026-10-12T10:00:00").unwrap();
        let keys: Vec<&str> = backup.settings.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["theme", "vacation_end", "vacation_start"]);

        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("b.json");
        write_file(&path, &backup).unwrap();

        let dst = open_in_memory().unwrap();
        service::create_routine(&dst, input_daily("지워질 루틴"), at("2026-10-12 09:00")).unwrap();
        import(&dst, &read_file(&path).unwrap()).unwrap();
        let history = service::history_day(&dst, "2026-10-12").unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].title, "출결 확인");
        assert_eq!(history[0].completed_at.as_deref(), Some("2026-10-12T09:10:00"));
        let s = settings::load(&dst).unwrap();
        assert_eq!(s.theme, "mint");
        assert_eq!((s.vacation_start.as_deref(), s.vacation_end.as_deref()), (Some("2026-12-24"), Some("2027-02-28")));
    }

    #[test]
    fn slot_survives_export_and_import() {
        let src = open_in_memory().unwrap();
        let input = crate::model::RoutineInput { slot: Some(crate::model::Slot::Class), ..input_daily("진도 체크") };
        service::create_routine(&src, input, at("2026-10-12 09:00")).unwrap();
        let dst = open_in_memory().unwrap();
        import(&dst, &export(&src, "2026-10-12T10:00:00").unwrap()).unwrap();
        assert_eq!(routines::list_unarchived(&dst).unwrap()[0].slot, Some(crate::model::Slot::Class));
    }

    #[test]
    fn a_backup_from_before_slots_still_loads() {
        let json = r#"{"app":"g-routine","version":1,"exportedAt":"","routines":[{"id":1,"title":"출결 확인","repeatType":"daily","weekdays":0,"onceDate":null,"dueTime":null,"link":null,"sortOrder":0,"createdAt":"2026-10-01T09:00:00","archivedAt":null}],"dayItems":[],"settings":[]}"#;
        let b: BackupFile = serde_json::from_str(json).unwrap();
        assert_eq!(b.routines[0].slot, None);
    }

    #[test]
    fn the_seen_version_belongs_to_this_pc_and_is_not_restored() {
        let src = open_in_memory().unwrap();
        settings::apply(&src, settings::SEEN_VERSION, "0.2.0").unwrap();
        let backup = export(&src, "2026-10-12T10:00:00").unwrap();
        assert!(backup.settings.iter().all(|(k, _)| k != settings::SEEN_VERSION));

        let dst = open_in_memory().unwrap();
        settings::apply(&dst, settings::SEEN_VERSION, "0.3.0").unwrap();
        import(&dst, &backup).unwrap();
        assert_eq!(settings::load(&dst).unwrap().seen_version.as_deref(), Some("0.3.0"));
    }

    #[test]
    fn import_skips_a_vacation_in_the_wrong_order() {
        let src = open_in_memory().unwrap();
        settings::set_vacation(&src, Some(("2027-02-28", "2026-12-24"))).unwrap();
        let dst = open_in_memory().unwrap();
        import(&dst, &export(&src, "2026-10-12T10:00:00").unwrap()).unwrap();
        let s = settings::load(&dst).unwrap();
        assert_eq!((s.vacation_start, s.vacation_end), (None, None));
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

    #[test]
    fn import_preserves_window_position_and_skips_invalid_settings() {
        let dst = open_in_memory().unwrap();
        // Set window position in destination
        settings::set_window_pos(&dst, 10, 10).unwrap();

        // Create backup with window_x in settings plus valid and invalid settings
        let backup = BackupFile {
            app: APP.into(),
            version: VERSION,
            exported_at: "2026-10-12T10:00:00".into(),
            routines: vec![],
            day_items: vec![],
            settings: vec![
                ("window_x".to_string(), "5".to_string()),
                ("theme".to_string(), "mint".to_string()),
                ("bogus".to_string(), "1".to_string()),
            ],
        };

        import(&dst, &backup).unwrap();
        let loaded = settings::load(&dst).unwrap();
        assert_eq!(loaded.theme, "mint");
        // Verify window position was preserved (not overwritten by backup)
        assert_eq!(settings::window_pos(&dst).unwrap(), Some((10, 10)));
        // Verify bogus key was not imported
        assert_eq!(settings::get(&dst, "bogus").unwrap(), None);
    }

    #[test]
    fn rejects_backup_with_invalid_weekdays() {
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("bad.json");
        let backup = BackupFile {
            app: APP.into(),
            version: VERSION,
            exported_at: "2026-10-12T10:00:00".into(),
            routines: vec![Routine {
                id: 1,
                title: "test".into(),
                repeat_type: crate::model::RepeatType::Daily,
                weekdays: 200,  // Invalid: > 0x7F
                once_date: None,
                due_time: None,
                link: None,
                slot: None,
                sort_order: 0,
                created_at: "2026-10-12T00:00:00".into(),
                archived_at: None,
            }],
            day_items: vec![],
            settings: vec![],
        };
        write_file(&path, &backup).unwrap();
        assert!(read_file(&path).is_err());

        // Verify destination data is unchanged
        let dst = open_in_memory().unwrap();
        service::create_routine(&dst, input_daily("original"), at("2026-10-12 09:00")).unwrap();
        let before = dst.query_row("SELECT COUNT(*) FROM routines", [], |r| r.get::<_, i64>(0)).unwrap();
        let _ = import(&dst, &backup);
        let after = dst.query_row("SELECT COUNT(*) FROM routines", [], |r| r.get::<_, i64>(0)).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn reads_backup_with_utf8_bom() {
        let src = open_in_memory().unwrap();
        service::create_routine(&src, input_daily("출결 확인"), at("2026-10-12 09:00")).unwrap();
        let backup = export(&src, "2026-10-12T10:00:00").unwrap();
        let t = tempfile::tempdir().unwrap();
        let path = t.path().join("bom.json");
        let mut bytes = "\u{feff}".as_bytes().to_vec();
        bytes.extend(serde_json::to_string_pretty(&backup).unwrap().into_bytes());
        fs::write(&path, bytes).unwrap();
        assert_eq!(read_file(&path).unwrap(), backup);
    }
}
