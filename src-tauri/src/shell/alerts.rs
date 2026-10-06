//! 마감 시각 알림: 30초마다 오늘 할 일을 살펴, 마감이 지난 항목을 Windows 알림으로 하루에 한 번씩 알린다.
//! 새로 마감이 지난 항목이 생기면 data-changed를 보내 위젯의 마감 칩도 붉게 바뀌게 한다.

use std::collections::HashSet;
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::db::settings;
use crate::model::DayItem;
use crate::state::AppState;
use crate::{now, service, DATA_CHANGED};

const INTERVAL: Duration = Duration::from_secs(30);

/// 이미 알린 항목을 업무일별로 기억한다 (앱을 다시 켜면 한 번 더 알릴 수 있다).
#[derive(Default)]
pub struct AlertTracker {
    day: String,
    sent: HashSet<i64>,
}

impl AlertTracker {
    /// 아직 알리지 않은 항목만 돌려주고 알린 것으로 기록한다. 업무일이 바뀌면 기록을 비운다.
    pub fn take_new(&mut self, day: &str, overdue: Vec<DayItem>) -> Vec<DayItem> {
        if self.day != day {
            self.day = day.to_string();
            self.sent.clear();
        }
        overdue.into_iter().filter(|i| self.sent.insert(i.id)).collect()
    }
}

pub fn alert_text(item: &DayItem) -> String {
    match item.due_time.as_deref() {
        Some(due) => format!("‘{}’ 마감 시각({due})이 지났어요.", item.title),
        None => format!("‘{}’ 마감 시각이 지났어요.", item.title),
    }
}

pub fn spawn_due_alerts(app: AppHandle) {
    std::thread::spawn(move || {
        let mut tracker = AlertTracker::default();
        loop {
            std::thread::sleep(INTERVAL);
            let state = app.state::<AppState>();
            // 한 번 읽은 시각으로 날짜와 마감 여부를 함께 판단한다 (자정 경계에서 어긋나지 않게)
            let at = now();
            let checked = state.with_conn(|c| {
                let s = settings::load(c)?;
                let day = crate::domain::day::fmt_day(service::today(c, at)?);
                Ok((s.due_alerts, day, service::overdue_items(c, at)?))
            });
            let Ok((alerts_on, day, overdue)) = checked else { continue };
            let fresh = tracker.take_new(&day, overdue);
            if fresh.is_empty() {
                continue;
            }
            if alerts_on {
                for item in &fresh {
                    let _ = app.notification().builder().title("G-routine").body(alert_text(item)).show();
                }
            }
            let _ = app.emit(DATA_CHANGED, ());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RepeatType;

    fn item(id: i64, title: &str, due: Option<&str>) -> DayItem {
        DayItem {
            id,
            day: "2026-10-05".into(),
            routine_id: id,
            title: title.into(),
            sort_order: id,
            completed_at: None,
            repeat_type: RepeatType::Daily,
            due_time: due.map(String::from),
            has_link: false,
            overdue: true,
        }
    }

    fn ids(items: Vec<DayItem>) -> Vec<i64> {
        items.into_iter().map(|i| i.id).collect()
    }

    #[test]
    fn each_item_is_alerted_once_per_business_day() {
        let mut t = AlertTracker::default();
        assert_eq!(ids(t.take_new("2026-10-05", vec![item(1, "A", Some("09:00"))])), vec![1]);
        assert_eq!(ids(t.take_new("2026-10-05", vec![item(1, "A", Some("09:00")), item(2, "B", Some("10:00"))])), vec![2]);
        assert!(t.take_new("2026-10-05", vec![item(1, "A", Some("09:00"))]).is_empty());
        // 다음 업무일에는 다시 알린다
        assert_eq!(ids(t.take_new("2026-10-06", vec![item(1, "A", Some("09:00"))])), vec![1]);
    }

    #[test]
    fn alert_text_names_the_routine_and_its_due_time() {
        assert_eq!(alert_text(&item(1, "출결 확인", Some("09:00"))), "‘출결 확인’ 마감 시각(09:00)이 지났어요.");
        assert_eq!(alert_text(&item(1, "출결 확인", None)), "‘출결 확인’ 마감 시각이 지났어요.");
    }
}
