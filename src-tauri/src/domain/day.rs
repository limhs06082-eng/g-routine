use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, Weekday};

/// 하루 시작 시각(day_start_hour) 이전이면 전날을 업무일로 본다.
pub fn business_day(now: NaiveDateTime, day_start_hour: u32) -> NaiveDate {
    (now - Duration::hours(i64::from(day_start_hour))).date()
}

/// 월=1, 화=2, 수=4, 목=8, 금=16, 토=32, 일=64
pub fn weekday_bit(day: NaiveDate) -> u8 {
    1u8 << day.weekday().num_days_from_monday()
}

pub fn is_weekend(day: NaiveDate) -> bool {
    matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

pub fn fmt_day(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}

pub fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

pub fn fmt_ts(t: NaiveDateTime) -> String {
    t.format("%Y-%m-%dT%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{at, date};

    #[test]
    fn business_day_uses_day_start_hour_boundary() {
        assert_eq!(business_day(at("2026-10-06 01:30"), 4), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-05 03:59"), 4), date("2026-10-04"));
        assert_eq!(business_day(at("2026-10-05 04:00"), 4), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-05 00:00"), 0), date("2026-10-05"));
        assert_eq!(business_day(at("2026-10-04 23:59"), 0), date("2026-10-04"));
    }

    #[test]
    fn weekday_bits_follow_monday_first_order() {
        assert_eq!(weekday_bit(date("2026-10-05")), 1); // 월
        assert_eq!(weekday_bit(date("2026-10-09")), 16); // 금
        assert_eq!(weekday_bit(date("2026-10-11")), 64); // 일
    }

    #[test]
    fn weekend_detection() {
        assert!(!is_weekend(date("2026-10-09")));
        assert!(is_weekend(date("2026-10-10")));
        assert!(is_weekend(date("2026-10-11")));
    }

    #[test]
    fn day_and_timestamp_formatting_round_trips() {
        assert_eq!(fmt_day(date("2026-10-05")), "2026-10-05");
        assert_eq!(parse_day("2026-10-05"), Some(date("2026-10-05")));
        assert_eq!(parse_day("2026-13-05"), None);
        assert_eq!(fmt_ts(at("2026-10-05 08:47")), "2026-10-05T08:47:00");
    }
}
