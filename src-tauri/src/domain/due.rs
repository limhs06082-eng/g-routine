//! 마감 시각 판단. 마감 시각은 업무일 기준이다.
//! 하루 시작 시각(예: 오전 4시)보다 이른 마감(예: 01:00)은 그 업무일의 다음 날 새벽을 뜻한다.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use crate::model::DayItem;

/// 업무일 `day`의 마감 시각 `due`가 실제로 가리키는 순간
pub fn due_moment(day: NaiveDate, due: NaiveTime, day_start_hour: u32) -> NaiveDateTime {
    let date = if due.hour() < day_start_hour { day + Duration::days(1) } else { day };
    date.and_time(due)
}

/// 아직 끝내지 않았고 마감 시각이 지난 항목
pub fn overdue<'a>(items: &'a [DayItem], day: NaiveDate, now: NaiveDateTime, day_start_hour: u32) -> Vec<&'a DayItem> {
    items
        .iter()
        .filter(|i| i.completed_at.is_none())
        .filter(|i| {
            i.due_time
                .as_deref()
                .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M").ok())
                .is_some_and(|due| due_moment(day, due, day_start_hour) <= now)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RepeatType;
    use crate::test_util::{at, date};

    fn item(id: i64, due: Option<&str>, done: bool) -> DayItem {
        DayItem {
            id,
            day: "2026-10-05".into(),
            routine_id: id,
            title: format!("루틴 {id}"),
            sort_order: id,
            completed_at: done.then(|| "2026-10-05T08:00:00".into()),
            repeat_type: RepeatType::Daily,
            due_time: due.map(String::from),
            has_link: false,
            overdue: false,
        }
    }

    fn t(s: &str) -> NaiveTime {
        NaiveTime::parse_from_str(s, "%H:%M").unwrap()
    }

    #[test]
    fn due_moment_rolls_early_morning_times_to_the_next_calendar_day() {
        assert_eq!(due_moment(date("2026-10-05"), t("09:00"), 4), at("2026-10-05 09:00"));
        assert_eq!(due_moment(date("2026-10-05"), t("01:00"), 4), at("2026-10-06 01:00"));
        assert_eq!(due_moment(date("2026-10-05"), t("04:00"), 4), at("2026-10-05 04:00"));
        assert_eq!(due_moment(date("2026-10-05"), t("00:30"), 0), at("2026-10-05 00:30"));
    }

    #[test]
    fn only_pending_items_past_their_due_time_are_overdue() {
        let items = vec![
            item(1, Some("09:00"), false), // 지남
            item(2, Some("09:00"), true),  // 끝냄
            item(3, Some("15:00"), false), // 아직
            item(4, None, false),          // 마감 없음
            item(5, Some("9시"), false),   // 형식 오류는 무시
            item(6, Some("01:00"), false), // 다음 날 새벽 1시 마감 → 아직
        ];
        let ids: Vec<i64> = overdue(&items, date("2026-10-05"), at("2026-10-05 09:00"), 4).iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![1]);
        let ids: Vec<i64> = overdue(&items, date("2026-10-05"), at("2026-10-06 01:30"), 4).iter().map(|i| i.id).collect();
        assert_eq!(ids, vec![1, 3, 6]);
    }
}
