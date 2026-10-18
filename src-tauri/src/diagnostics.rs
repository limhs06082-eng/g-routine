//! '문제 보고용 정보 저장': 동료 선생님 PC에서 생긴 문제를 알아볼 수 있게 상태와 최근 로그를 글 파일 하나로 모은다.
//! 루틴 이름 · 바로가기 주소 같은 개인 내용은 넣지 않는다 (개수와 상태만).

use rusqlite::Connection;

use crate::db::{integrity_ok, settings};
use crate::domain::holidays;
use crate::error::AppResult;
use crate::state::AppStatus;

/// DB에서 개인 내용 없이 셀 수 있는 것들
pub struct DbFacts {
    pub schema_version: i64,
    pub routines: i64,
    pub archived_routines: i64,
    pub recorded_days: i64,
    pub last_day: Option<String>,
    pub integrity_ok: bool,
}

pub fn db_facts(c: &Connection) -> AppResult<DbFacts> {
    let count = |sql: &str| -> AppResult<i64> { Ok(c.query_row(sql, [], |r| r.get(0))?) };
    Ok(DbFacts {
        schema_version: count("PRAGMA user_version")?,
        routines: count("SELECT COUNT(*) FROM routines WHERE archived_at IS NULL")?,
        archived_routines: count("SELECT COUNT(*) FROM routines WHERE archived_at IS NOT NULL")?,
        recorded_days: count("SELECT COUNT(DISTINCT day) FROM day_items")?,
        last_day: c.query_row("SELECT MAX(day) FROM day_items", [], |r| r.get(0))?,
        integrity_ok: integrity_ok(c)?,
    })
}

pub struct ReportInput<'a> {
    pub version: &'a str,
    pub created_at: &'a str,
    pub status: &'a AppStatus,
    /// 설정 (DB를 열지 못했으면 None)
    pub settings: Option<String>,
    pub db: Option<DbFacts>,
    pub log_lines: Vec<String>,
}

pub fn report(input: &ReportInput) -> String {
    let s = input.status;
    let mut out = String::new();
    out.push_str("G-routine 문제 보고용 정보\n");
    out.push_str("(루틴 이름 같은 개인 내용은 들어 있지 않아요. 이 파일을 만든 분께 보내 주세요.)\n\n");
    out.push_str(&format!("만든 때: {}\n", input.created_at));
    out.push_str(&format!("버전: {}\n", input.version));
    out.push_str(&format!("운영체제: {} {}\n", std::env::consts::OS, std::env::consts::ARCH));
    out.push_str("\n[상태]\n");
    out.push_str(&format!("준비됨: {}  포터블: {}  손상: {}  열기 실패: {}\n", s.ready, s.portable, s.corrupt, s.open_failed));
    out.push_str(&format!("데이터 폴더: {}\n", s.data_dir.as_deref().unwrap_or("(없음)")));
    out.push_str(&format!("예전 폴더를 못 찾음: {}\n", s.previous_dir.as_deref().unwrap_or("아니요")));
    out.push_str(&format!("단축키 등록: {}  자동 시작 실패: {}\n", s.shortcut, s.autostart_failed));
    out.push_str(&format!("받아 둔 새 버전: {}\n", s.pending_update.as_deref().unwrap_or("없음")));
    out.push_str(&format!("공휴일 표 마지막 해: {:?}\n", holidays::last_known_year()));
    out.push_str("\n[설정]\n");
    out.push_str(input.settings.as_deref().unwrap_or("(읽지 못함)"));
    out.push('\n');
    out.push_str("\n[데이터]\n");
    match &input.db {
        Some(d) => out.push_str(&format!(
            "구조 버전: {}  루틴: {}개 (보관 {}개)  기록한 날: {}일 (마지막 {})  무결성: {}\n",
            d.schema_version,
            d.routines,
            d.archived_routines,
            d.recorded_days,
            d.last_day.as_deref().unwrap_or("-"),
            if d.integrity_ok { "정상" } else { "이상 있음" }
        )),
        None => out.push_str("(데이터 파일을 열지 못함)\n"),
    }
    out.push_str(&format!("\n[최근 로그 {}줄]\n", input.log_lines.len()));
    for line in &input.log_lines {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 설정을 한 줄씩 (방학 날짜처럼 개인 내용이 아닌 것만 들어 있다)
pub fn settings_text(c: &Connection) -> AppResult<String> {
    Ok(format!("{:?}", settings::load(c)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;
    use crate::service;
    use crate::test_util::{at, input_daily};

    #[test]
    fn report_has_counts_and_logs_but_no_routine_titles() {
        let c = open_in_memory().unwrap();
        service::create_routine(&c, input_daily("비밀스러운 루틴 이름"), at("2026-10-12 09:00")).unwrap();
        service::get_today(&c, at("2026-10-12 09:00")).unwrap();
        let status = AppStatus { ready: true, data_dir: Some("D:\\G-routine\\data".into()), ..Default::default() };
        let text = report(&ReportInput {
            version: "0.4.0",
            created_at: "2026-10-18 22:00",
            status: &status,
            settings: Some(settings_text(&c).unwrap()),
            db: Some(db_facts(&c).unwrap()),
            log_lines: vec!["2026-10-18 21:30:00 INFO  [lib] 시작".into()],
        });
        assert!(text.contains("버전: 0.4.0"));
        assert!(text.contains("루틴: 1개"));
        assert!(text.contains("기록한 날: 1일 (마지막 2026-10-12)"));
        assert!(text.contains("무결성: 정상"));
        assert!(text.contains("[lib] 시작"));
        assert!(!text.contains("비밀스러운"));
    }
}
