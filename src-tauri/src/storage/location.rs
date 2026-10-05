use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::DB_FILE;
use crate::error::AppResult;

#[derive(Debug, Clone, PartialEq)]
pub struct Resolution {
    /// 사용할 데이터 폴더. Some이어도 DB 파일이 아직 없을 수 있다 (포터블 첫 실행).
    pub dir: Option<PathBuf>,
    pub portable: bool,
    /// location.json에 적혀 있었지만 찾지 못한 폴더
    pub previous: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocationFile {
    data_dir: PathBuf,
}

pub fn read_location(file: &Path) -> Option<PathBuf> {
    let text = fs::read_to_string(file).ok()?;
    serde_json::from_str::<LocationFile>(&text).ok().map(|l| l.data_dir)
}

pub fn write_location(file: &Path, dir: &Path) -> AppResult<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&LocationFile { data_dir: dir.to_path_buf() })?;
    fs::write(file, text)?;
    Ok(())
}

/// 저장 위치 결정 순서: ① exe 옆 data 폴더(포터블) ② location.json ③ 후보 드라이브 탐색
pub fn resolve(exe_dir: &Path, location_file: &Path, candidates: &[PathBuf]) -> Resolution {
    let portable = exe_dir.join("data");
    if portable.is_dir() {
        return Resolution { dir: Some(portable), portable: true, previous: None };
    }
    let previous = read_location(location_file);
    if let Some(p) = &previous {
        if p.join(DB_FILE).is_file() {
            return Resolution { dir: Some(p.clone()), portable: false, previous: None };
        }
    }
    for c in candidates {
        if c.join(DB_FILE).is_file() {
            let _ = write_location(location_file, c);
            return Resolution { dir: Some(c.clone()), portable: false, previous: None };
        }
    }
    Resolution { dir: None, portable: false, previous }
}

/// D:~Z: 중 존재하는 드라이브의 \G-routine\data (D 우선)
pub fn drive_candidates() -> Vec<PathBuf> {
    ('D'..='Z')
        .map(|l| PathBuf::from(format!("{l}:\\")))
        .filter(|root| root.exists())
        .map(|root| root.join("G-routine").join("data"))
        .collect()
}

pub fn suggested_dir() -> PathBuf {
    let d = PathBuf::from("D:\\");
    if d.exists() {
        return d.join("G-routine").join("data");
    }
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("C:\\"));
    home.join("Documents").join("G-routine").join("data")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DB_FILE;
    use std::fs;

    fn touch_db(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(DB_FILE), b"x").unwrap();
    }

    #[test]
    fn portable_data_folder_wins() {
        let t = tempfile::tempdir().unwrap();
        let exe = t.path().join("app");
        fs::create_dir_all(exe.join("data")).unwrap();
        let r = resolve(&exe, &t.path().join("cfg/location.json"), &[]);
        assert_eq!(r.dir, Some(exe.join("data")));
        assert!(r.portable);
    }

    #[test]
    fn recorded_location_with_db_is_used() {
        let t = tempfile::tempdir().unwrap();
        let data = t.path().join("D/G-routine/data");
        touch_db(&data);
        let loc = t.path().join("cfg/location.json");
        write_location(&loc, &data).unwrap();
        let r = resolve(&t.path().join("app"), &loc, &[]);
        assert_eq!(r.dir, Some(data));
        assert!(!r.portable);
    }

    #[test]
    fn falls_back_to_candidates_and_rewrites_location() {
        let t = tempfile::tempdir().unwrap();
        let empty = t.path().join("E/G-routine/data");
        let found = t.path().join("D/G-routine/data");
        touch_db(&found);
        let loc = t.path().join("cfg/location.json"); // 복원 프로그램이 지워서 없음
        let r = resolve(&t.path().join("app"), &loc, &[empty, found.clone()]);
        assert_eq!(r.dir, Some(found.clone()));
        assert_eq!(read_location(&loc), Some(found));
    }

    #[test]
    fn needs_setup_reports_previous_missing_dir() {
        let t = tempfile::tempdir().unwrap();
        let loc = t.path().join("cfg/location.json");
        let gone = t.path().join("Z/G-routine/data");
        write_location(&loc, &gone).unwrap();
        let r = resolve(&t.path().join("app"), &loc, &[]);
        assert_eq!(r.dir, None);
        assert_eq!(r.previous, Some(gone));
    }
}
