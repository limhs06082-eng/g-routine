use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;
use rusqlite::Connection;

use crate::db::{self, settings, DB_FILE};
use crate::domain::day::{business_day, fmt_day};
use crate::error::{is_corruption, AppError, AppResult};
use crate::state::{AppState, AppStatus};
use crate::storage::{backup, location};
use crate::templates;

pub fn open_checked(dir: &Path) -> AppResult<Connection> {
    let conn = db::open(&dir.join(DB_FILE))?;
    if !db::integrity_ok(&conn)? {
        return Err(AppError::Corrupt);
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
    set_ready_status(state, dir);
}

fn set_ready_status(state: &AppState, dir: &Path) {
    state.update_status(|s| {
        s.ready = true;
        s.corrupt = false;
        s.open_failed = false;
        s.previous_dir = None;
        s.data_dir = Some(dir.display().to_string());
        s.suggested_dir = dir.display().to_string();
    });
}

/// 앱 시작 시 저장 위치를 찾고, DB가 있으면 열고 그날 첫 백업을 만든다.
/// 열지 못하면 손상(corrupt)과 일시적 실패(open_failed)를 구분해 상태에 남긴다.
pub fn boot(state: &AppState, exe_dir: &Path, candidates: &[PathBuf], now: NaiveDateTime) {
    state.replace_conn(None);
    // 복구가 중간에 끊겨 본 DB 없이 복구본만 남은 폴더가 있으면, 저장 위치를 찾기 전에 마무리한다.
    let known_dirs = [Some(exe_dir.join("data")), location::read_location(&state.location_file)];
    for dir in known_dirs.iter().flatten().chain(candidates.iter()) {
        backup::recover_interrupted_restore(dir);
    }
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
        Err(e) => {
            let corrupt = is_corruption(&e);
            state.update_status(|s| {
                s.corrupt = corrupt;
                s.open_failed = !corrupt;
                s.previous_dir = Some(dir.display().to_string());
            });
        }
    }
}

