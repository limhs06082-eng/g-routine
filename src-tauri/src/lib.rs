mod commands;
mod db;
mod diagnostics;
mod domain;
mod error;
mod logging;
mod model;
mod service;
mod shell;
mod startup;
mod state;
mod storage;
mod templates;
#[cfg(test)]
mod test_util;

use tauri::Manager;

pub const DATA_CHANGED: &str = "data-changed";

pub fn now() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            shell::window::show_widget(app);
        }))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            if let Ok(dir) = app.path().app_local_data_dir() {
                logging::init(dir.join("logs"));
            }
            log::info!("G-routine {} 시작", app.package_info().version);
            let location_file = app.path().app_config_dir()?.join("location.json");
            let exe_dir = std::env::current_exe()?.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let state = state::AppState::new(location_file, exe_dir.clone());
            startup::boot(&state, &exe_dir, &storage::location::drive_candidates(), now());
            let status = state.status();
            log::info!(
                "시작 상태: 준비={} 포터블={} 손상={} 열기실패={} 예전폴더못찾음={}",
                status.ready,
                status.portable,
                status.corrupt,
                status.open_failed,
                status.previous_dir.is_some()
            );
            if let Some(dir) = state.data_dir() {
                logging::use_dir(dir.join("logs"));
            }
            app.manage(state);
            shell::startup(app.handle())?;
            shell::update::spawn_update_checker(app.handle().clone());
            shell::holidays::load_cached(app.handle());
            shell::ticker::spawn(
                app.handle().clone(),
                vec![shell::window::day_watch_job(), shell::alerts::job(), shell::holidays::job()],
            );
            let shortcut = shell::shortcut::register(app.handle());
            if !shortcut {
                log::warn!("Ctrl+Alt+G 단축키를 등록하지 못했어요 (다른 프로그램이 쓰는 중일 수 있음)");
            }
            app.state::<state::AppState>().shortcut.store(shortcut, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        })
        .on_window_event(shell::window::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::setup,
            commands::restore_backup,
            commands::retry_boot,
            commands::inspect_data_dir,
            commands::get_today,
            commands::set_done,
            commands::quick_add,
            commands::list_routines,
            commands::create_routine,
            commands::update_routine,
            commands::archive_routine,
            commands::reorder_routines,
            commands::history_month,
            commands::history_day,
            commands::get_settings,
            commands::set_setting,
            commands::set_vacation,
            commands::holiday_coverage,
            commands::save_diagnostics,
            commands::open_link,
            commands::export_backup,
            commands::import_backup,
            commands::change_data_dir,
            commands::open_manager,
            commands::resize_widget,
            commands::hide_widget,
            commands::install_update,
        ])
        .run(tauri::generate_context!())
        .expect("G-routine을 실행하지 못했어요");
}
