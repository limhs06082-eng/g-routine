use serde::{Deserialize, Serialize};

use crate::domain::rest::Rest;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatType {
    Daily,
    Weekdays,
    Once,
}

impl RepeatType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepeatType::Daily => "daily",
            RepeatType::Weekdays => "weekdays",
            RepeatType::Once => "once",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "daily" => Some(RepeatType::Daily),
            "weekdays" => Some(RepeatType::Weekdays),
            "once" => Some(RepeatType::Once),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Routine {
    pub id: i64,
    pub title: String,
    pub repeat_type: RepeatType,
    pub weekdays: u8,
    pub once_date: Option<String>,
    pub due_time: Option<String>,
    pub link: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
    pub archived_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutineInput {
    pub title: String,
    pub repeat_type: RepeatType,
    pub weekdays: u8,
    pub once_date: Option<String>,
    pub due_time: Option<String>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DayItem {
    pub id: i64,
    pub day: String,
    pub routine_id: i64,
    pub title: String,
    pub sort_order: i64,
    pub completed_at: Option<String>,
    pub repeat_type: RepeatType,
    pub due_time: Option<String>,
    pub has_link: bool,
    /// 오늘 목록에서만 채운다: 끝내지 않았는데 마감 시각이 지났는지
    #[serde(default)]
    pub overdue: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TodayView {
    pub day: String,
    /// 오늘이 쉬는 날(공휴일·방학·주말)이면 그 이유. 쉬는 날에는 반복 루틴이 숨겨진다.
    pub rest: Option<Rest>,
    pub pending: Vec<DayItem>,
    pub done: Vec<DayItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaySummary {
    pub day: String,
    pub total: i64,
    pub completed: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub always_on_top: bool,
    pub autostart: bool,
    pub hide_weekends: bool,
    pub day_start_hour: u32,
    pub theme: String,
    /// 마감 시각이 지난 할 일을 Windows 알림으로 알려 줄지
    pub due_alerts: bool,
    /// 공휴일(대체공휴일 포함)에는 반복 루틴을 숨길지
    pub hide_holidays: bool,
    /// 위젯을 알약 모양(✓ 3/7)으로 작게 보여 줄지
    pub mini_mode: bool,
    /// 방학 · 쉬는 기간 (YYYY-MM-DD, 둘 다 있거나 둘 다 없다)
    pub vacation_start: Option<String>,
    pub vacation_end: Option<String>,
}
