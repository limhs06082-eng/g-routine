//! 대한민국 관공서 공휴일 (대체공휴일 포함).
//! 표는 `data/holidays.json` 하나에 있다. 앱에 그대로 들어가고(내장 표),
//! 같은 파일을 GitHub main 브랜치에서 하루 한 번 받아 와 덮어쓴다(shell::holidays).
//! 그래서 임시공휴일이나 새 해의 월력요항은 이 파일만 고쳐 push하면 릴리스 없이 모든 PC에 반영된다.

use std::collections::BTreeMap;
#[cfg(test)]
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock, RwLock};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::domain::day::parse_day;

const BUILT_IN: &str = include_str!("../../data/holidays.json");
const NAME_MAX: usize = 30;
const PER_YEAR_MAX: usize = 60;

#[derive(Deserialize)]
struct FileEntry {
    date: String,
    name: String,
}

#[derive(Deserialize)]
struct HolidayFile {
    version: u32,
    years: BTreeMap<String, Vec<FileEntry>>,
}

/// 해마다의 공휴일. 들어 있는 해(years)는 그해 공휴일을 모두 알고 있다는 뜻이다.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HolidayTable {
    years: BTreeMap<i32, BTreeMap<NaiveDate, String>>,
}

impl HolidayTable {
    /// holidays.json을 읽는다. 형식이 하나라도 어긋나면 통째로 거절한다 (내려받은 파일이 깨졌을 때 대비).
    pub fn parse(json: &str) -> Result<Self, String> {
        let file: HolidayFile = serde_json::from_str(json).map_err(|e| format!("형식 오류: {e}"))?;
        if file.version != 1 {
            return Err(format!("모르는 버전: {}", file.version));
        }
        let mut years = BTreeMap::new();
        for (key, entries) in file.years {
            let year: i32 = key.parse().map_err(|_| format!("연도가 아님: {key}"))?;
            if !(2000..=2100).contains(&year) {
                return Err(format!("연도 범위 밖: {year}"));
            }
            if entries.is_empty() || entries.len() > PER_YEAR_MAX {
                return Err(format!("{year}년 항목 수가 이상함: {}", entries.len()));
            }
            let mut days = BTreeMap::new();
            for e in entries {
                let day = parse_day(&e.date).ok_or_else(|| format!("날짜 형식 오류: {}", e.date))?;
                let name = e.name.trim();
                if day.year() != year || name.is_empty() || name.chars().count() > NAME_MAX {
                    return Err(format!("항목 오류: {} {}", e.date, e.name));
                }
                if days.insert(day, name.to_string()).is_some() {
                    return Err(format!("같은 날짜가 두 번: {}", e.date));
                }
            }
            years.insert(year, days);
        }
        if years.is_empty() {
            return Err("공휴일이 하나도 없음".into());
        }
        Ok(HolidayTable { years })
    }

    pub fn name(&self, day: NaiveDate) -> Option<&str> {
        self.years.get(&day.year())?.get(&day).map(String::as_str)
    }

    /// 표에 들어 있는 마지막 해
    pub fn last_year(&self) -> Option<i32> {
        self.years.keys().next_back().copied()
    }

    #[cfg(test)]
    pub fn years(&self) -> BTreeSet<i32> {
        self.years.keys().copied().collect()
    }

    /// newer에 들어 있는 해는 newer의 것으로 바꾸고, 나머지 해는 그대로 둔다.
    pub fn merged(&self, newer: &HolidayTable) -> HolidayTable {
        let mut years = self.years.clone();
        years.extend(newer.years.clone());
        HolidayTable { years }
    }
}

pub fn built_in() -> HolidayTable {
    HolidayTable::parse(BUILT_IN).expect("내장 공휴일 표(data/holidays.json)가 올바라야 한다")
}

fn current() -> &'static RwLock<Arc<HolidayTable>> {
    static TABLE: OnceLock<RwLock<Arc<HolidayTable>>> = OnceLock::new();
    TABLE.get_or_init(|| RwLock::new(Arc::new(built_in())))
}

fn table() -> Arc<HolidayTable> {
    current().read().map(|t| Arc::clone(&t)).unwrap_or_else(|_| Arc::new(built_in()))
}

/// 내려받은 표를 내장 표 위에 덮어써서 앞으로의 판단에 쓴다.
pub fn use_downloaded(downloaded: &HolidayTable) {
    let next = Arc::new(built_in().merged(downloaded));
    if let Ok(mut t) = current().write() {
        *t = next;
    }
}

pub fn holiday_name(day: NaiveDate) -> Option<String> {
    table().name(day).map(String::from)
}

pub fn last_known_year() -> Option<i32> {
    table().last_year()
}

/// 공휴일 표가 오늘을 얼마나 덮고 있는지 (설정 탭 안내용)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CoverageState {
    /// 올해와 내년 걱정 없음
    Ok,
    /// 11월부터: 표가 올해까지만 있어 내년 공휴일이 아직 없음
    EndingSoon,
    /// 올해 공휴일이 표에 없음 (공휴일 숨기기가 쉬고 있음)
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Coverage {
    pub last_year: Option<i32>,
    pub state: CoverageState,
}

