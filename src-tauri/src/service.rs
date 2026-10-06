use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use crate::domain::day::{business_day, fmt_day, fmt_ts, parse_day};
use crate::domain::due;
use crate::domain::rest::{rest_day, RestRules};
use crate::db::{day_items, routines, settings};
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType, Routine, RoutineInput, Settings, TodayView};

pub fn today(c: &Connection, now: NaiveDateTime) -> AppResult<NaiveDate> {
    let s = settings::load(c)?;
    Ok(business_day(now, s.day_start_hour))
}

/// 설정에서 쉬는 날 규칙을 만든다 (방학 기간은 두 날짜가 모두 있고 순서가 맞을 때만 쓴다)
fn rest_rules(s: &Settings) -> RestRules {
    let vacation = match (s.vacation_start.as_deref().and_then(parse_day), s.vacation_end.as_deref().and_then(parse_day)) {
        (Some(start), Some(end)) if start <= end => Some((start, end)),
        _ => None,
    };
    RestRules { hide_weekends: s.hide_weekends, hide_holidays: s.hide_holidays, vacation }
}

pub fn resync_today(c: &Connection, now: NaiveDateTime) -> AppResult<()> {
    let s = settings::load(c)?;
    let day = business_day(now, s.day_start_hour);
    day_items::sync_day(c, day, rest_day(day, &rest_rules(&s)).is_some())
}

pub fn get_today(c: &Connection, now: NaiveDateTime) -> AppResult<TodayView> {
    let s = settings::load(c)?;
    let day = business_day(now, s.day_start_hour);
    let rest = rest_day(day, &rest_rules(&s));
    day_items::sync_day(c, day, rest.is_some())?;
    let (done, mut pending): (Vec<DayItem>, Vec<DayItem>) = day_items::items_for_day(c, &fmt_day(day))?
        .into_iter()
        .partition(|i| i.completed_at.is_some());
    let late: Vec<i64> = due::overdue(&pending, day, now, s.day_start_hour).iter().map(|i| i.id).collect();
    for item in &mut pending {
        item.overdue = late.contains(&item.id);
    }
    Ok(TodayView { day: fmt_day(day), rest, pending, done })
}

/// 방학 · 쉬는 기간을 정하거나(시작일과 끝나는 날 모두) 해제한다(둘 다 없음).
pub fn set_vacation(c: &Connection, start: Option<&str>, end: Option<&str>, now: NaiveDateTime) -> AppResult<Settings> {
    let range = match (start, end) {
        (None, None) => None,
        (Some(s), Some(e)) => {
            let (Some(sd), Some(ed)) = (parse_day(s), parse_day(e)) else {
                return Err(AppError::invalid("날짜 형식이 올바르지 않아요"));
            };
            if sd > ed {
                return Err(AppError::invalid("시작일이 끝나는 날보다 늦어요. 날짜를 다시 골라 주세요"));
            }
            Some((fmt_day(sd), fmt_day(ed)))
        }
        _ => return Err(AppError::invalid("시작일과 끝나는 날을 모두 골라 주세요")),
    };
    settings::set_vacation(c, range.as_ref().map(|(s, e)| (s.as_str(), e.as_str())))?;
    resync_today(c, now)?;
    settings::load(c)
}

/// 오늘 업무일에서 끝내지 않았고 마감 시각이 지난 항목 (알림용, DB를 바꾸지 않는다)
pub fn overdue_items(c: &Connection, now: NaiveDateTime) -> AppResult<Vec<DayItem>> {
    let s = settings::load(c)?;
    let day = business_day(now, s.day_start_hour);
    let items = day_items::items_for_day(c, &fmt_day(day))?;
    Ok(due::overdue(&items, day, now, s.day_start_hour)
        .into_iter()
        .map(|i| DayItem { overdue: true, ..i.clone() })
        .collect())
}

pub fn set_done(c: &Connection, item_id: i64, done: bool, now: NaiveDateTime) -> AppResult<()> {
    let today_date = today(c, now)?;
    let item_date = day_items::item_day(c, item_id)?;
    if item_date != fmt_day(today_date) {
        return Err(AppError::invalid("날짜가 바뀌었어요. 목록을 새로 불러올게요"));
    }
    day_items::set_done(c, item_id, done, &fmt_ts(now))
}

