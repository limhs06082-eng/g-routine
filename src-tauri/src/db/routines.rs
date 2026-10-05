use chrono::NaiveTime;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::day::{fmt_day, parse_day};
use crate::error::{AppError, AppResult};
use crate::model::{RepeatType, Routine, RoutineInput};

const COLS: &str = "id, title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at, archived_at";
const TITLE_MAX: usize = 40;

fn from_row(r: &Row) -> rusqlite::Result<Routine> {
    let repeat: String = r.get(2)?;
    let weekdays: i64 = r.get(3)?;
    Ok(Routine {
        id: r.get(0)?,
        title: r.get(1)?,
        repeat_type: RepeatType::parse(&repeat).unwrap_or(RepeatType::Daily),
        weekdays: weekdays as u8,
        once_date: r.get(4)?,
        due_time: r.get(5)?,
        link: r.get(6)?,
        sort_order: r.get(7)?,
        created_at: r.get(8)?,
        archived_at: r.get(9)?,
    })
}

pub fn list_unarchived(c: &Connection) -> AppResult<Vec<Routine>> {
    let mut st = c.prepare(&format!(
        "SELECT {COLS} FROM routines WHERE archived_at IS NULL ORDER BY sort_order, id"
    ))?;
    let rows = st.query_map([], from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// 루틴 관리 탭용: 삭제되지 않았고, 날짜가 지난 '오늘만' 루틴이 아닌 것.
pub fn list_for_manager(c: &Connection, today: &str) -> AppResult<Vec<Routine>> {
    let mut st = c.prepare(&format!(
        "SELECT {COLS} FROM routines
         WHERE archived_at IS NULL AND NOT (repeat_type = 'once' AND once_date < ?1)
         ORDER BY sort_order, id"
    ))?;
    let rows = st.query_map([today], from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(c: &Connection, id: i64) -> AppResult<Routine> {
    c.query_row(&format!("SELECT {COLS} FROM routines WHERE id = ?1"), [id], from_row)
        .optional()?
        .ok_or_else(|| AppError::invalid("루틴을 찾을 수 없어요"))
}

pub fn insert(c: &Connection, input: &RoutineInput, now: &str) -> AppResult<i64> {
    let next: i64 = c.query_row("SELECT COALESCE(MAX(sort_order), -1) + 1 FROM routines", [], |r| r.get(0))?;
    c.execute(
        "INSERT INTO routines (title, repeat_type, weekdays, once_date, due_time, link, sort_order, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.title,
            input.repeat_type.as_str(),
            i64::from(input.weekdays),
            input.once_date,
            input.due_time,
            input.link,
            next,
            now
        ],
    )?;
    Ok(c.last_insert_rowid())
}

pub fn update(c: &Connection, id: i64, input: &RoutineInput) -> AppResult<()> {
    let n = c.execute(
        "UPDATE routines SET title = ?2, repeat_type = ?3, weekdays = ?4, once_date = ?5, due_time = ?6, link = ?7
         WHERE id = ?1 AND archived_at IS NULL",
        params![
            id,
            input.title,
            input.repeat_type.as_str(),
            i64::from(input.weekdays),
            input.once_date,
            input.due_time,
            input.link
        ],
    )?;
    if n == 0 {
        return Err(AppError::invalid("루틴을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn archive(c: &Connection, id: i64, now: &str) -> AppResult<()> {
    let n = c.execute(
        "UPDATE routines SET archived_at = ?2 WHERE id = ?1 AND archived_at IS NULL",
        params![id, now],
    )?;
    if n == 0 {
        return Err(AppError::invalid("루틴을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn reorder(c: &Connection, ids: &[i64]) -> AppResult<()> {
    let tx = c.unchecked_transaction()?;
    for (i, id) in ids.iter().enumerate() {
        tx.execute("UPDATE routines SET sort_order = ?2 WHERE id = ?1", params![id, i as i64])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn is_allowed_link(link: &str) -> bool {
    let lower = link.to_ascii_lowercase();
    for scheme in ["https://", "http://"] {
        if lower.starts_with(scheme) {
            return link.len() > scheme.len();
        }
    }
    let b = link.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/');
    drive || link.starts_with(r"\\")
}

/// 저장 전 입력 정리 + 검증. 반복 종류에 맞지 않는 필드는 비운다.
pub fn validate(input: RoutineInput) -> AppResult<RoutineInput> {
    let title = input.title.trim().to_string();
    if title.is_empty() {
        return Err(AppError::invalid("루틴 이름을 입력해 주세요"));
    }
    if title.chars().count() > TITLE_MAX {
        return Err(AppError::invalid("루틴 이름은 40자 이내로 입력해 주세요"));
    }
    let due_time = match input.due_time.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(t) => {
            if t.len() != 5 || NaiveTime::parse_from_str(t, "%H:%M").is_err() {
                return Err(AppError::invalid("마감 시각 형식이 올바르지 않아요 (예: 09:00)"));
            }
            Some(t.to_string())
        }
    };
    let link = match input.link.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(l) => {
            if !is_allowed_link(l) {
                return Err(AppError::invalid(
                    "바로가기는 https:// 주소나 C:\\ 같은 프로그램 경로만 넣을 수 있어요",
                ));
            }
            Some(l.to_string())
        }
    };
    let (weekdays, once_date) = match input.repeat_type {
        RepeatType::Daily => (0, None),
        RepeatType::Weekdays => {
            let w = input.weekdays & 0x7F;
            if w == 0 {
                return Err(AppError::invalid("요일을 하나 이상 골라 주세요"));
            }
            (w, None)
        }
        RepeatType::Once => {
            let d = input
                .once_date
                .as_deref()
                .and_then(parse_day)
                .ok_or_else(|| AppError::invalid("날짜를 골라 주세요"))?;
            (0, Some(fmt_day(d)))
        }
    };
    Ok(RoutineInput { title, repeat_type: input.repeat_type, weekdays, once_date, due_time, link })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::model::RepeatType;
    use crate::test_util::{input_daily, input_once, input_weekdays};

    const NOW: &str = "2026-10-05T09:00:00";

    #[test]
    fn insert_assigns_increasing_sort_order_and_get_returns_row() {
        let c = open_in_memory().unwrap();
        let a = insert(&c, &input_daily("출결 확인"), NOW).unwrap();
        let b = insert(&c, &input_daily("수업 준비"), NOW).unwrap();
        assert_eq!(get(&c, a).unwrap().sort_order, 0);
        assert_eq!(get(&c, b).unwrap().sort_order, 1);
        assert_eq!(get(&c, b).unwrap().title, "수업 준비");
    }

    #[test]
    fn update_archive_and_reorder() {
        let c = open_in_memory().unwrap();
        let a = insert(&c, &input_daily("A"), NOW).unwrap();
        let b = insert(&c, &input_daily("B"), NOW).unwrap();
        update(&c, a, &input_weekdays("A2", 16)).unwrap();
        let ra = get(&c, a).unwrap();
        assert_eq!((ra.title.as_str(), ra.repeat_type, ra.weekdays), ("A2", RepeatType::Weekdays, 16));

        reorder(&c, &[b, a]).unwrap();
        let titles: Vec<String> = list_unarchived(&c).unwrap().into_iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["B", "A2"]);

        archive(&c, b, NOW).unwrap();
        assert_eq!(list_unarchived(&c).unwrap().len(), 1);
        assert!(get(&c, b).unwrap().archived_at.is_some());
        assert!(update(&c, b, &input_daily("X")).is_err());
    }

    #[test]
    fn manager_list_hides_past_once_routines() {
        let c = open_in_memory().unwrap();
        insert(&c, &input_daily("매일"), NOW).unwrap();
        insert(&c, &input_once("어제 할 일", "2026-10-04"), NOW).unwrap();
        insert(&c, &input_once("오늘 할 일", "2026-10-05"), NOW).unwrap();
        let titles: Vec<String> = list_for_manager(&c, "2026-10-05").unwrap().into_iter().map(|r| r.title).collect();
        assert_eq!(titles, vec!["매일", "오늘 할 일"]);
    }

    #[test]
    fn validate_normalizes_and_rejects_bad_input() {
        let ok = validate(RoutineInput { title: "  출결 확인 ".into(), weekdays: 5, once_date: Some("2026-10-05".into()), ..input_daily("") }).unwrap();
        assert_eq!(ok.title, "출결 확인");
        assert_eq!((ok.weekdays, ok.once_date), (0, None));

        assert!(validate(input_daily("   ")).is_err());
        assert!(validate(input_daily(&"가".repeat(41))).is_err());
        assert!(validate(input_weekdays("요일", 0)).is_err());
        assert!(validate(RoutineInput { once_date: None, ..input_once("날짜", "2026-10-05") }).is_err());
        assert!(validate(RoutineInput { due_time: Some("25:00".into()), ..input_daily("시각") }).is_err());
        assert_eq!(validate(RoutineInput { due_time: Some("".into()), ..input_daily("시각") }).unwrap().due_time, None);
        assert!(validate(RoutineInput { link: Some("javascript:alert(1)".into()), ..input_daily("링크") }).is_err());
        assert_eq!(
            validate(RoutineInput { link: Some(" https://www.neis.go.kr ".into()), ..input_daily("링크") }).unwrap().link,
            Some("https://www.neis.go.kr".into())
        );
    }

    #[test]
    fn allowed_links() {
        assert!(is_allowed_link("https://www.neis.go.kr"));
        assert!(is_allowed_link("http://edufine.go.kr"));
        assert!(is_allowed_link(r"C:\Program Files\App\app.exe"));
        assert!(is_allowed_link("D:/업무/출석부.xlsx"));
        assert!(is_allowed_link(r"\\server\share\file.hwp"));
        assert!(!is_allowed_link("https://"));
        assert!(!is_allowed_link("file:///C:/x"));
        assert!(!is_allowed_link("javascript:alert(1)"));
        assert!(!is_allowed_link("notepad.exe"));
    }
}