pub fn coverage(last_year: Option<i32>, today: NaiveDate) -> Coverage {
    let state = match last_year {
        Some(y) if y > today.year() => CoverageState::Ok,
        Some(y) if y == today.year() => {
            if today.month() >= 11 {
                CoverageState::EndingSoon
            } else {
                CoverageState::Ok
            }
        }
        _ => CoverageState::Missing,
    };
    Coverage { last_year, state }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::date;
    use chrono::Weekday;

    /// 표에 들어 있는 마지막 해 (새 해를 추가하면 함께 올린다)
    const KNOWN_UNTIL_YEAR: i32 = 2027;

    #[test]
    fn known_holidays_have_names() {
        let t = built_in();
        assert_eq!(t.name(date("2026-10-09")), Some("한글날"));
        assert_eq!(t.name(date("2026-10-05")), Some("대체공휴일(개천절)"));
        assert_eq!(t.name(date("2027-07-19")), Some("대체공휴일(제헌절)"));
        assert_eq!(t.name(date("2026-06-03")), Some("지방선거일"));
        assert_eq!(t.name(date("2026-07-17")), Some("제헌절"));
        assert_eq!(t.name(date("2026-10-06")), None);
        assert_eq!(holiday_name(date("2026-10-09")).as_deref(), Some("한글날"));
    }

    #[test]
    fn built_in_table_is_well_formed_and_within_known_years() {
        let t = built_in();
        assert_eq!(t.last_year(), Some(KNOWN_UNTIL_YEAR));
        // 대체공휴일은 모두 평일이어야 한다
        for days in t.years.values() {
            for (d, name) in days {
                if name.starts_with("대체공휴일") {
                    assert!(!matches!(d.weekday(), Weekday::Sat | Weekday::Sun), "{d} {name} falls on a weekend");
                }
            }
        }
    }

    #[test]
    fn year_totals_match_the_official_calendar() {
        // 일요일 포함 공휴일 수: 2026년 72일(월력요항 70일 + 지방선거일 + 제헌절), 2027년 72일(월력요항)
        let t = built_in();
        for (year, official) in [(2026, 72), (2027, 72)] {
            let sundays = (1..=366)
                .filter_map(|o| NaiveDate::from_yo_opt(year, o))
                .filter(|d| d.weekday() == Weekday::Sun)
                .count();
            let extra = t.years[&year].keys().filter(|d| d.weekday() != Weekday::Sun).count();
            assert_eq!(sundays + extra, official, "{year}");
        }
    }

    #[test]
    fn parse_rejects_anything_suspicious() {
        let ok = r#"{"version":1,"years":{"2028":[{"date":"2028-01-01","name":"신정"}]}}"#;
        assert!(HolidayTable::parse(ok).is_ok());
        for bad in [
            "not json",
            r#"{"version":2,"years":{"2028":[{"date":"2028-01-01","name":"신정"}]}}"#,
            r#"{"version":1,"years":{}}"#,
            r#"{"version":1,"years":{"2028":[]}}"#,
            r#"{"version":1,"years":{"abcd":[{"date":"2028-01-01","name":"신정"}]}}"#,
            r#"{"version":1,"years":{"2028":[{"date":"2029-01-01","name":"신정"}]}}"#,
            r#"{"version":1,"years":{"2028":[{"date":"2028-13-01","name":"신정"}]}}"#,
            r#"{"version":1,"years":{"2028":[{"date":"2028-01-01","name":"  "}]}}"#,
            r#"{"version":1,"years":{"2028":[{"date":"2028-01-01","name":"신정"},{"date":"2028-01-01","name":"신정"}]}}"#,
        ] {
            assert!(HolidayTable::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_downloaded_year_replaces_that_year_only() {
        // 2027년에 임시공휴일이 생긴 표를 받으면 2027년만 바뀌고 2026년은 그대로
        let downloaded = HolidayTable::parse(
            r#"{"version":1,"years":{
                "2027":[{"date":"2027-01-01","name":"신정"},{"date":"2027-03-02","name":"임시공휴일"}],
                "2028":[{"date":"2028-01-01","name":"신정"}]}}"#,
        )
        .unwrap();
        let merged = built_in().merged(&downloaded);
        assert_eq!(merged.name(date("2027-03-02")), Some("임시공휴일"));
        assert_eq!(merged.name(date("2027-10-09")), None);
        assert_eq!(merged.name(date("2026-10-09")), Some("한글날"));
        assert_eq!(merged.last_year(), Some(2028));
        assert_eq!(merged.years(), BTreeSet::from([2026, 2027, 2028]));
    }

    #[test]
    fn coverage_warns_from_november_and_when_the_year_is_missing() {
        assert_eq!(coverage(Some(2027), date("2026-10-06")).state, CoverageState::Ok);
        assert_eq!(coverage(Some(2027), date("2027-10-31")).state, CoverageState::Ok);
        assert_eq!(coverage(Some(2027), date("2027-11-01")).state, CoverageState::EndingSoon);
        assert_eq!(coverage(Some(2027), date("2028-01-03")).state, CoverageState::Missing);
        assert_eq!(coverage(None, date("2026-10-06")).state, CoverageState::Missing);
    }
}