pub fn quick_add(c: &Connection, title: &str, now: NaiveDateTime) -> AppResult<()> {
    let day = today(c, now)?;
    let input = RoutineInput {
        title: title.to_string(),
        repeat_type: RepeatType::Once,
        weekdays: 0,
        once_date: Some(fmt_day(day)),
        due_time: None,
        link: None,
    };
    create_routine(c, input, now).map(|_| ())
}

pub fn list_routines(c: &Connection, now: NaiveDateTime) -> AppResult<Vec<Routine>> {
    routines::list_for_manager(c, &fmt_day(today(c, now)?))
}

pub fn create_routine(c: &Connection, input: RoutineInput, now: NaiveDateTime) -> AppResult<i64> {
    let input = routines::validate(input)?;
    let id = routines::insert(c, &input, &fmt_ts(now))?;
    resync_today(c, now)?;
    Ok(id)
}

pub fn update_routine(c: &Connection, id: i64, input: RoutineInput, now: NaiveDateTime) -> AppResult<()> {
    let input = routines::validate(input)?;
    routines::update(c, id, &input)?;
    resync_today(c, now)
}

pub fn archive_routine(c: &Connection, id: i64, now: NaiveDateTime) -> AppResult<()> {
    routines::archive(c, id, &fmt_ts(now))?;
    resync_today(c, now)
}

pub fn reorder_routines(c: &Connection, ids: &[i64], now: NaiveDateTime) -> AppResult<()> {
    routines::reorder(c, ids)?;
    resync_today(c, now)
}

pub fn history_month(c: &Connection, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    if !(1..=12).contains(&month) {
        return Err(AppError::invalid("월 값이 올바르지 않아요"));
    }
    day_items::month_summary(c, year, month)
}

pub fn history_day(c: &Connection, day: &str) -> AppResult<Vec<DayItem>> {
    let d = parse_day(day).ok_or_else(|| AppError::invalid("날짜 형식이 올바르지 않아요"))?;
    day_items::items_for_day(c, &fmt_day(d))
}

