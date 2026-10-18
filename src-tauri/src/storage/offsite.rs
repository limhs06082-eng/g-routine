//! 두 번째 백업: 데이터 폴더(보통 D드라이브)와 다른 곳(OneDrive · USB)에 주 1회 JSON 백업을 둔다.
//! PC나 D드라이브가 고장 나도, 학년이 바뀌어 PC를 옮겨도 '불러오기'로 되살릴 수 있다.
//! 고른 폴더 안에 'G-routine 백업' 폴더를 만들고 최근 8개만 남긴다.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDate};
use rusqlite::Connection;
use serde::Serialize;

use crate::db::settings;
use crate::domain::day::{fmt_day, parse_day};
use crate::error::{AppError, AppResult};
use crate::storage::export;

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
    /// 위젯의 권유 카드를 '나중에'로 닫았는지
    pub dismissed: bool,
}

/// 이 PC의 OneDrive 폴더 (개인 · 학교 계정 어느 쪽이든 있는 것)
pub fn onedrive_dir() -> Option<PathBuf> {
    ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .find(|p| p.is_dir())
}

pub fn info(c: &Connection, suggested: Option<PathBuf>) -> AppResult<Backup2Info> {
    let dir = settings::get(c, settings::BACKUP2_DIR)?;
    Ok(Backup2Info {
        suggested: if dir.is_none() { suggested.map(|p| p.display().to_string()) } else { None },
        dir,
        last: settings::get(c, settings::BACKUP2_LAST)?,
        dismissed: settings::get(c, settings::BACKUP2_DISMISSED)?.is_some(),
    })
}

pub fn is_due(last: Option<NaiveDate>, today: NaiveDate) -> bool {
    last.is_none_or(|l| today - l >= Duration::days(EVERY_DAYS) || l > today)
}

/// 지금 백업한다. 고른 폴더가 없거나(USB를 뺐을 때 등) 쓸 수 없으면 오류.
pub fn write_backup(c: &Connection, base: &Path, today: NaiveDate, now_ts: &str) -> AppResult<PathBuf> {
    if !base.is_dir() {
        return Err(AppError::invalid("백업 폴더를 찾지 못했어요. USB를 꽂았는지, 폴더가 있는지 확인해 주세요"));
    }
    let dir = base.join(FOLDER);
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{PREFIX}{}.json", fmt_day(today)));
    export::write_file(&path, &export::export(c, now_ts)?)?;
    prune(&dir);
    settings::set(c, settings::BACKUP2_LAST, &fmt_day(today))?;
    Ok(path)
}

/// 최근 KEEP개만 남긴다 (이름이 날짜라 이름순이 날짜순)
fn prune(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(PREFIX)) && p.extension().is_some_and(|e| e == "json")
        })
        .collect();
    files.sort();
    let extra = files.len().saturating_sub(KEEP);
    for f in files.into_iter().take(extra) {
        let _ = fs::remove_file(f);
    }
}

/// 백업할 때가 되었으면 백업한다 (백그라운드 작업). 폴더를 정하지 않았으면 아무것도 하지 않는다.
pub fn run_if_due(c: &Connection, today: NaiveDate, now_ts: &str) -> AppResult<Option<PathBuf>> {
    let Some(base) = settings::get(c, settings::BACKUP2_DIR)? else { return Ok(None) };
    let last = settings::get(c, settings::BACKUP2_LAST)?.as_deref().and_then(parse_day);
    if !is_due(last, today) {
        return Ok(None);
    }
    write_backup(c, Path::new(&base), today, now_ts).map(Some)
}

/// 백업 폴더를 정한다(곧바로 한 번 백업해 쓸 수 있는지 확인한다). None이면 끈다.
pub fn set_dir(c: &Connection, dir: Option<&Path>, today: NaiveDate, now_ts: &str) -> AppResult<()> {
    match dir {
        Some(d) => {
            write_backup(c, d, today, now_ts)?;
            settings::set(c, settings::BACKUP2_DIR, &d.display().to_string())
        }
        None => {
            c.execute(
                "DELETE FROM settings WHERE key IN (?1, ?2)",
                rusqlite::params![settings::BACKUP2_DIR, settings::BACKUP2_LAST],
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::service;
    use crate::test_util::{at, date, input_daily};

    #[test]
    fn backs_up_weekly_into_its_own_folder_and_keeps_eight() {
        let c = open_in_memory().unwrap();
        service::create_routine(&c, input_daily("출결 확인"), at("2026-10-12 09:00")).unwrap();
        let t = tempfile::tempdir().unwrap();

        assert_eq!(run_if_due(&c, date("2026-10-12"), "t").unwrap(), None, "폴더를 정하기 전에는 하지 않는다");
        set_dir(&c, Some(t.path()), date("2026-10-12"), "t").unwrap();
        let folder = t.path().join(FOLDER);
        assert!(folder.join("g-routine-2026-10-12.json").exists());
        // 일주일이 안 지났으면 건너뛴다
        assert_eq!(run_if_due(&c, date("2026-10-18"), "t").unwrap(), None);
        assert!(run_if_due(&c, date("2026-10-19"), "t").unwrap().is_some());

        for week in 0..10 {
            write_backup(&c, t.path(), date("2026-11-01") + Duration::days(7 * week), "t").unwrap();
        }
        let count = fs::read_dir(&folder).unwrap().count();
        assert_eq!(count, KEEP);
        // 만든 파일은 '불러오기'로 그대로 읽을 수 있다
        let newest = folder.join("g-routine-2027-01-03.json");
        assert_eq!(export::read_file(&newest).unwrap().routines[0].title, "출결 확인");
    }

    #[test]
    fn a_missing_folder_is_reported_and_turning_off_clears_it() {
        let c = open_in_memory().unwrap();
        let t = tempfile::tempdir().unwrap();
        let gone = t.path().join("빠진 USB");
        assert!(set_dir(&c, Some(&gone), date("2026-10-12"), "t").is_err());
        assert_eq!(info(&c, None).unwrap().dir, None);

        set_dir(&c, Some(t.path()), date("2026-10-12"), "t").unwrap();
        let i = info(&c, Some(PathBuf::from("C:\\OneDrive"))).unwrap();
        assert_eq!((i.last.as_deref(), i.suggested), (Some("2026-10-12"), None), "이미 정했으면 권하지 않는다");
        set_dir(&c, None, date("2026-10-13"), "t").unwrap();
        let i = info(&c, Some(PathBuf::from("C:\\OneDrive"))).unwrap();
        assert_eq!((i.dir, i.last, i.suggested.as_deref()), (None, None, Some("C:\\OneDrive")));
    }

    #[test]
    fn due_after_a_week_or_when_the_clock_went_back() {
        assert!(is_due(None, date("2026-10-12")));
        assert!(!is_due(Some(date("2026-10-12")), date("2026-10-18")));
        assert!(is_due(Some(date("2026-10-12")), date("2026-10-19")));
        assert!(is_due(Some(date("2026-12-01")), date("2026-10-19")), "시계를 되돌린 경우");
    }
}
