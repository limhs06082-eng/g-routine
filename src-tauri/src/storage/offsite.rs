//! 두 번째 백업: 데이터 폴더(보통 D드라이브)와 다른 곳(OneDrive · USB)에 주 1회 JSON 백업을 둔다.
//! PC나 D드라이브가 고장 나도, 학년이 바뀌어 PC를 옮겨도 '불러오기'로 되살릴 수 있다.
//! 고른 폴더 안에 'G-routine 백업' 폴더를 만들고 최근 8개만 남긴다.
//!
//! 파일 쓰기는 DB 잠금 밖에서 한다(빠진 USB · 끊긴 네트워크 드라이브에서 오래 걸려도 위젯이 멈추지 않게).
//! 임시 파일에 다 쓴 뒤 이름을 바꿔, 쓰는 도중 전원이 꺼져도 예전 백업이 망가지지 않는다.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;

use crate::db::settings;
use crate::domain::day::{fmt_day, parse_day};
use crate::error::{AppError, AppResult};
use crate::storage::export::{self, BackupFile};

pub const FOLDER: &str = "G-routine 백업";
const KEEP: usize = 8;
const EVERY_DAYS: i64 = 7;
const PREFIX: &str = "g-routine-";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup2Info {
    /// 고른 폴더 (그 안의 'G-routine 백업' 폴더에 쓴다)
    pub dir: Option<String>,
    /// 마지막으로 백업한 날
    pub last: Option<String>,
    /// 아직 정하지 않았을 때 먼저 권할 OneDrive 폴더
    pub suggested: Option<String>,
    /// 위젯의 권유 카드를 닫았는지 (폴더를 한 번 정했어도 다시 묻지 않는다)
    pub dismissed: bool,
    /// 고른 폴더가 두 번째 백업으로 알맞지 않을 때의 안내 (C드라이브, 기록과 같은 드라이브)
    pub warning: Option<String>,
    /// 마지막 자동 백업이 실패했으면 그 안내 (USB를 빼 둔 때 등)
    pub error: Option<String>,
}

/// 이 PC의 OneDrive 폴더들 (개인 · 학교 계정)
fn onedrive_dirs() -> Vec<PathBuf> {
    ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .collect()
}

/// 먼저 권할 OneDrive 폴더
pub fn onedrive_dir() -> Option<PathBuf> {
    onedrive_dirs().into_iter().next()
}

/// "D:" 같은 드라이브 이름 (대문자)
fn drive_of(p: &Path) -> Option<String> {
    let s = p.to_string_lossy();
    let b = s.as_bytes();
    (b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':').then(|| s[..2].to_ascii_uppercase())
}

/// 두 번째 백업 폴더로 알맞지 않으면 그 까닭
pub fn folder_warning(dir: &Path, data_dir: Option<&Path>, system_drive: &str, onedrive: &[PathBuf]) -> Option<String> {
    let drive = drive_of(dir)?;
    if data_dir.and_then(drive_of).as_deref() == Some(drive.as_str()) {
        return Some("기록이 있는 드라이브와 같아서, 그 드라이브가 고장 나면 함께 잃어요. USB나 OneDrive를 골라 주세요".into());
    }
    let in_onedrive = onedrive.iter().any(|o| dir.starts_with(o));
    if drive.eq_ignore_ascii_case(system_drive) && !in_onedrive {
        return Some("C드라이브는 복원 프로그램이 다시 켤 때 지울 수 있어요. OneDrive나 USB를 골라 주세요".into());
    }
    None
}

pub fn info(c: &Connection, suggested: Option<PathBuf>, data_dir: Option<&Path>) -> AppResult<Backup2Info> {
    let dir = settings::get(c, settings::BACKUP2_DIR)?;
    let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
    Ok(Backup2Info {
        suggested: if dir.is_none() { suggested.map(|p| p.display().to_string()) } else { None },
        warning: dir.as_deref().and_then(|d| folder_warning(Path::new(d), data_dir, &system_drive, &onedrive_dirs())),
        dir,
        last: settings::get(c, settings::BACKUP2_LAST)?,
        dismissed: settings::get(c, settings::BACKUP2_DISMISSED)?.is_some(),
        error: settings::get(c, settings::BACKUP2_ERROR)?,
    })
}

