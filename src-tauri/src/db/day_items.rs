use std::collections::HashSet;

use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::day::fmt_day;
use crate::domain::rules::scheduled;
use crate::db::routines;
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType};

const ITEM_SELECT: &str = "SELECT d.id, d.day, d.routine_id, d.title_snapshot, d.sort_order, d.completed_at,
        r.repeat_type, r.due_time, r.link
   FROM day_items d JOIN routines r ON r.id = d.routine_id";

fn item_from_row(r: &Row) -> rusqlite::Result<DayItem> {
    let repeat: String = r.get(6)?;
    let link: Option<String> = r.get(8)?;
    Ok(DayItem {
        id: r.get(0)?,
        day: r.get(1)?,
        routine_id: r.get(2)?,
        title: r.get(3)?,
        sort_order: r.get(4)?,
        completed_at: r.get(5)?,
        repeat_type: RepeatType::parse(&repeat).unwrap_or(RepeatType::Daily),
        due_time: r.get(7)?,
        has_link: link.map(|l| !l.is_empty()).unwrap_or(false),
    })
}

/// 그 업무일의 할 일 스냅샷을 루틴 규칙에 맞춘다. 오늘 업무일에만 호출할 것.
/// - 새로 해당되는 루틴은 추가
/// - 더 이상 해당하지 않는 '미완료' 항목은 제거 (완료 항목은 보존)
/// - 미완료 항목의 이름과 모든 항목의 순서를 루틴과 맞춤
pub fn sync_day(c: &Connection, day: NaiveDate, hide_weekends: bool) -> AppResult<()> {
    let all = routines::list_unarchived(c)?;
    let sched = scheduled(&all, day, hide_weekends);
    let d = fmt_day(day);
    let tx = c.unchecked_transaction()?;
    for r in &sched {
        tx.execute(
            "INSERT OR IGNORE INTO day_items (day, routine_id, title_snapshot, sort_order) VALUES (?1, ?2, ?3, ?4)",
            params![d, r.id, r.title, r.sort_order],
        )?;
        tx.execute(
            "UPDATE day_items SET sort_order = ?3 WHERE day = ?1 AND routine_id = ?2",
            params![d, r.id, r.sort_order],
        )?;
        tx.execute(
            "UPDATE day_items SET title_snapshot = ?3 WHERE day = ?1 AND routine_id = ?2 AND completed_at IS NULL",
            params![d, r.id, r.title],
        )?;
    }
    let keep: HashSet<i64> = sched.iter().map(|r| r.id).collect();
    let stale: Vec<i64> = {
        let mut st = tx.prepare("SELECT id, routine_id FROM day_items WHERE day = ?1 AND completed_at IS NULL")?;
        let rows = st
            .query_map([&d], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows.into_iter().filter(|(_, rid)| !keep.contains(rid)).map(|(id, _)| id).collect()
    };
    for id in stale {
        tx.execute("DELETE FROM day_items WHERE id = ?1", [id])?;
    }
    tx.commit()?;
    Ok(())
}

pub fn items_for_day(c: &Connection, day: &str) -> AppResult<Vec<DayItem>> {
    let mut st = c.prepare(&format!("{ITEM_SELECT} WHERE d.day = ?1 ORDER BY d.sort_order, d.id"))?;
    let rows = st.query_map([day], item_from_row)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn set_done(c: &Connection, item_id: i64, done: bool, now: &str) -> AppResult<()> {
    let n = if done {
        c.execute(
            "UPDATE day_items SET completed_at = COALESCE(completed_at, ?2) WHERE id = ?1",
            params![item_id, now],
        )?
    } else {
        c.execute("UPDATE day_items SET completed_at = NULL WHERE id = ?1", [item_id])?
    };
    if n == 0 {
        return Err(AppError::invalid("항목을 찾을 수 없어요"));
    }
    Ok(())
}

pub fn month_summary(c: &Connection, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    let prefix = format!("{year:04}-{month:02}-%");
    let mut st = c.prepare(
        "SELECT day, COUNT(*), COUNT(completed_at) FROM day_items WHERE day LIKE ?1 GROUP BY day ORDER BY day",
    )?;
    let rows = st
        .query_map([prefix], |r| Ok(DaySummary { day: r.get(0)?, total: r.get(1)?, completed: r.get(2)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn item_day(c: &Connection, item_id: i64) -> AppResult<String> {
    c.query_row("SELECT day FROM day_items WHERE id = ?1", [item_id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::invalid("항목을 찾을 수 없어요"))
}
