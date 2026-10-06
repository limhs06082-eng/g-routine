//! 쉬는 날 판단: 공휴일 → 방학 → 주말 순으로 이유를 정한다.
//! 쉬는 날에는 반복 루틴(매일/요일)을 띄우지 않고, 그 날짜로 직접 적은 하루만 할 일만 보인다.

use chrono::NaiveDate;
use serde::Serialize;

use crate::domain::day::is_weekend;
use crate::domain::holidays::holiday_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RestKind {
    Weekend,
    Holiday,
    Vacation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rest {
    pub kind: RestKind,
    /// 화면에 보일 이름 (예: "추석", "방학", "주말")
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RestRules {
    pub hide_weekends: bool,
    pub hide_holidays: bool,
    /// 방학 · 쉬는 기간 (시작일, 끝나는 날) — 둘 다 포함
    pub vacation: Option<(NaiveDate, NaiveDate)>,
}

pub fn rest_day(day: NaiveDate, rules: &RestRules) -> Option<Rest> {
    if rules.hide_holidays {
        if let Some(name) = holiday_name(day) {
            return Some(Rest { kind: RestKind::Holiday, name: name.to_string() });
        }
    }
    if let Some((start, end)) = rules.vacation {
        if start <= day && day <= end {
            return Some(Rest { kind: RestKind::Vacation, name: "방학".to_string() });
        }
    }
    if rules.hide_weekends && is_weekend(day) {
        return Some(Rest { kind: RestKind::Weekend, name: "주말".to_string() });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::date;

    fn rules(weekends: bool, holidays: bool, vacation: Option<(&str, &str)>) -> RestRules {
        RestRules { hide_weekends: weekends, hide_holidays: holidays, vacation: vacation.map(|(s, e)| (date(s), date(e))) }
    }

    #[test]
    fn holidays_come_first_then_vacation_then_weekend() {
        let all = rules(true, true, Some(("2026-10-01", "2026-10-31")));
        assert_eq!(rest_day(date("2026-10-09"), &all).map(|r| r.kind), Some(RestKind::Holiday));
        assert_eq!(rest_day(date("2026-10-09"), &all).unwrap().name, "한글날");
        assert_eq!(rest_day(date("2026-10-06"), &all).map(|r| r.kind), Some(RestKind::Vacation));
        let no_vacation = rules(true, true, None);
        assert_eq!(rest_day(date("2026-10-10"), &no_vacation).map(|r| r.kind), Some(RestKind::Weekend));
        assert_eq!(rest_day(date("2026-10-06"), &no_vacation), None);
    }

    #[test]
    fn switches_turn_each_reason_off() {
        let none = rules(false, false, None);
        assert_eq!(rest_day(date("2026-10-09"), &none), None); // 한글날이지만 공휴일 숨김 끔
        assert_eq!(rest_day(date("2026-10-10"), &none), None); // 토요일이지만 주말 숨김 끔
    }

    #[test]
    fn vacation_range_includes_both_ends() {
        let r = rules(false, false, Some(("2026-12-24", "2027-02-28")));
        assert!(rest_day(date("2026-12-24"), &r).is_some());
        assert!(rest_day(date("2027-02-28"), &r).is_some());
        assert!(rest_day(date("2026-12-23"), &r).is_none());
        assert!(rest_day(date("2027-03-01"), &r).is_none());
    }
}