pub fn set_setting(c: &Connection, key: &str, value: &str, now: NaiveDateTime) -> AppResult<Settings> {
    settings::apply(c, key, value)?;
    if key == "hide_weekends" || key == "hide_holidays" || key == "day_start_hour" {
        resync_today(c, now)?;
    }
    settings::load(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::domain::rest::RestKind;
    use crate::test_util::{at, input_daily, input_once, input_weekdays};

    fn titles(items: &[DayItem]) -> Vec<&str> {
        items.iter().map(|i| i.title.as_str()).collect()
    }

    const MON: &str = "2026-10-12 09:00";
    const TUE: &str = "2026-10-13 09:00";
    const FRI: &str = "2026-10-16 09:00";
    const SAT: &str = "2026-10-17 09:00";

    #[test]
    fn today_lists_daily_and_matching_weekday_routines() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        create_routine(&c, input_weekdays("주간학습안내", 16), at(MON)).unwrap();

        let mon = get_today(&c, at(MON)).unwrap();
        assert_eq!(mon.day, "2026-10-12");
        assert_eq!(titles(&mon.pending), vec!["출결 확인"]);

        let fri = get_today(&c, at(FRI)).unwrap();
        assert_eq!(titles(&fri.pending), vec!["출결 확인", "주간학습안내"]);
    }

    #[test]
    fn get_today_is_idempotent() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        get_today(&c, at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(v.pending.len(), 1);
    }

    #[test]
    fn early_morning_belongs_to_previous_business_day() {
        let c = open_in_memory().unwrap();
        let v = get_today(&c, at("2026-10-13 01:30")).unwrap();
        assert_eq!(v.day, "2026-10-12");
    }

    #[test]
    fn complete_and_undo() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();

        set_done(&c, item.id, true, at("2026-10-12 08:47")).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert!(v.pending.is_empty());
        assert_eq!(v.done[0].completed_at.as_deref(), Some("2026-10-12T08:47:00"));

        set_done(&c, item.id, false, at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&v.pending), vec!["출결 확인"]);
        assert!(v.done.is_empty());
    }

    #[test]
    fn rename_updates_only_pending_items_of_today() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("누가기록"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("공문 확인"), at(MON)).unwrap();
        let mon = get_today(&c, at(MON)).unwrap();
        let done_id = mon.pending.iter().find(|i| i.routine_id == b).unwrap().id;
        set_done(&c, done_id, true, at(MON)).unwrap();

        update_routine(&c, a, input_daily("누가기록 작성"), at(MON)).unwrap();
        update_routine(&c, b, input_daily("공문 처리"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&v.pending), vec!["누가기록 작성"]);
        assert_eq!(titles(&v.done), vec!["공문 확인"]);

        // 다음 날에는 새 이름으로 생성
        let tue = get_today(&c, at(TUE)).unwrap();
        assert_eq!(titles(&tue.pending), vec!["누가기록 작성", "공문 처리"]);
        // 월요일 기록은 그대로
        assert_eq!(titles(&history_day(&c, "2026-10-12").unwrap()), vec!["누가기록 작성", "공문 확인"]);
    }

    #[test]
    fn changing_weekdays_drops_pending_but_keeps_completed_today() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("A"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("B"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        let b_item = v.pending.iter().find(|i| i.routine_id == b).unwrap().id;
        set_done(&c, b_item, true, at(MON)).unwrap();

        update_routine(&c, a, input_weekdays("A", 2), at(MON)).unwrap(); // 화요일만
        update_routine(&c, b, input_weekdays("B", 2), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert!(v.pending.is_empty());
        assert_eq!(titles(&v.done), vec!["B"]);
    }

    #[test]
    fn once_item_expires_as_incomplete() {
        let c = open_in_memory().unwrap();
        quick_add(&c, "가정통신문 회수", at(MON)).unwrap();
        let mon = get_today(&c, at(MON)).unwrap();
        assert_eq!(titles(&mon.pending), vec!["가정통신문 회수"]);
        assert_eq!(mon.pending[0].repeat_type, RepeatType::Once);

        let tue = get_today(&c, at(TUE)).unwrap();
        assert!(tue.pending.is_empty());
        let mon_history = history_day(&c, "2026-10-12").unwrap();
        assert_eq!(mon_history.len(), 1);
        assert_eq!(mon_history[0].completed_at, None);
        assert!(list_routines(&c, at(TUE)).unwrap().is_empty());
    }

    #[test]
    fn archive_removes_pending_today_and_keeps_history() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();
        set_done(&c, item.id, true, at(MON)).unwrap();
        get_today(&c, at(TUE)).unwrap();

        archive_routine(&c, a, at(TUE)).unwrap();
        assert!(get_today(&c, at(TUE)).unwrap().pending.is_empty());
        assert_eq!(titles(&history_day(&c, "2026-10-12").unwrap()), vec!["출결 확인"]);
        assert!(list_routines(&c, at(TUE)).unwrap().is_empty());
    }

    #[test]
    fn weekend_hides_recurring_but_keeps_once() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(SAT)).unwrap();
        create_routine(&c, input_once("주말 정리", "2026-10-17"), at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert_eq!(v.rest.as_ref().map(|r| r.kind), Some(RestKind::Weekend));
        assert_eq!(titles(&v.pending), vec!["주말 정리"]);

        set_setting(&c, "hide_weekends", "false", at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert_eq!(v.rest, None);
        assert_eq!(titles(&v.pending), vec!["출결 확인", "주말 정리"]);
    }

    const HANGUL_DAY: &str = "2026-10-09 09:00"; // 금요일, 한글날

    #[test]
    fn holidays_hide_recurring_routines_unless_turned_off() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(HANGUL_DAY)).unwrap();
        create_routine(&c, input_once("학급 게시판 정리", "2026-10-09"), at(HANGUL_DAY)).unwrap();
        let v = get_today(&c, at(HANGUL_DAY)).unwrap();
        assert_eq!(v.rest.as_ref().map(|r| (r.kind, r.name.as_str())), Some((RestKind::Holiday, "한글날")));
        assert_eq!(titles(&v.pending), vec!["학급 게시판 정리"]);

        set_setting(&c, "hide_holidays", "false", at(HANGUL_DAY)).unwrap();
        let v = get_today(&c, at(HANGUL_DAY)).unwrap();
        assert_eq!(v.rest, None);
        assert_eq!(titles(&v.pending), vec!["출결 확인", "학급 게시판 정리"]);
    }

    #[test]
    fn vacation_pauses_recurring_routines_and_clears_again() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(TUE)).unwrap();
        let s = set_vacation(&c, Some("2026-10-13"), Some("2026-10-14"), at(TUE)).unwrap();
        assert_eq!((s.vacation_start.as_deref(), s.vacation_end.as_deref()), (Some("2026-10-13"), Some("2026-10-14")));
        let v = get_today(&c, at(TUE)).unwrap();
        assert_eq!(v.rest.as_ref().map(|r| r.kind), Some(RestKind::Vacation));
        assert!(v.pending.is_empty());

        let s = set_vacation(&c, None, None, at(TUE)).unwrap();
        assert_eq!((s.vacation_start, s.vacation_end), (None, None));
        assert_eq!(titles(&get_today(&c, at(TUE)).unwrap().pending), vec!["출결 확인"]);

        // 방학이 끝난 다음 날에는 다시 보인다
        set_vacation(&c, Some("2026-10-13"), Some("2026-10-14"), at(TUE)).unwrap();
        assert_eq!(titles(&get_today(&c, at("2026-10-15 09:00")).unwrap().pending), vec!["출결 확인"]);
    }

    #[test]
    fn vacation_needs_both_dates_in_order() {
        let c = open_in_memory().unwrap();
        assert!(set_vacation(&c, Some("2026-10-14"), Some("2026-10-13"), at(TUE)).is_err());
        assert!(set_vacation(&c, Some("2026-10-13"), None, at(TUE)).is_err());
        assert!(set_vacation(&c, Some("2026/10/06"), Some("2026-10-14"), at(TUE)).is_err());
        let s = settings::load(&c).unwrap();
        assert_eq!((s.vacation_start, s.vacation_end), (None, None));
    }

    #[test]
    fn reorder_changes_today_order() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("A"), at(MON)).unwrap();
        let b = create_routine(&c, input_daily("B"), at(MON)).unwrap();
        reorder_routines(&c, &[b, a], at(MON)).unwrap();
        assert_eq!(titles(&get_today(&c, at(MON)).unwrap().pending), vec!["B", "A"]);
    }

    #[test]
    fn month_summary_counts_completed() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("A"), at(MON)).unwrap();
        create_routine(&c, input_daily("B"), at(MON)).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        set_done(&c, v.pending[0].id, true, at(MON)).unwrap();
        get_today(&c, at(TUE)).unwrap();

        let m = history_month(&c, 2026, 10).unwrap();
        assert_eq!(
            m,
            vec![
                DaySummary { day: "2026-10-12".into(), total: 2, completed: 1 },
                DaySummary { day: "2026-10-13".into(), total: 2, completed: 0 },
            ]
        );
        assert!(history_month(&c, 2026, 13).is_err());
        assert!(history_day(&c, "2026/10/05").is_err());
    }

    #[test]
    fn invalid_input_is_rejected_without_side_effects() {
        let c = open_in_memory().unwrap();
        assert!(create_routine(&c, input_daily(" "), at(MON)).is_err());
        assert!(quick_add(&c, "  ", at(MON)).is_err());
        assert!(list_routines(&c, at(MON)).unwrap().is_empty());
    }

    #[test]
    fn set_done_rejects_past_business_day_pending() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("과제"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();

        // Attempt to complete MON item on TUE should fail
        let err = set_done(&c, item.id, true, at(TUE));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("날짜가 바뀌었어요"));

        // MON history should still show incomplete
        let mon_history = history_day(&c, "2026-10-12").unwrap();
        assert_eq!(mon_history[0].completed_at, None);
    }

    #[test]
    fn set_done_rejects_past_business_day_completed() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("과제"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();

        // Complete on MON
        set_done(&c, item.id, true, at(MON)).unwrap();

        // Attempt to uncomplete on TUE should fail
        let err = set_done(&c, item.id, false, at(TUE));
        assert!(err.is_err());

        // MON history should still show completed
        let mon_history = history_day(&c, "2026-10-12").unwrap();
        assert!(mon_history[0].completed_at.is_some());
    }

    #[test]
    fn set_done_rejects_nonexistent_item() {
        let c = open_in_memory().unwrap();
        let err = set_done(&c, 9999, true, at(MON));
        assert!(err.is_err());
        assert!(err.unwrap_err().to_string().contains("항목을 찾을 수 없어요"));
    }

    #[test]
    fn today_marks_pending_items_past_their_due_time() {
        let c = open_in_memory().unwrap();
        let due = RoutineInput { due_time: Some("09:00".into()), ..input_daily("출결 확인") };
        create_routine(&c, due, at(MON)).unwrap();
        create_routine(&c, input_daily("수업 준비"), at(MON)).unwrap();

        let before = get_today(&c, at("2026-10-12 08:59")).unwrap();
        assert!(before.pending.iter().all(|i| !i.overdue));

        let after = get_today(&c, at("2026-10-12 09:00")).unwrap();
        let flags: Vec<(&str, bool)> = after.pending.iter().map(|i| (i.title.as_str(), i.overdue)).collect();
        assert_eq!(flags, vec![("출결 확인", true), ("수업 준비", false)]);

        set_done(&c, after.pending[0].id, true, at("2026-10-12 09:05")).unwrap();
        let done = get_today(&c, at("2026-10-12 09:10")).unwrap();
        assert!(done.done.iter().all(|i| !i.overdue));
    }

    #[test]
    fn overdue_alerts_list_only_pending_past_due_items_of_today() {
        let c = open_in_memory().unwrap();
        create_routine(&c, RoutineInput { due_time: Some("09:00".into()), ..input_daily("출결 확인") }, at(MON)).unwrap();
        create_routine(&c, RoutineInput { due_time: Some("15:00".into()), ..input_daily("공문 확인") }, at(MON)).unwrap();
        get_today(&c, at(MON)).unwrap();
        let titles: Vec<String> = overdue_items(&c, at("2026-10-12 10:00")).unwrap().into_iter().map(|i| i.title).collect();
        assert_eq!(titles, vec!["출결 확인".to_string()]);
    }

    #[test]
    fn moving_the_day_start_later_never_rewrites_an_earlier_day() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        create_routine(&c, input_daily("공문 확인"), at(MON)).unwrap();
        get_today(&c, at(MON)).unwrap(); // 월요일 기록: 두 항목 모두 미완료
        get_today(&c, at(TUE)).unwrap(); // 화요일로 넘어감

        // 화요일 오전 9시에 하루 시작 시각을 오후 3시로 바꾸면 업무일 계산상 오늘이 월요일이 된다
        archive_routine(&c, a, at(TUE)).unwrap();
        set_setting(&c, "day_start_hour", "15", at(TUE)).unwrap();
        let v = get_today(&c, at(TUE)).unwrap();
        assert_eq!(v.day, "2026-10-12");
        // 월요일 기록은 그대로 남아야 한다 (보관한 루틴의 미완료 항목도 지워지지 않음)
        assert_eq!(titles(&history_day(&c, "2026-10-12").unwrap()), vec!["출결 확인", "공문 확인"]);
    }

    #[test]
    fn yesterday_stays_closed_even_when_today_had_nothing_to_do() {
        let c = open_in_memory().unwrap();
        let a = create_routine(&c, input_daily("출결 확인"), at("2026-10-08 09:00")).unwrap();
        create_routine(&c, input_daily("공문 확인"), at("2026-10-08 09:00")).unwrap();
        get_today(&c, at("2026-10-08 09:00")).unwrap();
        assert!(get_today(&c, at(HANGUL_DAY)).unwrap().pending.is_empty()); // 공휴일: 기록 없음

        archive_routine(&c, a, at(HANGUL_DAY)).unwrap();
        set_setting(&c, "day_start_hour", "15", at(HANGUL_DAY)).unwrap();
        assert_eq!(titles(&history_day(&c, "2026-10-08").unwrap()), vec!["출결 확인", "공문 확인"]);
    }

    #[test]
    fn a_pc_clock_set_back_by_days_still_fills_today() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        get_today(&c, at("2026-10-20 09:00")).unwrap(); // 시계가 앞서 있던 때
        assert_eq!(titles(&get_today(&c, at(TUE)).unwrap().pending), vec!["출결 확인"]);
    }

    #[test]
    fn vacation_starting_today_keeps_what_was_already_done() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(TUE)).unwrap();
        create_routine(&c, input_daily("공문 확인"), at(TUE)).unwrap();
        let v = get_today(&c, at(TUE)).unwrap();
        set_done(&c, v.pending[0].id, true, at(TUE)).unwrap();

        set_vacation(&c, Some("2026-10-13"), Some("2026-10-20"), at(TUE)).unwrap();
        let v = get_today(&c, at(TUE)).unwrap();
        assert_eq!(titles(&v.done), vec!["출결 확인"]);
        assert!(v.pending.is_empty());
    }
}
