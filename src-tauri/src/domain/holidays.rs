//! 대한민국 관공서 공휴일 (대체공휴일 포함).
//! 출처: 우주항공청 월력요항(2026, 2027) — 2027년부터 노동절·제헌절이 공휴일로 반영됨.
//! 새 해의 월력요항이 발표되면 이 표에 추가하고 릴리스한다 (자동 업데이트로 배포됨).

use chrono::NaiveDate;

use crate::domain::day::fmt_day;

const HOLIDAYS: &[(&str, &str)] = &[
    // 2026
    ("2026-01-01", "신정"),
    ("2026-02-16", "설날 연휴"),
    ("2026-02-17", "설날"),
    ("2026-02-18", "설날 연휴"),
    ("2026-03-01", "3·1절"),
    ("2026-03-02", "대체공휴일(3·1절)"),
    ("2026-05-01", "노동절"),
    ("2026-05-05", "어린이날"),
    ("2026-05-24", "부처님 오신 날"),
    ("2026-05-25", "대체공휴일(부처님 오신 날)"),
    ("2026-06-06", "현충일"),
    ("2026-08-15", "광복절"),
    ("2026-08-17", "대체공휴일(광복절)"),
    ("2026-09-24", "추석 연휴"),
    ("2026-09-25", "추석"),
    ("2026-09-26", "추석 연휴"),
    ("2026-10-03", "개천절"),
    ("2026-10-05", "대체공휴일(개천절)"),
    ("2026-10-09", "한글날"),
    ("2026-12-25", "성탄절"),
    // 2027
    ("2027-01-01", "신정"),
    ("2027-02-06", "설날 연휴"),
    ("2027-02-07", "설날"),
    ("2027-02-08", "설날 연휴"),
    ("2027-02-09", "대체공휴일(설날)"),
    ("2027-03-01", "3·1절"),
    ("2027-05-01", "노동절"),
    ("2027-05-03", "대체공휴일(노동절)"),
    ("2027-05-05", "어린이날"),
    ("2027-05-13", "부처님 오신 날"),
    ("2027-06-06", "현충일"),
    ("2027-07-17", "제헌절"),
    ("2027-07-19", "대체공휴일(제헌절)"),
    ("2027-08-15", "광복절"),
    ("2027-08-16", "대체공휴일(광복절)"),
    ("2027-09-14", "추석 연휴"),
    ("2027-09-15", "추석"),
    ("2027-09-16", "추석 연휴"),
    ("2027-10-03", "개천절"),
    ("2027-10-04", "대체공휴일(개천절)"),
    ("2027-10-09", "한글날"),
    ("2027-10-11", "대체공휴일(한글날)"),
    ("2027-12-25", "성탄절"),
    ("2027-12-27", "대체공휴일(성탄절)"),
];

pub fn holiday_name(day: NaiveDate) -> Option<&'static str> {
    let key = fmt_day(day);
    HOLIDAYS.iter().find(|(d, _)| *d == key).map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::day::parse_day;
    use crate::test_util::date;
    use chrono::{Datelike, Weekday};

    /// 표에 들어 있는 마지막 해 (새 해를 추가하면 함께 올린다)
    const KNOWN_UNTIL_YEAR: i32 = 2027;

    #[test]
    fn known_holidays_have_names() {
        assert_eq!(holiday_name(date("2026-10-09")), Some("한글날"));
        assert_eq!(holiday_name(date("2026-10-05")), Some("대체공휴일(개천절)"));
        assert_eq!(holiday_name(date("2027-07-19")), Some("대체공휴일(제헌절)"));
        assert_eq!(holiday_name(date("2026-10-06")), None);
    }

    #[test]
    fn table_is_well_formed_sorted_and_within_known_years() {
        let days: Vec<NaiveDate> = HOLIDAYS.iter().map(|(d, _)| parse_day(d).expect("valid date")).collect();
        assert!(days.windows(2).all(|w| w[0] < w[1]), "dates must be unique and sorted");
        assert!(days.iter().all(|d| d.year() <= KNOWN_UNTIL_YEAR));
        // 대체공휴일은 모두 평일이어야 한다
        for (d, name) in HOLIDAYS {
            if name.starts_with("대체공휴일") {
                let wd = parse_day(d).unwrap().weekday();
                assert!(!matches!(wd, Weekday::Sat | Weekday::Sun), "{d} {name} falls on a weekend");
            }
        }
    }

    #[test]
    fn year_totals_match_the_official_calendar() {
        // 월력요항 발표 수치: 일요일 포함 공휴일 2026년 70일, 2027년 72일
        for (year, official) in [(2026, 70), (2027, 72)] {
            let sundays = (1..=366)
                .filter_map(|o| NaiveDate::from_yo_opt(year, o))
                .filter(|d| d.weekday() == Weekday::Sun)
                .count();
            let extra = HOLIDAYS
                .iter()
                .map(|(d, _)| parse_day(d).unwrap())
                .filter(|d| d.year() == year && d.weekday() != Weekday::Sun)
                .count();
            assert_eq!(sundays + extra, official, "{year}");
        }
    }
}
