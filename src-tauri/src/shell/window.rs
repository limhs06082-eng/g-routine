use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::db::settings;
use crate::service;
use crate::shell::memory::set_low_memory;
use crate::shell::position::{compute_position, fit_height, Rect};
use crate::state::AppState;
use crate::{now, startup, DATA_CHANGED};

pub const WIDGET: &str = "widget";
pub const MANAGER: &str = "manager";
/// WebView2 실행 인자. GPU 프로세스를 끄면 메모리가 약 75MB 줄어든다(194MB → 119MB 실측).
/// 이 값을 지정하면 Tauri 기본 인자가 사라지므로 기본값(--disable-features=…)을 함께 적는다.
/// WebView2는 같은 데이터 폴더의 창끼리 인자가 같아야 하므로 tauri.conf.json 위젯 창과 반드시 같은 값을 쓴다.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-gpu";
const MARGIN: f64 = 12.0;
/// 미니 모드(알약 모양)도 들어가도록 낮게 둔다
const MIN_HEIGHT: f64 = 40.0;

pub fn show_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        set_low_memory(&w, false);
    }
}

/// 단축키(Ctrl+Alt+G): 보이면 숨기고, 숨겨져 있으면 띄운다.
pub fn toggle_widget(app: &AppHandle) {
    let visible = app.get_webview_window(WIDGET).and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if visible {
        hide_widget(app);
    } else {
        show_widget(app);
    }
}

/// 부팅 · 자동 시작 때: 보이게만 하고 포커스는 빼앗지 않는다.
pub fn show_widget_quietly(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.show();
        let _ = w.unminimize();
        set_low_memory(&w, false);
    }
}

/// 숨기면 트레이에만 남으므로 WebView2를 메모리 절약 모드로 바꾼다.
pub fn hide_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.hide();
        set_low_memory(&w, true);
    }
}

fn mark_programmatic_move(app: &AppHandle) {
    if let Ok(mut g) = app.state::<AppState>().programmatic_move.lock() {
        *g = Some(Instant::now());
    }
}

/// saved = (x, 아래쪽 y). 저장 위치가 화면 밖이면 주 모니터 작업 영역 우측 하단에 둔다.
/// DPI가 다른 모니터로 옮기면 Windows가 창 크기를 바꾸므로, 이동 후 크기를 다시 읽어 한 번 더 맞춘다.
pub fn place_widget(app: &AppHandle, saved: Option<(i32, i32)>) -> tauri::Result<()> {
    let Some(w) = app.get_webview_window(WIDGET) else { return Ok(()) };
    let size = w.outer_size()?;
    let (ww, wh) = (size.width as i32, size.height as i32);
    let monitors: Vec<Rect> = w
        .available_monitors()?
        .iter()
        .map(|m| Rect { x: m.position().x, y: m.position().y, w: m.size().width as i32, h: m.size().height as i32 })
        .collect();
    let Some(m) = w.primary_monitor()?.or(w.current_monitor()?) else { return Ok(()) };
    let wa = m.work_area();
    let work = Rect { x: wa.position.x, y: wa.position.y, w: wa.size.width as i32, h: wa.size.height as i32 };
    let margin = (MARGIN * m.scale_factor()).round() as i32;

    let pos = compute_position(saved, ww, wh, &monitors, work, margin);
    mark_programmatic_move(app);
    w.set_position(PhysicalPosition::new(pos.0, pos.1))?;

    let after = w.outer_size()?;
    if after != size {
        let pos = compute_position(saved, after.width as i32, after.height as i32, &monitors, work, margin);
        mark_programmatic_move(app);
        w.set_position(PhysicalPosition::new(pos.0, pos.1))?;
    }
    Ok(())
}

pub fn reset_position(app: &AppHandle) {
    let _ = app.state::<AppState>().with_conn(settings::clear_window_pos);
    let _ = place_widget(app, None);
    show_widget(app);
}

