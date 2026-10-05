use chrono::NaiveDate;

use crate::domain::day::{is_weekend, parse_day, weekday_bit};
use crate::model::{RepeatType, Routine};

pub fn applies(r: &Routine, day: NaiveDate) -> bool {
    if r.archived_at.is_some() {
        return false;
    }
    match r.repeat_type {
        RepeatType::Daily => true,
        RepeatType::Weekdays => r.weekdays & weekday_bit(day) != 0,
        RepeatType::Once => r.once_date.as_deref().and_then(parse_day) == Some(day),
    }
}

/// 그날 해야 할 루틴 (정렬: sort_order, id).
/// 주말 숨김이 켜져 있으면 주말에는 반복 루틴(daily/weekdays)을 빼고 once만 남긴다.
pub fn scheduled(routines: &[Routine], day: NaiveDate, hide_weekends: bool) -> Vec<&Routine> {
    let weekend_off = hide_weekends && is_weekend(day);
    let mut list: Vec<&Routine> = routines
        .iter()
        .filter(|r| applies(r, day))
        .filter(|r| !(weekend_off && r.repeat_type != RepeatType::Once))
        .collect();
    list.sort_by_key(|r| (r.sort_order, r.id));
    list
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RepeatType;
    use crate::test_util::{date, routine};

    #[test]
    fn daily_applies_every_day_unless_archived() {
        let mut r = routine(1, "출결 확인", RepeatType::Daily, 0, None);
        assert!(applies(&r, date("2026-10-05")));
        r.archived_at = Some("2026-10-05T10:00:00".into());
        assert!(!applies(&r, date("2026-10-05")));
    }

    #[test]
    fn weekdays_match_selected_bits() {
        let r = routine(1, "주간학습안내", RepeatType::Weekdays, 16, None);
        assert!(applies(&r, date("2026-10-09")));
        assert!(!applies(&r, date("2026-10-05")));
    }

    #[test]
    fn once_matches_only_its_date() {
        let r = routine(1, "가정통신문 회수", RepeatType::Once, 0, Some("2026-10-05"));
        assert!(applies(&r, date("2026-10-05")));
        assert!(!applies(&r, date("2026-10-06")));
    }

    #[test]
    fn scheduled_sorts_by_sort_order_and_hides_recurring_on_weekends() {
        let mut a = routine(1, "A", RepeatType::Daily, 0, None);
        a.sort_order = 5;
        let b = routine(2, "B", RepeatType::Daily, 0, None);
        let once = routine(3, "주말 할 일", RepeatType::Once, 0, Some("2026-10-10"));
        let all = vec![a, b, once];

        let weekday: Vec<&str> = scheduled(&all, date("2026-10-05"), true).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(weekday, vec!["B", "A"]);

        let saturday: Vec<&str> = scheduled(&all, date("2026-10-10"), true).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday, vec!["주말 할 일"]);

        let saturday_shown: Vec<&str> = scheduled(&all, date("2026-10-10"), false).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday_shown, vec!["B", "주말 할 일", "A"]);
    }
}
