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
    c.execute("VACUUM INTO ?1", [target.to_string_lossy().to_string()])?;
    let files = list_backups(&dir);
    if files.len() > KEEP {
        for old in &files[..files.len() - KEEP] {
            let _ = fs::remove_file(old);
        }
    }
    Ok(Some(target))
}

pub fn latest_backup(data_dir: &Path) -> Option<PathBuf> {
    list_backups(&backups_dir(data_dir)).pop()
}

pub fn restore_latest(data_dir: &Path) -> AppResult<()> {
    let latest = latest_backup(data_dir).ok_or_else(|| AppError::invalid("복구할 백업 파일이 없어요"))?;
    fs::copy(latest, data_dir.join(DB_FILE))?;
    Ok(())
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
}