/// 내용 높이(논리 px)에 맞춰 위젯 높이를 바꾸되 아래 모서리를 고정한다.
/// 최대 높이는 위젯이 있는 모니터의 작업 영역 높이(여백 제외)다. 그보다 길 때만 화면 안에서 스크롤된다.
/// 반환값: 내용이 최대 높이를 넘었는지. true일 때만 화면 쪽에서 스크롤을 켠다.
pub fn resize_widget(app: &AppHandle, logical_height: f64) -> tauri::Result<bool> {
    let Some(w) = app.get_webview_window(WIDGET) else { return Ok(false) };
    let Some(m) = w.current_monitor()?.or(w.primary_monitor()?) else { return Ok(false) };
    let scale = w.scale_factor()?;
    let pos = w.outer_position()?;
    let old = w.outer_size()?;
    let wa = m.work_area();
    let work = Rect { x: wa.position.x, y: wa.position.y, w: wa.size.width as i32, h: wa.size.height as i32 };
    let margin = (MARGIN * scale).round() as i32;
    let wanted = (logical_height * scale).round() as i32;
    let min_h = (MIN_HEIGHT * scale).round() as i32;
    let (y, new_h) = fit_height(pos.y, old.height as i32, wanted, min_h, work, margin);
    let capped = wanted > new_h;
    if new_h == old.height as i32 && y == pos.y {
        return Ok(capped);
    }
    mark_programmatic_move(app);
    w.set_size(PhysicalSize::new(old.width, new_h as u32))?;
    w.set_position(PhysicalPosition::new(pos.x, y))?;
    Ok(capped)
}

pub fn open_manager(app: &AppHandle) -> tauri::Result<()> {
    if let Some(w) = app.get_webview_window(MANAGER) {
        w.show()?;
        w.unminimize()?;
        return w.set_focus();
    }
    WebviewWindowBuilder::new(app, MANAGER, WebviewUrl::App("index.html".into()))
        .title("G-routine 관리")
        .inner_size(760.0, 560.0)
        .min_inner_size(640.0, 480.0)
        .center()
        .disable_drag_drop_handler()
        .additional_browser_args(BROWSER_ARGS)
        .build()?;
    Ok(())
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if window.label() != WIDGET {
        return;
    }
    match event {
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            hide_widget(window.app_handle());
        }
        WindowEvent::Moved(pos) => {
            let h = window.outer_size().map(|s| s.height as i32).unwrap_or(0);
            schedule_position_save(window.app_handle(), pos.x, pos.y + h);
        }
        _ => {}
    }
}

/// 사용자가 끌어서 옮긴 위치만 저장한다 (프로그램 이동 직후 400ms는 무시, 500ms 디바운스).
fn schedule_position_save(app: &AppHandle, x: i32, bottom: i32) {
    let state = app.state::<AppState>();
    let recent = state
        .programmatic_move
        .lock()
        .ok()
        .and_then(|g| *g)
        .map(|t| t.elapsed() < Duration::from_millis(400))
        .unwrap_or(false);
    if recent {
        return;
    }
    let seq = state.move_seq.fetch_add(1, Ordering::SeqCst) + 1;
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        let state = app.state::<AppState>();
        if state.move_seq.load(Ordering::SeqCst) == seq {
            let _ = state.with_conn(|c| settings::set_window_pos(c, x, bottom));
        }
    });
}

/// 30초마다 업무일을 확인해 바뀌면 백업 후 data-changed를 보낸다 (절전 복귀 포함).
pub fn spawn_day_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last = None;
        loop {
            let state = app.state::<AppState>();
            if let Ok(day) = state.with_conn(|c| service::today(c, now())) {
                if last.is_some() && last != Some(day) {
                    if let Some(dir) = state.data_dir() {
                        let _ = state.with_conn(|c| startup::run_daily_backup(c, &dir, now()));
                    }
                    let _ = app.emit(DATA_CHANGED, ());
                }
                last = Some(day);
            }
            std::thread::sleep(Duration::from_secs(30));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::BROWSER_ARGS;

    #[test]
    fn widget_and_manager_share_browser_args() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let widget = conf["app"]["windows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|w| w["label"] == "widget")
            .unwrap();
        assert_eq!(widget["additionalBrowserArgs"].as_str(), Some(BROWSER_ARGS));
    }

    #[test]
    fn browser_args_keep_tauri_defaults_and_disable_gpu() {
        assert!(BROWSER_ARGS.contains("--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection"));
        assert!(BROWSER_ARGS.contains("--disable-gpu"));
    }

    #[test]
    fn release_build_has_a_strict_content_security_policy() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let csp = conf["app"]["security"]["csp"].as_str().expect("csp must be set");
        assert!(csp.contains("default-src 'self';"));
        assert!(csp.contains("connect-src ipc: http://ipc.localhost"));
        assert!(!csp.contains("unsafe-eval"));
        // 외부(원격) 출처는 허용하지 않는다
        assert!(!csp.contains("https:"));
    }
}
