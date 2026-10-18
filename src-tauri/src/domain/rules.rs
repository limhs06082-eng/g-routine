use chrono::{Datelike, NaiveDate};

use crate::domain::day::{is_weekend, parse_day, weekday_bit};
use crate::domain::holidays::holiday_name;
use crate::domain::rest::RestKind;
use crate::model::{RepeatType, Routine};

pub fn applies(r: &Routine, day: NaiveDate) -> bool {
    if r.archived_at.is_some() || !in_period(r, day) {
        return false;
    }
    match r.repeat_type {
        RepeatType::Daily => true,
        RepeatType::Weekdays => r.weekdays & weekday_bit(day) != 0,
        RepeatType::Once => r.once_date.as_deref().and_then(parse_day) == Some(day),
        RepeatType::Monthly => r.month_day.is_some_and(|m| monthly_applies(m, day)),
    }
}

/// 기간 한정 루틴이면 그 기간 안인지 (시작일 · 끝나는 날 포함)
fn in_period(r: &Routine, day: NaiveDate) -> bool {
    let from = r.active_from.as_deref().and_then(parse_day);
    let until = r.active_until.as_deref().and_then(parse_day);
    from.is_none_or(|f| f <= day) && until.is_none_or(|u| day <= u)
}

/// 주말이나 공휴일 (매월 반복을 앞 평일로 당길 때 쓴다. 쉬는 날 숨기기 설정과는 상관없다)
fn is_off_day(d: NaiveDate) -> bool {
    is_weekend(d) || holiday_name(d).is_some()
}

fn last_day_of_month(year: i32, month: u32) -> Option<NaiveDate> {
    let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)?.pred_opt()
}

/// 그달 '매월 N일'이 실제로 보이는 날. N이 0이거나 그달에 없는 날(31일 등)이면 그달 마지막 날.
/// 그날이 주말 · 공휴일이면 바로 앞 평일로 당긴다 (마감 같은 일을 놓치지 않게. 앞달로 넘어갈 수도 있다).
pub fn monthly_target(year: i32, month: u32, month_day: u8) -> Option<NaiveDate> {
    let last = last_day_of_month(year, month)?;
    let mut d = if month_day == 0 || u32::from(month_day) > last.day() {
        last
    } else {
        NaiveDate::from_ymd_opt(year, month, u32::from(month_day))?
    };
    for _ in 0..20 {
        if !is_off_day(d) {
            return Some(d);
        }
        d = d.pred_opt()?;
    }
    Some(d)
}

fn monthly_applies(month_day: u8, day: NaiveDate) -> bool {
    let this = monthly_target(day.year(), day.month(), month_day);
    // 다음 달 1일이 주말이면 이번 달 마지막 평일로 당겨 올 수 있다
    let (ny, nm) = if day.month() == 12 { (day.year() + 1, 1) } else { (day.year(), day.month() + 1) };
    let next = monthly_target(ny, nm, month_day);
    this == Some(day) || next == Some(day)
}

/// 쉬는 날에도 보이는 루틴인가.
/// - 공휴일 · 방학: 그 날짜로 직접 적은 하루만 할 일(once)만 보인다.
/// - 주말: 하루만 할 일과, 토 · 일을 직접 고른 요일 루틴은 보인다 ('매일' 루틴만 쉰다).
fn shows_on_rest_day(r: &Routine, rest: RestKind) -> bool {
    match r.repeat_type {
        RepeatType::Once => true,
        RepeatType::Weekdays => rest == RestKind::Weekend,
        RepeatType::Daily | RepeatType::Monthly => false,
    }
}

/// 그날 해야 할 루틴 (정렬: sort_order, id).
/// rest는 쉬는 날의 이유 (domain::rest가 정한다). 쉬는 날에는 `shows_on_rest_day`인 루틴만 남긴다.
pub fn scheduled(routines: &[Routine], day: NaiveDate, rest: Option<RestKind>) -> Vec<&Routine> {
    let mut list: Vec<&Routine> =
        routines.iter().filter(|r| applies(r, day)).filter(|r| rest.is_none_or(|kind| shows_on_rest_day(r, kind))).collect();
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

    fn monthly(id: i64, day: u8) -> Routine {
        Routine { month_day: Some(day), ..routine(id, "월말 정리", RepeatType::Monthly, 0, None) }
    }

    #[test]
    fn monthly_moves_to_the_weekday_before_a_weekend_or_holiday() {
        // 2026-10-10은 토요일 → 10월 9일(금)은 한글날 → 10월 8일(목)
        assert_eq!(monthly_target(2026, 10, 10), Some(date("2026-10-08")));
        // 평일이면 그대로
        assert_eq!(monthly_target(2026, 10, 15), Some(date("2026-10-15")));
        // 0은 마지막 날: 2026-10-31(토) → 10월 30일(금) = 마지막 평일
        assert_eq!(monthly_target(2026, 10, 0), Some(date("2026-10-30")));
        // 31일이 없는 달은 마지막 날: 2026-11-30(월)
        assert_eq!(monthly_target(2026, 11, 31), Some(date("2026-11-30")));
        // 2027-05-01(토, 노동절) → 앞달 4월 30일(금)
        assert_eq!(monthly_target(2027, 5, 1), Some(date("2027-04-30")));
    }

    #[test]
    fn monthly_edges_across_the_year_february_and_chuseok() {
        // 2027-01-01(금, 신정) → 앞 평일은 2026-12-31(목): 해를 넘어 앞당긴다
        assert_eq!(monthly_target(2027, 1, 1), Some(date("2026-12-31")));
        assert!(applies(&monthly(1, 1), date("2026-12-31")));
        // 평년 2월에는 29 · 30 · 31일이 없어 마지막 날(2/28 일요일) → 2/26(금)
        for day in [29, 30, 31, 0] {
            assert_eq!(monthly_target(2027, 2, day), Some(date("2027-02-26")), "{day}");
        }
        // 추석 연휴(2026-09-24 ~ 26) 안의 25일 → 9/23(수)
        assert_eq!(monthly_target(2026, 9, 25), Some(date("2026-09-23")));
    }

    #[test]
    fn monthly_routines_show_only_on_their_day() {
        let r = monthly(1, 1);
        assert!(applies(&r, date("2027-04-30")), "5월 1일(토)의 일은 4월 30일(금)에");
        assert!(!applies(&r, date("2027-05-01")));
        assert!(applies(&r, date("2026-12-01")));
        assert!(!applies(&r, date("2026-12-02")));
        let last = monthly(2, 0);
        assert!(applies(&last, date("2026-10-30")));
        assert!(!applies(&last, date("2026-10-31")));
    }

    #[test]
    fn a_routine_with_a_period_shows_only_inside_it() {
        let r = Routine {
            active_from: Some("2026-10-12".into()),
            active_until: Some("2026-10-16".into()),
            ..routine(1, "방과후 출석", RepeatType::Daily, 0, None)
        };
        assert!(!applies(&r, date("2026-10-11")));
        assert!(applies(&r, date("2026-10-12")));
        assert!(applies(&r, date("2026-10-16")));
        assert!(!applies(&r, date("2026-10-19")));
        let open_ended = Routine { active_from: Some("2026-10-12".into()), ..routine(2, "2학기", RepeatType::Daily, 0, None) };
        assert!(applies(&open_ended, date("2027-01-04")));
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

        let saturday: Vec<&str> =
            scheduled(&all, date("2026-10-10"), Some(RestKind::Weekend)).iter().map(|r| r.title.as_str()).collect();
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
