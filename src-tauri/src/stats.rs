//! 기록 통계와 CSV 내보내기 (관리 창 '완료 기록' 탭)
//! - 기간 동안의 날짜별 달성, 루틴별 달성률(자주 놓치는 일), 연속 달성 일수
//! - 기간의 기록을 엑셀에서 열 수 있는 CSV로

use chrono::NaiveDate;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::day_items;
use crate::domain::day::{fmt_day, parse_day};
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, RepeatType, Slot};

/// 한 번에 볼 수 있는 가장 긴 기간 (한 학년 정도)
const MAX_DAYS: i64 = 400;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineStat {
    pub routine_id: i64,
    /// 그 기간의 마지막 기록에 적힌 이름
    pub title: String,
    pub total: i64,
    pub completed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub from: String,
    pub to: String,
    /// 기록이 있는 날만, 날짜순
    pub days: Vec<DaySummary>,
    pub total: i64,
    pub completed: i64,
    /// 달성률이 낮은 루틴부터 (자주 놓치는 일)
    pub routines: Vec<RoutineStat>,
    /// 오늘까지 모두 마친 날이 이어진 수 (기록이 없는 쉬는 날은 건너뛴다. 아직 끝나지 않은 오늘은 세지 않고 끊지도 않는다)
    pub streak: i64,
}

pub fn parse_range(from: &str, to: &str) -> AppResult<(NaiveDate, NaiveDate)> {
    let (Some(f), Some(t)) = (parse_day(from), parse_day(to)) else {
        return Err(AppError::invalid("기간 날짜 형식이 올바르지 않아요"));
    };
    if f > t {
        return Err(AppError::invalid("기간의 시작일이 끝나는 날보다 늦어요"));
    }
    if (t - f).num_days() >= MAX_DAYS {
        return Err(AppError::invalid("한 번에 볼 수 있는 기간은 400일까지예요"));
    }
    Ok((f, t))
}

/// 최근 날부터 거꾸로 모두 마친 날을 센다. days는 날짜순(오래된 것부터)이어야 한다.
pub fn streak(days: &[DaySummary], today: &str) -> i64 {
    let mut count = 0;
    for d in days.iter().rev() {
        if d.day.as_str() > today || d.total == 0 {
            continue;
        }
        let all_done = d.completed >= d.total;
        if d.day == today && !all_done {
            continue; // 오늘은 아직 하는 중
        }
        if !all_done {
            break;
        }
        count += 1;
    }
    count
}

pub fn history_stats(c: &Connection, from: &str, to: &str, today: NaiveDate) -> AppResult<Stats> {
    let (f, t) = parse_range(from, to)?;
    let (from, to) = (fmt_day(f), fmt_day(t));
    let days = day_items::range_summary(c, &from, &to)?;
    let total = days.iter().map(|d| d.total).sum();
    let completed = days.iter().map(|d| d.completed).sum();
    let mut routines = day_items::routine_summary(c, &from, &to)?;
    routines.sort_by(|a, b| {
        let ra = a.completed as f64 / a.total.max(1) as f64;
        let rb = b.completed as f64 / b.total.max(1) as f64;
        ra.total_cmp(&rb).then(b.total.cmp(&a.total))
    });
    let today = fmt_day(today);
    let streak = streak(&day_items::range_summary(c, "0000-01-01", &today)?, &today);
    Ok(Stats { from, to, days, total, completed, routines, streak })
}

fn repeat_name(t: RepeatType) -> &'static str {
    match t {
        RepeatType::Daily => "매일",
        RepeatType::Weekdays => "요일 지정",
        RepeatType::Once => "하루만",
        RepeatType::Monthly => "매월",
    }
}

fn slot_name(s: Option<Slot>) -> &'static str {
    match s {
        Some(Slot::Morning) => "조회 전",
        Some(Slot::Class) => "수업 중",
        Some(Slot::After) => "방과 후",
        None => "",
    }
}

/// 엑셀에서 쉼표 · 따옴표 · 줄바꿈이 든 칸을 올바르게 읽도록 감싼다.
/// =, +, -, @로 시작하는 칸은 수식으로 실행되지 않게 앞에 '를 붙인다.
fn cell(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.to_string() };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

