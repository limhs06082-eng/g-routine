use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use rusqlite::Connection;

use crate::db::{self, settings, DB_FILE};
use crate::domain::day::{business_day, fmt_day};
use crate::error::{AppError, AppResult};
use crate::state::{AppState, AppStatus};
use crate::storage::{backup, location};
use crate::templates;

pub fn open_checked(dir: &Path) -> AppResult<Connection> {
    let conn = db::open(&dir.join(DB_FILE))?;
    if !db::integrity_ok(&conn)? {
        return Err(AppError::invalid("데이터 파일이 손상되었어요"));
    }
    Ok(conn)
}

pub fn run_daily_backup(c: &Connection, dir: &Path, now: NaiveDateTime) -> AppResult<()> {
    let s = settings::load(c)?;
    backup::daily_backup(c, dir, &fmt_day(business_day(now, s.day_start_hour)))?;
    Ok(())
}

fn mark_ready(state: &AppState, dir: &Path, conn: Connection) {
    state.replace_conn(Some(conn));
    state.update_status(|s| {
        s.ready = true;
        s.corrupt = false;
        s.previous_dir = None;
        s.data_dir = Some(dir.display().to_string());
        s.suggested_dir = dir.display().to_string();
    });
}

/// 앱 시작 시 저장 위치를 찾고, DB가 있으면 열고 그날 첫 백업을 만든다.
pub fn boot(state: &AppState, exe_dir: &Path, candidates: &[PathBuf], now: NaiveDateTime) {
    let res = location::resolve(exe_dir, &state.location_file, candidates);
    let suggested = res.dir.clone().unwrap_or_else(location::suggested_dir);
    state.update_status(|s| {
        *s = AppStatus {
            portable: res.portable,
            previous_dir: res.previous.as_ref().map(|p| p.display().to_string()),
            suggested_dir: suggested.display().to_string(),
            ..AppStatus::default()
        };
    });
    let Some(dir) = res.dir else { return };
    if !dir.join(DB_FILE).is_file() {
        return;
    }
    match open_checked(&dir) {
        Ok(conn) => {
            let _ = run_daily_backup(&conn, &dir, now);
            mark_ready(state, &dir, conn);
        }
        Err(_) => state.update_status(|s| {
            s.corrupt = true;
            s.previous_dir = Some(dir.display().to_string());
        }),
    }
}

/// 첫 실행 설정. 폴더에 DB가 이미 있으면 그대로 연결하고 템플릿은 무시한다.
pub fn setup(state: &AppState, dir: &Path, template: &str, now: NaiveDateTime) -> AppResult<()> {
    templates::seeds(template)?;
    let st = state.status();
    let dir: PathBuf = if st.portable { PathBuf::from(&st.suggested_dir) } else { dir.to_path_buf() };
    if dir.as_os_str().is_empty() {
        return Err(AppError::invalid("저장할 폴더를 골라 주세요"));
    }
    fs::create_dir_all(&dir)?;
    let conn = if dir.join(DB_FILE).is_file() {
        open_checked(&dir)?
    } else {
        let c = db::open(&dir.join(DB_FILE))?;
        templates::apply(&c, template, now)?;
        c
    };
    if !st.portable {
        location::write_location(&state.location_file, &dir)?;
    }
    let _ = run_daily_backup(&conn, &dir, now);
    mark_ready(state, &dir, conn);
    Ok(())
}

/// 손상된 DB를 가장 최근 자동 백업으로 되돌린다.
pub fn restore(state: &AppState, now: NaiveDateTime) -> AppResult<()> {
    let st = state.status();
    let dir = st
        .previous_dir
        .map(PathBuf::from)
        .ok_or_else(|| AppError::invalid("복구할 폴더를 찾지 못했어요"))?;
    backup::restore_latest(&dir)?;
    let conn = open_checked(&dir)?;
    if !st.portable {
        location::write_location(&state.location_file, &dir)?;
    }
    let _ = run_daily_backup(&conn, &dir, now);
    mark_ready(state, &dir, conn);
    Ok(())
}

