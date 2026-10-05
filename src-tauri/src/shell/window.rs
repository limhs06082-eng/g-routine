use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder, WindowEvent};

use crate::db::settings;
use crate::service;
use crate::shell::position::{anchored_y, compute_position, Rect};
use crate::state::AppState;
use crate::{now, startup, DATA_CHANGED};

pub const WIDGET: &str = "widget";
pub const MANAGER: &str = "manager";
const MARGIN: f64 = 12.0;
const MIN_HEIGHT: f64 = 100.0;
const MAX_HEIGHT: f64 = 560.0;

pub fn show_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

pub fn hide_widget(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(WIDGET) {
        let _ = w.hide();
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
pub fn resize_widget(app: &AppHandle, logical_height: f64) -> tauri::Result<()> {
    let Some(w) = app.get_webview_window(WIDGET) else { return Ok(()) };
    let scale = w.scale_factor()?;
    let pos = w.outer_position()?;
    let old = w.outer_size()?;
    let new_h = (logical_height.clamp(MIN_HEIGHT, MAX_HEIGHT) * scale).round() as i32;
    if new_h == old.height as i32 {
        return Ok(());
    }
    let y = anchored_y(pos.y, old.height as i32, new_h);
    mark_programmatic_move(app);
    w.set_size(PhysicalSize::new(old.width, new_h as u32))?;
    w.set_position(PhysicalPosition::new(pos.x, y))
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
            let _ = window.hide();
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
