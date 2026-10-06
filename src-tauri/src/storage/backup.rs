use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::db::DB_FILE;
use crate::error::{AppError, AppResult};

pub const KEEP: usize = 7;
const PREFIX: &str = "g-routine-";

fn backups_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn list_backups(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name.starts_with(PREFIX) && name.ends_with(".db")
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

/// 업무일마다 한 번 DB 사본을 만든다. 이미 있으면 None.
pub fn daily_backup(c: &Connection, data_dir: &Path, day: &str) -> AppResult<Option<PathBuf>> {
    let dir = backups_dir(data_dir);
    fs::create_dir_all(&dir)?;
    let target = dir.join(format!("{PREFIX}{day}.db"));
    if target.exists() {
        return Ok(None);
    }
    // Use temp file to ensure atomicity: VACUUM INTO temp, then rename only on success
    let temp = dir.join(format!("{PREFIX}{day}.db.tmp"));
    let _ = fs::remove_file(&temp); // Delete any stale temp file
    match c.execute("VACUUM INTO ?1", [temp.to_string_lossy().to_string()]) {
        Ok(_) => match fs::rename(&temp, &target) {
            Ok(_) => {
                let files = list_backups(&dir);
                if files.len() > KEEP {
                    for old in &files[..files.len() - KEEP] {
                        let _ = fs::remove_file(old);
                    }
                }
                Ok(Some(target))
            }
            Err(e) => {
                let _ = fs::remove_file(&temp);
                Err(e.into())
            }
        },
        Err(e) => {
            let _ = fs::remove_file(&temp);
            Err(e.into())
        }
    }
}

pub fn latest_backup(data_dir: &Path) -> Option<PathBuf> {
    list_backups(&backups_dir(data_dir)).pop()
}

/// 백업 파일을 읽기 전용으로 열어 쓸 수 있는 G-routine DB인지 확인한다 (파일을 바꾸지 않는다).
fn is_valid_backup(path: &Path) -> bool {
    let check = || -> rusqlite::Result<bool> {
        let c = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
        let quick: String = c.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if quick != "ok" {
            return Ok(false);
        }
        let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version < 1 {
            return Ok(false);
        }
        let tables: i64 =
            c.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='routines'", [], |r| r.get(0))?;
        Ok(tables == 1)
    }; // 연결은 여기서 닫힌다
    check().unwrap_or(false)
}

fn quarantine_suffix() -> String {
    format!(".corrupt-{}", chrono::Local::now().format("%Y%m%d-%H%M%S"))
}

/// 같은 이름이 이미 있으면 -2, -3 …을 붙인다.
fn free_name(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let base = path.as_os_str().to_string_lossy().to_string();
    (2..)
        .map(|n| PathBuf::from(format!("{base}-{n}")))
        .find(|p| !p.exists())
        .expect("빈 이름은 언젠가 나온다")
}

/// 지금 DB 파일과 그 저널 파일들을 `.corrupt-<시각>` 이름으로 옆에 치워 둔다 (지우지 않는다).
pub fn quarantine_db(data_dir: &Path) -> AppResult<()> {
    let suffix = quarantine_suffix();
    for ext in ["", "-journal", "-wal", "-shm"] {
        let from = data_dir.join(format!("{DB_FILE}{ext}"));
        if from.exists() {
            let to = free_name(&data_dir.join(format!("{DB_FILE}{ext}{suffix}")));
            fs::rename(&from, &to)?;
        }
    }
    Ok(())
}

/// 가장 최근의 정상 백업으로 DB를 되돌린다. 손상된 DB는 지우지 않고 옆에 보관한다.
///
/// 백업은 새것부터 읽기 전용으로 검사한다. 호출하는 쪽은 이 DB의 연결을 먼저 닫아야 한다.
/// 복구 중에 쓰는 임시 복구본 경로
pub fn restore_tmp_path(data_dir: &Path) -> PathBuf {
    data_dir.join(format!("{DB_FILE}.restore.tmp"))
}

/// 손상된 DB를 지우지 않고 보관한다. 본 파일은 복사본으로 남기고(본 파일은 그 자리에 그대로 둔다),
/// 저널(-journal/-wal/-shm)은 옮겨 둔다. 저널이 남아 있으면 복구한 파일에 예전 변경이 다시 적용될 수 있다.
fn preserve_corrupt_copy(data_dir: &Path) -> AppResult<()> {
    let suffix = quarantine_suffix();
    let db_path = data_dir.join(DB_FILE);
    if db_path.exists() {
        fs::copy(&db_path, free_name(&data_dir.join(format!("{DB_FILE}{suffix}"))))?;
    }
    for ext in ["-journal", "-wal", "-shm"] {
        let from = data_dir.join(format!("{DB_FILE}{ext}"));
        if from.exists() {
            fs::rename(&from, free_name(&data_dir.join(format!("{DB_FILE}{ext}{suffix}"))))?;
        }
    }
    Ok(())
}

/// 가장 최근의 정상 백업으로 DB를 되돌린다. 호출 전에 DB 연결을 모두 닫아야 한다.
/// 순서: 백업 → 임시 복구본(검사) → 손상본 복사 보관 → 임시 복구본을 본 파일 위로 한 번에 교체.
/// 어느 단계에서 전원이 꺼져도 g-routine.db는 손상본이나 복구본 중 하나로 늘 존재한다.
pub fn restore_latest(data_dir: &Path) -> AppResult<()> {
    let mut backups = list_backups(&backups_dir(data_dir));
    backups.reverse(); // 새것부터

    let Some(backup_path) = backups.into_iter().find(|p| is_valid_backup(p)) else {
        return Err(AppError::invalid("복구할 수 있는 백업 파일이 없어요"));
    };
    let db_path = data_dir.join(DB_FILE);
    let temp_path = restore_tmp_path(data_dir);
    let _ = fs::remove_file(&temp_path);
    let prepared = fs::copy(&backup_path, &temp_path)
        .map_err(AppError::from)
        .and_then(|_| {
            if is_valid_backup(&temp_path) {
                Ok(())
            } else {
                Err(AppError::invalid("백업 파일을 복사하지 못했어요. 다시 시도해 주세요"))
            }
        })
        .and_then(|_| preserve_corrupt_copy(data_dir));
    if let Err(e) = prepared {
        let _ = fs::remove_file(&temp_path);
        return Err(e);
    }
    // Windows에서 fs::rename은 기존 파일을 덮어쓰며 한 번에 바꾼다 (MOVEFILE_REPLACE_EXISTING).
    fs::rename(&temp_path, &db_path)?;
    Ok(())
}

/// 예전 버전에서 복구가 중간에 끊겨 본 DB 없이 복구본만 남은 경우, 시작할 때 마무리한다.
/// 본 DB가 있으면 남은 복구본은 지운다. 복구본을 본 DB로 썼으면 true.
pub fn recover_interrupted_restore(data_dir: &Path) -> bool {
    let db_path = data_dir.join(DB_FILE);
    let temp_path = restore_tmp_path(data_dir);
    if !temp_path.exists() {
        return false;
    }
    if db_path.exists() {
        let _ = fs::remove_file(&temp_path);
        return false;
    }
    is_valid_backup(&temp_path) && fs::rename(&temp_path, &db_path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn daily_backup_once_per_day_and_rotates() {
        let t = tempfile::tempdir().unwrap();
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(daily_backup(&c, t.path(), "2026-10-01").unwrap().is_some());
        assert!(daily_backup(&c, t.path(), "2026-10-01").unwrap().is_none());
        for d in 2..=9 {
            daily_backup(&c, t.path(), &format!("2026-10-0{d}")).unwrap();
        }
        let files = list_backups(&backups_dir(t.path()));
        assert_eq!(files.len(), KEEP);
        assert!(files[0].ends_with("g-routine-2026-10-03.db"));
        assert!(latest_backup(t.path()).unwrap().ends_with("g-routine-2026-10-09.db"));
    }

    #[test]
    fn restore_latest_replaces_db_file() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            daily_backup(&c, t.path(), "2026-10-05").unwrap();
        }
        fs::write(t.path().join(DB_FILE), b"broken").unwrap();
        restore_latest(t.path()).unwrap();
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(db::integrity_ok(&c).unwrap());
    }

    #[test]
    fn restore_without_backups_fails() {
        let t = tempfile::tempdir().unwrap();
        assert!(restore_latest(t.path()).is_err());
    }

    #[test]
    fn restore_skips_corrupted_newest_backup() {
        let t = tempfile::tempdir().unwrap();
        // Create two backups
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            daily_backup(&c, t.path(), "2026-10-01").unwrap();
            daily_backup(&c, t.path(), "2026-10-02").unwrap();
        }
        // Corrupt the newest backup
        let backups = list_backups(&backups_dir(t.path()));
        fs::write(&backups[1], b"corrupted data").unwrap();
        // Restore should use the older one
        restore_latest(t.path()).unwrap();
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(db::integrity_ok(&c).unwrap());
    }

    fn routine_titles(path: &Path) -> Vec<String> {
        let c = db::open(path).unwrap();
        let mut st = c.prepare("SELECT title FROM routines ORDER BY id").unwrap();
        st.query_map([], |r| r.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    }

    fn corrupt_names(dir: &Path, prefix: &str) -> Vec<String> {
        fs::read_dir(dir)
            .unwrap()
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .filter(|n| n.starts_with(&format!("{prefix}.corrupt-")))
            .collect()
    }

    #[test]
    fn restore_skips_empty_newest_backup_and_uses_older_one() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            c.execute(
                "INSERT INTO routines (title, repeat_type, sort_order, created_at) VALUES ('예전 루틴', 'daily', 0, '2026-10-01T00:00:00')",
                [],
            )
            .unwrap();
            daily_backup(&c, t.path(), "2026-10-01").unwrap();
        }
        let newest = backups_dir(t.path()).join("g-routine-2026-10-02.db");
        fs::write(&newest, b"").unwrap();
        fs::write(t.path().join(DB_FILE), b"broken").unwrap();

        restore_latest(t.path()).unwrap();
        assert_eq!(routine_titles(&t.path().join(DB_FILE)), vec!["예전 루틴".to_string()]);
        // 검사가 빈 백업 파일을 DB로 바꿔 놓지 않아야 한다
        assert_eq!(fs::metadata(&newest).unwrap().len(), 0);
    }

    #[test]
    fn restore_quarantines_old_db_and_journal() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            daily_backup(&c, t.path(), "2026-10-05").unwrap();
        }
        fs::write(t.path().join(DB_FILE), b"broken").unwrap();
        fs::write(t.path().join(format!("{DB_FILE}-journal")), b"stale journal").unwrap();

        restore_latest(t.path()).unwrap();
        assert_eq!(corrupt_names(t.path(), DB_FILE).len(), 1);
        assert_eq!(corrupt_names(t.path(), &format!("{DB_FILE}-journal")).len(), 1);
        assert!(!t.path().join(format!("{DB_FILE}-journal")).exists());
        let kept = t.path().join(&corrupt_names(t.path(), DB_FILE)[0]);
        assert_eq!(fs::read(kept).unwrap(), b"broken");
        let c = db::open(&t.path().join(DB_FILE)).unwrap();
        assert!(db::integrity_ok(&c).unwrap());
    }

    #[test]
    fn interrupted_restore_is_finished_from_the_leftover_copy() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            c.execute(
                "INSERT INTO routines (title, repeat_type, sort_order, created_at) VALUES ('복구된 루틴', 'daily', 0, '2026-10-05T00:00:00')",
                [],
            )
            .unwrap();
            daily_backup(&c, t.path(), "2026-10-05").unwrap();
        }
        // 예전 버전이 손상 파일을 옮긴 직후 멈춘 상태: 본 DB는 없고 복구본만 남아 있다
        fs::remove_file(t.path().join(DB_FILE)).unwrap();
        fs::copy(backups_dir(t.path()).join("g-routine-2026-10-05.db"), restore_tmp_path(t.path())).unwrap();

        assert!(recover_interrupted_restore(t.path()));
        assert_eq!(routine_titles(&t.path().join(DB_FILE)), vec!["복구된 루틴".to_string()]);
        assert!(!restore_tmp_path(t.path()).exists());
    }

    #[test]
    fn leftover_copy_is_ignored_when_invalid_and_cleared_when_db_exists() {
        let t = tempfile::tempdir().unwrap();
        fs::write(restore_tmp_path(t.path()), b"half written").unwrap();
        assert!(!recover_interrupted_restore(t.path()));
        assert!(!t.path().join(DB_FILE).exists());

        drop(db::open(&t.path().join(DB_FILE)).unwrap());
        assert!(!recover_interrupted_restore(t.path()));
        assert!(!restore_tmp_path(t.path()).exists());
        assert!(t.path().join(DB_FILE).exists());
    }

    #[test]
    fn restore_never_moves_the_live_db_away() {
        let t = tempfile::tempdir().unwrap();
        {
            let c = db::open(&t.path().join(DB_FILE)).unwrap();
            daily_backup(&c, t.path(), "2026-10-05").unwrap();
        }
        fs::write(t.path().join(DB_FILE), b"broken").unwrap();
        restore_latest(t.path()).unwrap();
        // 손상본은 복사로 보관되고, 본 파일은 복구본으로 한 번에 바뀐다
        let kept = t.path().join(&corrupt_names(t.path(), DB_FILE)[0]);
        assert_eq!(fs::read(kept).unwrap(), b"broken");
        assert!(!restore_tmp_path(t.path()).exists());
        assert!(db::integrity_ok(&db::open(&t.path().join(DB_FILE)).unwrap()).unwrap());
    }
}
