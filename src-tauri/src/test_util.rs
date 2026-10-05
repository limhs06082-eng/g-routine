use chrono::{NaiveDate, NaiveDateTime};

use crate::model::{RepeatType, Routine, RoutineInput};

pub fn at(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").expect("valid datetime")
}

pub fn date(s: &str) -> NaiveDate {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").expect("valid date")
}

pub fn input_daily(title: &str) -> RoutineInput {
    RoutineInput {
        title: title.into(),
        repeat_type: RepeatType::Daily,
        weekdays: 0,
        once_date: None,
        due_time: None,
        link: None,
    }
}

pub fn input_weekdays(title: &str, weekdays: u8) -> RoutineInput {
    RoutineInput { repeat_type: RepeatType::Weekdays, weekdays, ..input_daily(title) }
}

pub fn input_once(title: &str, day: &str) -> RoutineInput {
    RoutineInput { repeat_type: RepeatType::Once, once_date: Some(day.into()), ..input_daily(title) }
}

pub fn routine(id: i64, title: &str, repeat_type: RepeatType, weekdays: u8, once_date: Option<&str>) -> Routine {
    Routine {
        id,
        title: title.into(),
        repeat_type,
        weekdays,
        once_date: once_date.map(String::from),
        due_time: None,
        link: None,
        sort_order: id,
        created_at: "2026-10-01T09:00:00".into(),
        archived_at: None,
    }
}
