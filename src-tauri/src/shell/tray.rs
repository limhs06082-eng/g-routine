use tauri::menu::{CheckMenuItem, CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::db::settings;
use crate::model::Settings;
use crate::shell::{apply_setting_side_effects, window};
use crate::state::AppState;
use crate::DATA_CHANGED;

pub struct TrayChecks {
    top: CheckMenuItem<Wry>,
    autostart: CheckMenuItem<Wry>,
}

pub fn build(app: &AppHandle, s: Option<&Settings>) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "위젯 보이기").build(app)?;
    let manager = MenuItemBuilder::with_id("manager", "관리 창 열기").build(app)?;
    let top = CheckMenuItemBuilder::with_id("top", "항상 맨 위에 표시")
        .checked(s.map(|s| s.always_on_top).unwrap_or(false))
        .build(app)?;
    let autostart = CheckMenuItemBuilder::with_id("autostart", "컴퓨터 켜면 자동 시작")
        .checked(s.map(|s| s.autostart).unwrap_or(false))
        .build(app)?;
    let reset = MenuItemBuilder::with_id("reset", "위치 초기화").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "종료").build(app)?;
    let menu = MenuBuilder::new(app)
        .item(&show)
        .item(&manager)
        .separator()
        .item(&top)
        .item(&autostart)
        .separator()
        .item(&reset)
        .item(&quit)
        .build()?;

    let mut builder = TrayIconBuilder::with_id("main")
        .tooltip("G-routine")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => window::show_widget(app),
            "manager" => {
                let _ = window::open_manager(app);
            }
            "top" => toggle(app, "always_on_top"),
            "autostart" => toggle(app, "autostart"),
            "reset" => window::reset_position(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                window::show_widget(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;
    app.manage(TrayChecks { top, autostart });
    Ok(())
}

pub fn sync_checks(app: &AppHandle, s: &Settings) {
    if let Some(c) = app.try_state::<TrayChecks>() {
        let _ = c.top.set_checked(s.always_on_top);
        let _ = c.autostart.set_checked(s.autostart);
    }
}

fn toggle(app: &AppHandle, key: &str) {
    let state = app.state::<AppState>();
    let result = state.with_conn(|c| {
        let s = settings::load(c)?;
        let current = if key == "always_on_top" { s.always_on_top } else { s.autostart };
        settings::apply(c, key, if current { "false" } else { "true" })?;
        settings::load(c)
    });
    match result {
        Ok(s) => {
            apply_setting_side_effects(app, key, &s);
            let _ = app.emit(DATA_CHANGED, ());
        }
        Err(_) => {
            if let Some(c) = app.try_state::<TrayChecks>() {
                let _ = c.top.set_checked(false);
                let _ = c.autostart.set_checked(false);
            }
        }
    }
}