pub fn is_due(last: Option<NaiveDate>, today: NaiveDate) -> bool {
    last.is_none_or(|l| today - l >= Duration::days(EVERY_DAYS) || l > today)
}

/// 백업할 때가 되었으면 (폴더, 백업 내용)을 준비한다. DB 잠금 안에서 부르고, 쓰기는 `write`로 잠금 밖에서 한다.
pub fn plan(c: &Connection, today: NaiveDate, now_ts: &str) -> AppResult<Option<(PathBuf, BackupFile)>> {
    let Some(base) = settings::get(c, settings::BACKUP2_DIR)? else { return Ok(None) };
    let last = settings::get(c, settings::BACKUP2_LAST)?.as_deref().and_then(parse_day);
    if !is_due(last, today) {
        return Ok(None);
    }
    Ok(Some((PathBuf::from(base), export::export(c, now_ts)?)))
}

/// 백업 파일을 쓴다 (DB 잠금 밖에서). 고른 폴더가 없거나(USB를 뺐을 때 등) 쓸 수 없으면 오류.
pub fn write(base: &Path, today: NaiveDate, backup: &BackupFile) -> AppResult<PathBuf> {
    if !base.is_dir() {
        return Err(AppError::invalid("백업 폴더를 찾지 못했어요. USB를 꽂았는지, 폴더가 있는지 확인해 주세요"));
    }
    let dir = base.join(FOLDER);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{PREFIX}{}.json", fmt_day(today)));
    let tmp = dir.join(format!("{PREFIX}{}.json.tmp", fmt_day(today)));
    export::write_file(&tmp, backup)?;
    fs::rename(&tmp, &path)?;
    prune(&dir);
    Ok(path)
}

/// 결과를 적는다 (DB 잠금 안에서): 잘 됐으면 마지막 날, 아니면 화면에 보일 오류
pub fn mark(c: &Connection, today: NaiveDate, result: &AppResult<PathBuf>) -> AppResult<()> {
    match result {
        Ok(_) => {
            settings::set(c, settings::BACKUP2_LAST, &fmt_day(today))?;
            c.execute("DELETE FROM settings WHERE key = ?1", [settings::BACKUP2_ERROR])?;
        }
        Err(e) => settings::set(c, settings::BACKUP2_ERROR, &format!("{} 백업 실패: {e}", fmt_day(today)))?,
    }
    Ok(())
}

