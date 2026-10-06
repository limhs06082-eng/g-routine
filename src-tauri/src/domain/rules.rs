use chrono::NaiveDate;

use crate::domain::day::{parse_day, weekday_bit};
use crate::domain::rest::RestKind;
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

/// 쉬는 날에도 보이는 루틴인가.
/// - 공휴일 · 방학: 그 날짜로 직접 적은 하루만 할 일(once)만 보인다.
/// - 주말: 하루만 할 일과, 토 · 일을 직접 고른 요일 루틴은 보인다 ('매일' 루틴만 쉰다).
fn shows_on_rest_day(r: &Routine, rest: RestKind) -> bool {
    match r.repeat_type {
        RepeatType::Once => true,
        RepeatType::Weekdays => rest == RestKind::Weekend,
        RepeatType::Daily => false,
    }
}

/// 그날 해야 할 루틴 (정렬: sort_order, id).
/// rest는 쉬는 날의 이유 (domain::rest가 정한다). 쉬는 날에는 `shows_on_rest_day`인 루틴만 남긴다.
pub fn scheduled(routines: &[Routine], day: NaiveDate, rest: Option<RestKind>) -> Vec<&Routine> {
    let mut list: Vec<&Routine> = routines
        .iter()
        .filter(|r| applies(r, day))
        .filter(|r| rest.is_none_or(|kind| shows_on_rest_day(r, kind)))
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
    fn scheduled_sorts_by_sort_order_and_hides_recurring_on_rest_days() {
        let mut a = routine(1, "A", RepeatType::Daily, 0, None);
        a.sort_order = 5;
        let b = routine(2, "B", RepeatType::Daily, 0, None);
        let once = routine(3, "주말 할 일", RepeatType::Once, 0, Some("2026-10-10"));
        let all = vec![a, b, once];

        let weekday: Vec<&str> = scheduled(&all, date("2026-10-05"), None).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(weekday, vec!["B", "A"]);

        let saturday: Vec<&str> = scheduled(&all, date("2026-10-10"), Some(RestKind::Weekend)).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday, vec!["주말 할 일"]);

        let saturday_shown: Vec<&str> = scheduled(&all, date("2026-10-10"), None).iter().map(|r| r.title.as_str()).collect();
        assert_eq!(saturday_shown, vec!["B", "주말 할 일", "A"]);
    }

    #[test]
    fn weekend_days_picked_on_purpose_show_even_when_weekends_are_hidden() {
        let daily = routine(1, "출결 확인", RepeatType::Daily, 0, None);
        let saturday_club = routine(2, "토요 방과후 지도", RepeatType::Weekdays, 32, None);
        let friday = routine(3, "주간학습안내 배부", RepeatType::Weekdays, 16, None);
        let all = vec![daily, saturday_club, friday];
        let titles = |rest| scheduled(&all, date("2026-10-10"), rest).iter().map(|r| r.title.as_str()).collect::<Vec<_>>();

        assert_eq!(titles(Some(RestKind::Weekend)), vec!["토요 방과후 지도"]);
        // 공휴일 · 방학에는 요일 루틴도 쉰다
        assert!(titles(Some(RestKind::Holiday)).is_empty());
        assert!(titles(Some(RestKind::Vacation)).is_empty());
    }
}