/// 저장 폴더 변경. 새 폴더에 DB가 있으면 그것에 연결하고, 없으면 현재 DB를 복사한다.
pub fn change_dir(state: &AppState, new_dir: &Path) -> AppResult<()> {
    let st = state.status();
    if st.portable {
        return Err(AppError::invalid("포터블 모드에서는 저장 위치를 바꿀 수 없어요"));
    }
    if st.data_dir.as_deref().map(Path::new) == Some(new_dir) {
        return Ok(());
    }
    fs::create_dir_all(new_dir)?;
    let target = new_dir.join(DB_FILE);
    if !target.is_file() {
        state.with_conn(|c| {
            c.execute("VACUUM INTO ?1", [target.to_string_lossy().to_string()])?;
            Ok(())
        })?;
    }
    let conn = open_checked(new_dir)?;
    location::write_location(&state.location_file, new_dir)?;
    mark_ready(state, new_dir, conn);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::routines;
    use crate::service;
    use crate::test_util::at;

    const NOW: &str = "2026-10-05 09:00";

    fn state_in(root: &Path) -> AppState {
        AppState::new(root.join("cfg").join("location.json"))
    }

    fn routine_count(s: &AppState) -> usize {
        s.with_conn(|c| Ok(routines::list_unarchived(c)?.len())).unwrap()
    }

    #[test]
    fn first_boot_needs_setup_then_setup_seeds_template() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        let data = t.path().join("D").join("G-routine").join("data");
        let state = state_in(t.path());
        boot(&state, &exe, &[], at(NOW));
        assert!(!state.status().ready);

        setup(&state, &data, "homeroom", at(NOW)).unwrap();
        let st = state.status();
        assert!(st.ready);
        assert_eq!(st.data_dir, Some(data.display().to_string()));
        let v = state.with_conn(|c| service::get_today(c, at(NOW))).unwrap();
        assert_eq!(v.pending.len(), 5); // 금요일 전용 1개 제외
        assert!(data.join("backups").join("g-routine-2026-10-05.db").is_file());

        drop(state);
        let again = state_in(t.path());
        boot(&again, &exe, &[], at(NOW));
        assert!(again.status().ready);
    }

    #[test]
    fn boot_reconnects_from_candidates_when_location_file_is_wiped() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("D").join("G-routine").join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "empty", at(NOW)).unwrap();
        }
        fs::remove_dir_all(t.path().join("cfg")).unwrap();
        let state = state_in(t.path());
        boot(&state, &t.path().join("app"), &[data.clone()], at(NOW));
        assert!(state.status().ready);
        assert_eq!(location::read_location(&state.location_file), Some(data));
    }

    #[test]
    fn setup_on_existing_db_keeps_data_and_ignores_template() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "subject", at(NOW)).unwrap();
        }
        let s = state_in(t.path());
        setup(&s, &data, "homeroom", at(NOW)).unwrap();
        assert_eq!(routine_count(&s), 4);
    }

    #[test]
    fn setup_rejects_unknown_template_before_touching_disk() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        assert!(setup(&s, &t.path().join("data"), "nope", at(NOW)).is_err());
        assert!(!t.path().join("data").exists());
    }

    #[test]
    fn portable_mode_uses_exe_data_folder() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        fs::create_dir_all(exe.join("data")).unwrap();
        let s = state_in(t.path());
        boot(&s, &exe, &[], at(NOW));
        let st = s.status();
        assert!(st.portable && !st.ready);

        setup(&s, Path::new("ignored"), "empty", at(NOW)).unwrap();
        assert!(exe.join("data").join(DB_FILE).is_file());
        assert!(!s.location_file.exists());
        assert!(change_dir(&s, &t.path().join("other")).is_err());
    }

    #[test]
    fn corrupt_db_is_reported_and_restorable_from_backup() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "homeroom", at(NOW)).unwrap();
        }
        fs::write(data.join(DB_FILE), b"this is not a database file at all").unwrap();
        let s = state_in(t.path());
        boot(&s, &t.path().join("app"), &[], at(NOW));
        let st = s.status();
        assert!(st.corrupt && !st.ready);
        assert_eq!(st.previous_dir, Some(data.display().to_string()));

        restore(&s, at(NOW)).unwrap();
        assert!(s.status().ready);
        assert_eq!(routine_count(&s), 6);
    }

    #[test]
    fn change_dir_copies_database_and_updates_location() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        setup(&s, &t.path().join("a"), "subject", at(NOW)).unwrap();
        let b = t.path().join("b");
        change_dir(&s, &b).unwrap();
        assert!(b.join(DB_FILE).is_file());
        assert_eq!(s.status().data_dir, Some(b.display().to_string()));
        assert_eq!(location::read_location(&s.location_file), Some(b));
        assert_eq!(routine_count(&s), 4);
    }
}
