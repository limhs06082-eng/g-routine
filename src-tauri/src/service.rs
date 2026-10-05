use chrono::{NaiveDate, NaiveDateTime};
use rusqlite::Connection;

use crate::domain::day::{business_day, fmt_day, fmt_ts, is_weekend, parse_day};
use crate::db::{day_items, routines, settings};
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType, Routine, RoutineInput, Settings, TodayView};

pub fn today(c: &Connection, now: NaiveDateTime) -> AppResult<NaiveDate> {
    let s = settings::load(c)?;
    Ok(business_day(now, s.day_start_hour))
}

pub fn resync_today(c: &Connection, now: NaiveDateTime) -> AppResult<()> {
    let s = settings::load(c)?;
    day_items::sync_day(c, business_day(now, s.day_start_hour), s.hide_weekends)
}

pub fn get_today(c: &Connection, now: NaiveDateTime) -> AppResult<TodayView> {
    let s = settings::load(c)?;
    let day = business_day(now, s.day_start_hour);
    day_items::sync_day(c, day, s.hide_weekends)?;
    let (done, pending): (Vec<DayItem>, Vec<DayItem>) = day_items::items_for_day(c, &fmt_day(day))?
        .into_iter()
        .partition(|i| i.completed_at.is_some());
    Ok(TodayView { day: fmt_day(day), weekend_hidden: s.hide_weekends && is_weekend(day), pending, done })
}

pub fn set_done(c: &Connection, item_id: i64, done: bool, now: NaiveDateTime) -> AppResult<()> {
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
    if key == "hide_weekends" || key == "day_start_hour" {
        resync_today(c, now)?;
    }
    settings::load(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::test_util::{at, input_daily, input_once, input_weekdays};

    fn titles(items: &[DayItem]) -> Vec<&str> {
        items.iter().map(|i| i.title.as_str()).collect()
    }

    const MON: &str = "2026-10-05 09:00";
    const TUE: &str = "2026-10-06 09:00";
    const FRI: &str = "2026-10-09 09:00";
    const SAT: &str = "2026-10-10 09:00";

    #[test]
    fn today_lists_daily_and_matching_weekday_routines() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        create_routine(&c, input_weekdays("주간학습안내", 16), at(MON)).unwrap();

        let mon = get_today(&c, at(MON)).unwrap();
        assert_eq!(mon.day, "2026-10-05");
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
        let v = get_today(&c, at("2026-10-06 01:30")).unwrap();
        assert_eq!(v.day, "2026-10-05");
    }

    #[test]
    fn complete_and_undo() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(MON)).unwrap();
        let item = get_today(&c, at(MON)).unwrap().pending[0].clone();

        set_done(&c, item.id, true, at("2026-10-05 08:47")).unwrap();
        let v = get_today(&c, at(MON)).unwrap();
        assert!(v.pending.is_empty());
        assert_eq!(v.done[0].completed_at.as_deref(), Some("2026-10-05T08:47:00"));

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
        assert_eq!(titles(&history_day(&c, "2026-10-05").unwrap()), vec!["누가기록 작성", "공문 확인"]);
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
        let mon_history = history_day(&c, "2026-10-05").unwrap();
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
        assert_eq!(titles(&history_day(&c, "2026-10-05").unwrap()), vec!["출결 확인"]);
        assert!(list_routines(&c, at(TUE)).unwrap().is_empty());
    }

    #[test]
    fn weekend_hides_recurring_but_keeps_once() {
        let c = open_in_memory().unwrap();
        create_routine(&c, input_daily("출결 확인"), at(SAT)).unwrap();
        create_routine(&c, input_once("주말 정리", "2026-10-10"), at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert!(v.weekend_hidden);
        assert_eq!(titles(&v.pending), vec!["주말 정리"]);

        set_setting(&c, "hide_weekends", "false", at(SAT)).unwrap();
        let v = get_today(&c, at(SAT)).unwrap();
        assert!(!v.weekend_hidden);
        assert_eq!(titles(&v.pending), vec!["출결 확인", "주말 정리"]);
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
                DaySummary { day: "2026-10-05".into(), total: 2, completed: 1 },
                DaySummary { day: "2026-10-06".into(), total: 2, completed: 0 },
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
}
