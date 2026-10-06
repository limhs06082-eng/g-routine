use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::{routines, settings};
use crate::domain::day::fmt_ts;
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, Routine, RoutineInput, Settings, TodayView};
use crate::shell::{self, window};
use crate::state::{AppState, AppStatus};
use crate::storage::{export, location};
use crate::{now, service, startup, DATA_CHANGED};

fn changed(app: &AppHandle) {
    let _ = app.emit(DATA_CHANGED, ());
}

/// 공휴일 표가 어느 해까지 있는지 (설정 탭 안내용)
#[tauri::command]
pub fn holiday_coverage(state: State<'_, AppState>) -> crate::domain::holidays::Coverage {
    use crate::domain::holidays;
    let today = state.with_conn(|c| service::today(c, now())).unwrap_or_else(|_| now().date());
    holidays::coverage(holidays::last_known_year(), today)
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> AppStatus {
    state.status()
}

#[tauri::command]
pub fn setup(app: AppHandle, state: State<'_, AppState>, dir: String, template: String) -> AppResult<()> {
    startup::setup(&state, Path::new(&dir), &template, now())?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn restore_backup(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    startup::restore(&state, now())?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

/// 데이터 파일을 지금 열 수 없었을 때(잠김 등) 시작 과정을 다시 시도한다.
/// 잠긴 DB는 busy_timeout(3초)만큼 기다리므로 메인 스레드를 막지 않게 async로 둔다.
#[tauri::command]
pub async fn retry_boot(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    if state.status().ready {
        return Ok(());
    }
    startup::boot(&state, &state.exe_dir, &location::drive_candidates(), now());
    if state.status().ready {
        shell::after_ready(&app);
    }
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn inspect_data_dir(dir: String) -> location::DataDirInfo {
    location::inspect_data_dir(Path::new(&dir))
}

#[tauri::command]
pub fn get_today(state: State<'_, AppState>) -> AppResult<TodayView> {
    state.with_conn(|c| service::get_today(c, now()))
}

#[tauri::command]
pub fn set_done(app: AppHandle, state: State<'_, AppState>, item_id: i64, done: bool) -> AppResult<()> {
    state.with_conn(|c| service::set_done(c, item_id, done, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn quick_add(app: AppHandle, state: State<'_, AppState>, title: String) -> AppResult<()> {
    state.with_conn(|c| service::quick_add(c, &title, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn list_routines(state: State<'_, AppState>) -> AppResult<Vec<Routine>> {
    state.with_conn(|c| service::list_routines(c, now()))
}

#[tauri::command]
pub fn create_routine(app: AppHandle, state: State<'_, AppState>, input: RoutineInput) -> AppResult<i64> {
    let id = state.with_conn(|c| service::create_routine(c, input, now()))?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub fn update_routine(app: AppHandle, state: State<'_, AppState>, id: i64, input: RoutineInput) -> AppResult<()> {
    state.with_conn(|c| service::update_routine(c, id, input, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn archive_routine(app: AppHandle, state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|c| service::archive_routine(c, id, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn reorder_routines(app: AppHandle, state: State<'_, AppState>, ids: Vec<i64>) -> AppResult<()> {
    state.with_conn(|c| service::reorder_routines(c, &ids, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub fn history_month(state: State<'_, AppState>, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    state.with_conn(|c| service::history_month(c, year, month))
}

#[tauri::command]
pub fn history_day(state: State<'_, AppState>, day: String) -> AppResult<Vec<DayItem>> {
    state.with_conn(|c| service::history_day(c, &day))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_conn(settings::load)
}

#[tauri::command]
pub fn set_setting(app: AppHandle, state: State<'_, AppState>, key: String, value: String) -> AppResult<Settings> {
    let s = state.with_conn(|c| service::set_setting(c, &key, &value, now()))?;
    shell::apply_setting_side_effects(&app, &key, &s);
    changed(&app);
    Ok(s)
}

/// 방학 · 쉬는 기간 설정. 시작일과 끝나는 날을 함께 주거나, 둘 다 비워서 해제한다.
#[tauri::command]
pub fn set_vacation(
    app: AppHandle,
    state: State<'_, AppState>,
    start: Option<String>,
    end: Option<String>,
) -> AppResult<Settings> {
    let s = state.with_conn(|c| service::set_vacation(c, start.as_deref(), end.as_deref(), now()))?;
    changed(&app);
    Ok(s)
}

#[tauri::command]
pub fn open_link(app: AppHandle, state: State<'_, AppState>, routine_id: i64) -> AppResult<()> {
    let routine = state.with_conn(|c| routines::get(c, routine_id))?;
    let link = routine
        .link
        .filter(|l| routines::is_allowed_link(l))
        .ok_or_else(|| AppError::invalid("연결된 바로가기가 없어요"))?;
    let opener = app.opener();
    let result = if link.to_ascii_lowercase().starts_with("http") {
        opener.open_url(link, None::<&str>)
    } else {
        opener.open_path(link, None::<&str>)
    };
    result.map_err(|_| AppError::invalid("바로가기를 열 수 없어요. 주소나 경로를 확인해 주세요"))
}

#[tauri::command]
pub async fn export_backup(state: State<'_, AppState>, path: String) -> AppResult<()> {
    let backup = state.with_conn(|c| export::export(c, &fmt_ts(now())))?;
    export::write_file(Path::new(&path), &backup)
}

#[tauri::command]
pub async fn import_backup(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    let backup = export::read_file(Path::new(&path))?;
    state.with_conn(|c| {
        export::import(c, &backup)?;
        service::resync_today(c, now())
    })?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn change_data_dir(app: AppHandle, state: State<'_, AppState>, dir: String) -> AppResult<()> {
    startup::change_dir(&state, Path::new(&dir), now())?;
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

/// Windows에서 동기 명령 안에서 창을 만들면 교착될 수 있어 async로 둔다.
#[tauri::command]
pub async fn open_manager(app: AppHandle) -> AppResult<()> {
    window::open_manager(&app)?;
    Ok(())
}

#[tauri::command]
pub fn resize_widget(app: AppHandle, height: f64) -> AppResult<bool> {
    Ok(window::resize_widget(&app, height)?)
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    window::hide_widget(&app);
}