pub fn to_csv(items: &[DayItem]) -> String {
    const WEEKDAYS: [&str; 7] = ["월", "화", "수", "목", "금", "토", "일"];
    let mut out = String::from("날짜,요일,루틴,반복,시간대,마감,완료 시각,상태\r\n");
    for i in items {
        use chrono::Datelike;
        let weekday = parse_day(&i.day).map(|d| WEEKDAYS[d.weekday().num_days_from_monday() as usize]).unwrap_or("");
        let done_at = i.completed_at.as_deref().map(|t| t.replace('T', " ")).unwrap_or_default();
        let row = [
            i.day.clone(),
            weekday.to_string(),
            i.title.clone(),
            repeat_name(i.repeat_type).to_string(),
            slot_name(i.slot).to_string(),
            i.due_time.clone().unwrap_or_default(),
            done_at,
            if i.completed_at.is_some() { "완료" } else { "미완료" }.to_string(),
        ];
        out.push_str(&row.iter().map(|c| cell(c)).collect::<Vec<_>>().join(","));
        out.push_str("\r\n");
    }
    out
}

pub fn export_csv(c: &Connection, from: &str, to: &str) -> AppResult<String> {
    let (f, t) = parse_range(from, to)?;
    let items = day_items::items_between(c, &fmt_day(f), &fmt_day(t))?;
    Ok(to_csv(&items))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::model::RoutineInput;
    use crate::service;
    use crate::test_util::{at, date, input_daily};

    fn summary(day: &str, total: i64, completed: i64) -> DaySummary {
        DaySummary { day: day.into(), total, completed }
    }

    #[test]
    fn streak_skips_rest_days_and_an_unfinished_today() {
        let days = [
            summary("2026-10-08", 3, 2),
            summary("2026-10-12", 3, 3),
            summary("2026-10-13", 2, 2),
            summary("2026-10-14", 3, 1), // 오늘: 아직 하는 중
        ];
        assert_eq!(streak(&days, "2026-10-14"), 2);
        assert_eq!(streak(&days[..2], "2026-10-12"), 1);
        assert_eq!(streak(&[summary("2026-10-12", 3, 2)], "2026-10-13"), 0);
    }

    #[test]
    fn stats_show_rates_per_routine_with_the_most_missed_first() {
        let c = open_in_memory().unwrap();
        service::create_routine(&c, input_daily("출결 확인"), at("2026-10-12 08:00")).unwrap();
        service::create_routine(&c, input_daily("공문 확인"), at("2026-10-12 08:00")).unwrap();
        for (day, done_both) in [("2026-10-12", true), ("2026-10-13", false), ("2026-10-14", true)] {
            let v = service::get_today(&c, at(&format!("{day} 09:00"))).unwrap();
            service::set_done(&c, v.pending[0].id, true, at(&format!("{day} 09:10"))).unwrap();
            if done_both {
                service::set_done(&c, v.pending[1].id, true, at(&format!("{day} 15:00"))).unwrap();
            }
        }
        let s = history_stats(&c, "2026-10-12", "2026-10-14", date("2026-10-14")).unwrap();
        assert_eq!((s.total, s.completed, s.days.len()), (6, 5, 3));
        assert_eq!(s.routines[0].title, "공문 확인", "자주 놓친 일이 먼저");
        assert_eq!((s.routines[0].total, s.routines[0].completed), (3, 2));
        assert_eq!(s.streak, 1);
        assert!(history_stats(&c, "2026-10-14", "2026-10-12", date("2026-10-14")).is_err());
    }

    #[test]
    fn csv_opens_in_excel_with_korean_columns_and_safe_cells() {
        let c = open_in_memory().unwrap();
        let input = RoutineInput { due_time: Some("09:00".into()), slot: Some(Slot::Morning), ..input_daily("출결, 확인") };
        service::create_routine(&c, input, at("2026-10-12 08:00")).unwrap();
        service::create_routine(&c, input_daily("=HYPERLINK(\"x\")"), at("2026-10-12 08:00")).unwrap();
        let v = service::get_today(&c, at("2026-10-12 09:00")).unwrap();
        service::set_done(&c, v.pending[0].id, true, at("2026-10-12 08:55")).unwrap();
        let csv = export_csv(&c, "2026-10-12", "2026-10-12").unwrap();
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "날짜,요일,루틴,반복,시간대,마감,완료 시각,상태");
        assert_eq!(lines[1], "2026-10-12,월,\"출결, 확인\",매일,조회 전,09:00,2026-10-12 08:55:00,완료");
        assert_eq!(lines[2], "2026-10-12,월,\"'=HYPERLINK(\"\"x\"\")\",매일,,,,미완료");
    }
}
