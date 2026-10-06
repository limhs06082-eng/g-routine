use chrono::NaiveDateTime;
use rusqlite::Connection;

use crate::domain::day::fmt_ts;
use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::{RepeatType, RoutineInput, Slot};

fn item(title: &str, repeat_type: RepeatType, weekdays: u8, slot: Slot, link: Option<&str>) -> RoutineInput {
    RoutineInput {
        title: title.into(),
        repeat_type,
        weekdays,
        once_date: None,
        due_time: None,
        link: link.map(String::from),
        slot: Some(slot),
    }
}

pub fn seeds(template: &str) -> AppResult<Vec<RoutineInput>> {
    use RepeatType::{Daily, Weekdays};
    use Slot::{After, Class, Morning};
    match template {
        "homeroom" => Ok(vec![
            item("출결 확인", Daily, 0, Morning, Some("https://www.neis.go.kr")),
            item("수업 준비", Daily, 0, Morning, None),
            item("누가기록 작성", Daily, 0, After, None),
            item("공문 확인", Daily, 0, After, None),
            item("알림장 작성", Daily, 0, After, None),
            item("주간학습안내 배부", Weekdays, 16, After, None),
        ]),
        "subject" => Ok(vec![
            item("수업 준비", Daily, 0, Morning, None),
            item("공문 확인", Daily, 0, After, None),
            item("수업 기록 · 진도 체크", Daily, 0, Class, None),
            item("수행평가 기록", Daily, 0, Class, None),
        ]),
        "empty" => Ok(vec![]),
        _ => Err(AppError::invalid("알 수 없는 템플릿이에요")),
    }
}

pub fn apply(c: &Connection, template: &str, now: NaiveDateTime) -> AppResult<()> {
    for input in seeds(template)? {
        routines::insert(c, &routines::validate(input)?, &fmt_ts(now))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::test_util::at;

    #[test]
    fn templates_seed_expected_counts() {
        assert_eq!(seeds("homeroom").unwrap().len(), 6);
        assert_eq!(seeds("subject").unwrap().len(), 4);
        assert!(seeds("empty").unwrap().is_empty());
        assert!(seeds("unknown").is_err());
    }

    #[test]
    fn apply_inserts_valid_routines() {
        let c = open_in_memory().unwrap();
        apply(&c, "homeroom", at("2026-10-05 08:00")).unwrap();
        let list = routines::list_unarchived(&c).unwrap();
        assert_eq!(list[0].title, "출결 확인");
        assert_eq!(list[0].link.as_deref(), Some("https://www.neis.go.kr"));
        assert_eq!(list[5].weekdays, 16);
        assert_eq!(list[0].slot, Some(Slot::Morning));
        assert_eq!(list[5].slot, Some(Slot::After));
    }
}
