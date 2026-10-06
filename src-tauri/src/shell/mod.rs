pub mod alerts;
pub mod holidays;
pub mod memory;
pub mod position;
pub mod shortcut;
pub mod tray;
pub mod update;
pub mod window;

use tauri::{AppHandle, Manager};

use crate::db::settings;
use crate::model::Settings;
use crate::state::AppState;

/// 부팅 직후: 트레이, 설정 반영, 위젯 배치 · 표시, 날짜 감시 시작
pub fn startup(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let s = state.with_conn(settings::load).ok();
    let saved = state.with_conn(settings::window_pos).ok().flatten();
    tray::build(app, s.as_ref())?;
    if let Some(s) = &s {
        apply_all(app, s);
    }
    window::place_widget(app, saved)?;
    window::show_widget_quietly(app);
    window::spawn_day_watcher(app.clone());
    Ok(())
}

/// 시작 설정 · 복구 · 백업 불러오기 직후 설정을 OS에 반영
pub fn after_ready(app: &AppHandle) {
    if let Ok(s) = app.state::<AppState>().with_conn(settings::load) {
        apply_all(app, &s);
    }
    // 데이터 폴더가 시작보다 늦게 준비된 경우(처음 설정 · 다시 시도)에도 보관해 둔 공휴일 표를 쓴다
    holidays::load_cached(app);
}

fn apply_all(app: &AppHandle, s: &Settings) {
    apply_always_on_top(app, s.always_on_top);
    sync_autostart(app, s.autostart);
    tray::sync_checks(app, s);
}

pub fn apply_setting_side_effects(app: &AppHandle, key: &str, s: &Settings) {
    match key {
        "always_on_top" => apply_always_on_top(app, s.always_on_top),
        "autostart" => sync_autostart(app, s.autostart),
        _ => {}
    }
    tray::sync_checks(app, s);
}

fn apply_always_on_top(app: &AppHandle, on: bool) {
    if let Some(w) = app.get_webview_window(window::WIDGET) {
        let _ = w.set_always_on_top(on);
    }
}

/// 자동 시작 등록(HKCU Run). 복원 프로그램에 지워졌으면 실행 때마다 다시 등록된다.
/// 디버그 빌드는 개발용 exe가 등록되지 않도록 건너뛴다.
fn sync_autostart(app: &AppHandle, on: bool) {
    if cfg!(debug_assertions) {
        return;
    }
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let enabled = launcher.is_enabled().unwrap_or(false);
    if on && !enabled {
        let _ = launcher.enable();
    } else if !on && enabled {
        let _ = launcher.disable();
    }
}