/// 최근 KEEP개만 남긴다. 고친 시각 순으로 판단한다 (PC 시계를 되돌려 이름 날짜가 꼬여도 방금 만든 것을 지우지 않게).
fn prune(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut files: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIX)) && p.extension().is_some_and(|e| e == "json")
        })
        .filter_map(|p| Some((fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort();
    let extra = files.len().saturating_sub(KEEP);
    for (_, f) in files.into_iter().take(extra) {
        let _ = fs::remove_file(f);
    }
}

/// 테스트에서 쓰는 한 번에 하기 (잠금을 나누지 않는다)
#[cfg(test)]
pub fn run_if_due(c: &Connection, today: NaiveDate, now_ts: &str) -> AppResult<Option<PathBuf>> {
    let Some((base, backup)) = plan(c, today, now_ts)? else { return Ok(None) };
    let result = write(&base, today, &backup);
    mark(c, today, &result)?;
    result.map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::service;
    use crate::test_util::{at, date, input_daily};

    fn choose(c: &Connection, dir: &Path, today: NaiveDate) -> AppResult<()> {
        let backup = export::export(c, "t")?;
        let result = write(dir, today, &backup);
        mark(c, today, &result)?;
        result?;
        settings::set(c, settings::BACKUP2_DIR, &dir.display().to_string())
    }

    #[test]
    fn backs_up_weekly_into_its_own_folder_and_keeps_eight() {
        let c = open_in_memory().unwrap();
        service::create_routine(&c, input_daily("출결 확인"), at("2026-10-12 09:00")).unwrap();
        let t = tempfile::tempdir().unwrap();

        assert_eq!(run_if_due(&c, date("2026-10-12"), "t").unwrap(), None, "폴더를 정하기 전에는 하지 않는다");
        choose(&c, t.path(), date("2026-10-12")).unwrap();
        let folder = t.path().join(FOLDER);
        assert!(folder.join("g-routine-2026-10-12.json").exists());
        // 일주일이 안 지났으면 건너뛴다
        assert_eq!(run_if_due(&c, date("2026-10-18"), "t").unwrap(), None);
        assert!(run_if_due(&c, date("2026-10-19"), "t").unwrap().is_some());

        for week in 0..10 {
            let backup = export::export(&c, "t").unwrap();
            write(t.path(), date("2026-11-01") + Duration::days(7 * week), &backup).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(15)); // 고친 시각이 서로 다르게
        }
        let names: Vec<String> =
            fs::read_dir(&folder).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().to_string()).collect();
        assert_eq!(names.len(), KEEP);
        assert!(names.iter().all(|n| !n.ends_with(".tmp")), "임시 파일이 남지 않는다");
        // 만든 파일은 '불러오기'로 그대로 읽을 수 있다
        let newest = folder.join("g-routine-2027-01-03.json");
        assert_eq!(export::read_file(&newest).unwrap().routines[0].title, "출결 확인");

        // 시계를 되돌려 이름이 가장 이른 파일이 생겨도 방금 만든 것은 남는다
        let backup = export::export(&c, "t").unwrap();
        let back = write(t.path(), date("2026-01-05"), &backup).unwrap();
        assert!(back.exists());
    }

    #[test]
    fn a_missing_folder_is_reported_and_shown() {
        let c = open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        choose(&c, t.path(), date("2026-10-12")).unwrap();
        // USB를 뺀 것처럼 폴더를 바꿔 둔다
        settings::set(&c, settings::BACKUP2_DIR, &t.path().join("빠진 USB").display().to_string()).unwrap();
        assert!(run_if_due(&c, date("2026-10-19"), "t").is_err());
        let i = info(&c, None, None).unwrap();
        assert!(i.error.as_deref().is_some_and(|e| e.contains("2026-10-19") && e.contains("백업 폴더를 찾지 못했어요")));
        assert_eq!(i.last.as_deref(), Some("2026-10-12"), "마지막으로 잘 된 날은 그대로");
        // 다시 잘 되면 오류 안내가 사라진다
        settings::set(&c, settings::BACKUP2_DIR, &t.path().display().to_string()).unwrap();
        run_if_due(&c, date("2026-10-19"), "t").unwrap();
        assert_eq!(info(&c, None, None).unwrap().error, None);
    }

    #[test]
    fn folders_on_the_system_or_data_drive_get_a_warning() {
        let onedrive = vec![PathBuf::from(r"C:\Users\t\OneDrive")];
        let data = Some(Path::new(r"D:\G-routine\data"));
        assert!(folder_warning(Path::new(r"D:\백업"), data, "C:", &onedrive).is_some_and(|w| w.contains("같아서")));
        assert!(folder_warning(Path::new(r"C:\Users\t\Desktop"), data, "C:", &onedrive).is_some_and(|w| w.contains("C드라이브")));
        assert_eq!(folder_warning(Path::new(r"C:\Users\t\OneDrive\문서"), data, "C:", &onedrive), None);
        assert_eq!(folder_warning(Path::new(r"E:\"), data, "C:", &onedrive), None, "USB");
        assert_eq!(folder_warning(Path::new(r"\\server\share"), data, "C:", &onedrive), None);
    }

    #[test]
    fn due_after_a_week_or_when_the_clock_went_back() {
        assert!(is_due(None, date("2026-10-12")));
        assert!(!is_due(Some(date("2026-10-12")), date("2026-10-18")));
        assert!(is_due(Some(date("2026-10-12")), date("2026-10-19")));
        assert!(is_due(Some(date("2026-12-01")), date("2026-10-19")), "시계를 되돌린 경우");
    }
}
