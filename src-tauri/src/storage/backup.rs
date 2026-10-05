use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

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

/// Restore the most recent valid backup to the data directory.
///
/// Iterates through backups from newest to oldest, checking integrity with `db::integrity_ok`.
/// Caller must drop its own Connection to this database before calling.
pub fn restore_latest(data_dir: &Path) -> AppResult<()> {
    use crate::db;

    let mut backups = list_backups(&backups_dir(data_dir));
    backups.reverse(); // Iterate newest to oldest

    for backup_path in backups {
        // Check integrity without keeping connection open
        let valid = {
            match db::open(&backup_path) {
                Ok(c) => db::integrity_ok(&c).unwrap_or(false),
                Err(_) => false,
            }
        }; // Connection dropped here

        if valid {
            // Copy to temp file first, then rename to ensure atomicity
            let db_path = data_dir.join(DB_FILE);
            let temp_path = data_dir.join(format!("{DB_FILE}.restore.tmp"));
            fs::copy(&backup_path, &temp_path)?;
            fs::rename(&temp_path, &db_path)?;
            return Ok(());
        }
    }

    Err(AppError::invalid("복구할 수 있는 백업 파일이 없어요"))
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
}
