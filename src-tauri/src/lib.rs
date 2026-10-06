mod commands;
mod db;
mod domain;
mod error;
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
        .setup(|app| {
            let location_file = app.path().app_config_dir()?.join("location.json");
            let exe_dir = std::env::current_exe()?
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_default();
            let state = state::AppState::new(location_file, exe_dir.clone());
            startup::boot(&state, &exe_dir, &storage::location::drive_candidates(), now());
            app.manage(state);
            shell::startup(app.handle())?;
            shell::update::spawn_update_checker(app.handle().clone());
            shell::alerts::spawn_due_alerts(app.handle().clone());
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
            commands::open_link,
            commands::export_backup,
            commands::import_backup,
            commands::change_data_dir,
            commands::open_manager,
            commands::resize_widget,
            commands::hide_widget,
        ])
        .run(tauri::generate_context!())
        .expect("G-routine을 실행하지 못했어요");
}