/// 첫 실행 설정. 폴더에 DB가 이미 있으면 그대로 연결하고 템플릿은 무시한다.
/// 그 DB가 손상되었으면 옆에 보관(quarantine)하고 템플릿으로 새로 시작한다.
pub fn setup(state: &AppState, dir: &Path, template: &str, now: NaiveDateTime) -> AppResult<()> {
    templates::seeds(template)?;
    let st = state.status();
    let dir: PathBuf = if st.portable {
        PathBuf::from(&st.suggested_dir)
    } else {
        if dir.as_os_str().is_empty() {
            return Err(AppError::invalid("저장할 폴더를 골라 주세요"));
        }
        location::normalize_data_dir(dir)
    };
    if dir.as_os_str().is_empty() {
        return Err(AppError::invalid("저장할 폴더를 골라 주세요"));
    }
    fs::create_dir_all(&dir)?;
    let existing = if dir.join(DB_FILE).is_file() {
        match open_checked(&dir) {
            Ok(c) => Some(c),
            Err(e) if is_corruption(&e) => {
                backup::quarantine_db(&dir)?;
                None
            }
            Err(e) => return Err(e),
        }
    } else {
        None
    };
    let conn = match existing {
        Some(c) => c,
        None => {
            let c = db::open(&dir.join(DB_FILE))?;
            templates::apply(&c, template, now)?;
            c
        }
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
/// 고른 폴더는 자동 탐색되도록 `…\G-routine\data` 모양으로 맞춘다.
pub fn change_dir(state: &AppState, chosen: &Path, now: NaiveDateTime) -> AppResult<()> {
    let new_dir = location::normalize_data_dir(chosen);
    let new_dir = new_dir.as_path();
    let st = state.status();
    if st.portable {
        return Err(AppError::invalid("포터블 모드에서는 저장 위치를 바꿀 수 없어요"));
    }
    if st.data_dir.as_deref().map(Path::new) == Some(new_dir) {
        return Ok(());
    }
    fs::create_dir_all(new_dir)?;
    let target = new_dir.join(DB_FILE);
    // 복사부터 연결 교체까지 한 번의 잠금 안에서 처리한다.
    // 그 사이에 누른 체크는 잠금이 풀린 뒤 새 DB에 쓰이므로 옛 폴더에만 남지 않는다.
    state.swap_conn(|current| {
        if !target.is_file() {
            let temp = new_dir.join(format!("{DB_FILE}.tmp"));
            let _ = fs::remove_file(&temp);
            let copied = current
                .execute("VACUUM INTO ?1", [temp.to_string_lossy().to_string()])
                .map_err(AppError::from)
                .and_then(|_| fs::rename(&temp, &target).map_err(AppError::from));
            if let Err(e) = copied {
                let _ = fs::remove_file(&temp);
                return Err(e);
            }
        }
        let conn = open_checked(new_dir)?;
        location::write_location(&state.location_file, new_dir)?;
        let _ = run_daily_backup(&conn, new_dir, now);
        Ok(conn)
    })?;
    set_ready_status(state, new_dir);
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
        AppState::new(root.join("cfg").join("location.json"), root.join("app"))
    }

    fn data_in(root: &Path, name: &str) -> PathBuf {
        root.join(name).join("G-routine").join("data")
    }

    fn corrupt_files(dir: &Path) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with(&format!("{DB_FILE}.corrupt-")))
            .collect()
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
        let data = data_in(t.path(), "D");
        {
            let s = state_in(t.path());
            setup(&s, &data, "subject", at(NOW)).unwrap();
        }
        let s = state_in(t.path());
        setup(&s, &data, "homeroom", at(NOW)).unwrap();
        assert_eq!(routine_count(&s), 4);
    }

    #[test]
    fn setup_normalizes_chosen_folder_to_g_routine_data() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        let parent = t.path().join("D");
        setup(&s, &parent, "empty", at(NOW)).unwrap();
        let data = data_in(t.path(), "D");
        assert!(data.join(DB_FILE).is_file());
        assert_eq!(s.status().data_dir, Some(data.display().to_string()));
        assert_eq!(location::read_location(&s.location_file), Some(data));
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
        assert!(change_dir(&s, &t.path().join("other"), at(NOW)).is_err());
    }

    #[test]
    fn corrupt_db_is_reported_and_restorable_from_backup() {
        let t = tempfile::tempdir().unwrap();
        let data = data_in(t.path(), "D");
        {
            let s = state_in(t.path());
            setup(&s, &data, "homeroom", at(NOW)).unwrap();
        }
        fs::write(data.join(DB_FILE), b"this is not a database file at all").unwrap();
        let s = state_in(t.path());
        boot(&s, &t.path().join("app"), &[], at(NOW));
        let st = s.status();
        assert!(st.corrupt && !st.open_failed && !st.ready);
        assert_eq!(st.previous_dir, Some(data.display().to_string()));

        restore(&s, at(NOW)).unwrap();
        assert!(s.status().ready);
        assert_eq!(routine_count(&s), 6);
        assert_eq!(corrupt_files(&data).len(), 1);
    }

    #[test]
    fn is_corruption_distinguishes_garbage_from_cannot_open() {
        let t = tempfile::tempdir().unwrap();
        fs::write(t.path().join(DB_FILE), b"this is not a database file at all").unwrap();
        let err = open_checked(t.path()).err().expect("garbage must not open");
        assert!(is_corruption(&err));
        assert!(is_corruption(&AppError::Corrupt));

        let cant_open = AppError::Db(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            None,
        ));
        assert!(!is_corruption(&cant_open));
        assert!(!is_corruption(&AppError::invalid("x")));
    }

    #[test]
    fn locked_db_reports_open_failed_and_retry_succeeds_after_unlock() {
        let t = tempfile::tempdir().unwrap();
        let data = data_in(t.path(), "D");
        {
            let s = state_in(t.path());
            setup(&s, &data, "subject", at(NOW)).unwrap();
        }
        let locker = Connection::open(data.join(DB_FILE)).unwrap();
        locker.execute_batch("BEGIN EXCLUSIVE").unwrap();

        let s = state_in(t.path());
        boot(&s, &t.path().join("app"), &[], at(NOW)); // busy_timeout(3초) 뒤 실패
        let st = s.status();
        assert!(st.open_failed && !st.corrupt && !st.ready);
        assert_eq!(st.previous_dir, Some(data.display().to_string()));
        assert!(corrupt_files(&data).is_empty());

        locker.execute_batch("ROLLBACK").unwrap();
        drop(locker);
        boot(&s, &s.exe_dir.clone(), &[], at(NOW));
        let st = s.status();
        assert!(st.ready && !st.open_failed);
        assert_eq!(routine_count(&s), 4);
    }

    #[test]
    fn setup_on_garbage_db_quarantines_it_and_starts_fresh() {
        let t = tempfile::tempdir().unwrap();
        let data = data_in(t.path(), "D");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join(DB_FILE), b"this is not a database file at all").unwrap();

        let s = state_in(t.path());
        setup(&s, &data, "homeroom", at(NOW)).unwrap();
        assert!(s.status().ready);
        assert_eq!(routine_count(&s), 6);
        let kept = corrupt_files(&data);
        assert_eq!(kept.len(), 1);
        assert_eq!(fs::read(data.join(&kept[0])).unwrap(), b"this is not a database file at all");
    }

    #[test]
    fn change_dir_copies_database_and_updates_location() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        setup(&s, &data_in(t.path(), "A"), "subject", at(NOW)).unwrap();
        let b = data_in(t.path(), "B");
        change_dir(&s, &b, at(NOW)).unwrap();
        assert!(b.join(DB_FILE).is_file());
        assert!(!b.join(format!("{DB_FILE}.tmp")).exists());
        assert!(b.join("backups").join("g-routine-2026-10-05.db").is_file());
        assert_eq!(s.status().data_dir, Some(b.display().to_string()));
        assert_eq!(location::read_location(&s.location_file), Some(b));
        assert_eq!(routine_count(&s), 4);
    }

    #[test]
    fn change_dir_into_parent_folder_lands_in_g_routine_data() {
        let t = tempfile::tempdir().unwrap();
        let s = state_in(t.path());
        setup(&s, &data_in(t.path(), "A"), "subject", at(NOW)).unwrap();
        change_dir(&s, &t.path().join("D"), at(NOW)).unwrap();
        let landed = data_in(t.path(), "D");
        assert!(landed.join(DB_FILE).is_file());
        assert_eq!(s.status().data_dir, Some(landed.display().to_string()));
        assert_eq!(location::read_location(&s.location_file), Some(landed));
    }

    #[test]
    fn boot_finishes_a_restore_that_was_interrupted_by_power_loss() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("D").join("G-routine").join("data");
        {
            let s = state_in(t.path());
            setup(&s, &data, "subject", at(NOW)).unwrap();
        }
        // 예전 버전의 복구가 손상 파일을 옮긴 직후 끊긴 상태
        let backup = data.join("backups").join("g-routine-2026-10-05.db");
        fs::rename(data.join(DB_FILE), data.join(format!("{DB_FILE}.corrupt-20261005-090000"))).unwrap();
        fs::copy(&backup, crate::storage::backup::restore_tmp_path(&data)).unwrap();

        let s = state_in(t.path());
        boot(&s, &t.path().join("app"), &[], at(NOW));
        assert!(s.status().ready);
        assert_eq!(routine_count(&s), 4);
    }
}
