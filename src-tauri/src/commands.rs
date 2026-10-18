use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::{routines, settings};
use crate::domain::day::fmt_ts;
use crate::error::{AppError, AppResult};
use crate::model::{DayItem, DaySummary, Routine, RoutineInput, Settings, TodayView};
use crate::shell::{self, window};
use crate::state::{AppState, AppStatus};
use crate::storage::{export, location, offsite};
use crate::{now, service, startup, DATA_CHANGED};

fn changed(app: &AppHandle) {
    let _ = app.emit(DATA_CHANGED, ());
}

/// 공휴일 표가 어느 해까지 있는지 (설정 탭 안내용)
#[tauri::command]
pub async fn holiday_coverage(state: State<'_, AppState>) -> AppResult<crate::domain::holidays::Coverage> {
    use crate::domain::holidays;
    let today = state.with_conn(|c| service::today(c, now())).unwrap_or_else(|_| now().date());
    Ok(holidays::coverage(holidays::last_known_year(), today))
}

#[tauri::command]
pub fn get_status(state: State<'_, AppState>) -> AppStatus {
    state.status()
}

#[tauri::command]
pub async fn setup(app: AppHandle, state: State<'_, AppState>, dir: String, template: String) -> AppResult<()> {
    startup::setup(&state, Path::new(&dir), &template, now())?;
    // 새로 설치한 사람에게는 '바뀐 점' 안내를 띄우지 않는다
    let version = app.package_info().version.to_string();
    let _ = state.with_conn(|c| settings::apply(c, settings::SEEN_VERSION, &version));
    shell::after_ready(&app);
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn restore_backup(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
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
pub async fn inspect_data_dir(dir: String) -> location::DataDirInfo {
    location::inspect_data_dir(Path::new(&dir))
}

#[tauri::command]
pub async fn get_today(state: State<'_, AppState>) -> AppResult<TodayView> {
    state.with_conn(|c| service::get_today(c, now()))
}

#[tauri::command]
pub async fn set_done(app: AppHandle, state: State<'_, AppState>, item_id: i64, done: bool) -> AppResult<()> {
    state.with_conn(|c| service::set_done(c, item_id, done, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn quick_add(app: AppHandle, state: State<'_, AppState>, title: String) -> AppResult<()> {
    state.with_conn(|c| service::quick_add(c, &title, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn list_routines(state: State<'_, AppState>) -> AppResult<Vec<Routine>> {
    state.with_conn(|c| service::list_routines(c, now()))
}

#[tauri::command]
pub async fn create_routine(app: AppHandle, state: State<'_, AppState>, input: RoutineInput) -> AppResult<i64> {
    let id = state.with_conn(|c| service::create_routine(c, input, now()))?;
    changed(&app);
    Ok(id)
}

#[tauri::command]
pub async fn update_routine(app: AppHandle, state: State<'_, AppState>, id: i64, input: RoutineInput) -> AppResult<()> {
    state.with_conn(|c| service::update_routine(c, id, input, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn archive_routine(app: AppHandle, state: State<'_, AppState>, id: i64) -> AppResult<()> {
    state.with_conn(|c| service::archive_routine(c, id, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn reorder_routines(app: AppHandle, state: State<'_, AppState>, ids: Vec<i64>) -> AppResult<()> {
    state.with_conn(|c| service::reorder_routines(c, &ids, now()))?;
    changed(&app);
    Ok(())
}

#[tauri::command]
pub async fn history_month(state: State<'_, AppState>, year: i32, month: u32) -> AppResult<Vec<DaySummary>> {
    state.with_conn(|c| service::history_month(c, year, month))
}

#[tauri::command]
pub async fn history_day(state: State<'_, AppState>, day: String) -> AppResult<Vec<DayItem>> {
    state.with_conn(|c| service::history_day(c, &day))
}

/// 기간 통계: 날짜별 달성, 루틴별 달성률, 연속 달성 일수
#[tauri::command]
pub async fn history_stats(state: State<'_, AppState>, from: String, to: String) -> AppResult<crate::stats::Stats> {
    state.with_conn(|c| {
        let today = service::today(c, now())?;
        crate::stats::history_stats(c, &from, &to, today)
    })
}

/// 기간의 기록을 CSV로 저장한다 (엑셀에서 한글이 깨지지 않게 BOM을 붙인다)
#[tauri::command]
pub async fn export_csv(state: State<'_, AppState>, path: String, from: String, to: String) -> AppResult<()> {
    let csv = state.with_conn(|c| crate::stats::export_csv(c, &from, &to))?;
    std::fs::write(&path, format!("\u{feff}{csv}"))?;
    Ok(())
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    state.with_conn(settings::load)
}

#[tauri::command]
pub async fn set_setting(app: AppHandle, state: State<'_, AppState>, key: String, value: String) -> AppResult<Settings> {
    let s = state.with_conn(|c| service::set_setting(c, &key, &value, now()))?;
    shell::apply_setting_side_effects(&app, &key, &s);
    changed(&app);
    Ok(s)
}

/// 방학 · 쉬는 기간 설정. 시작일과 끝나는 날을 함께 주거나, 둘 다 비워서 해제한다.
#[tauri::command]
pub async fn set_vacation(
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
pub async fn open_link(app: AppHandle, state: State<'_, AppState>, routine_id: i64) -> AppResult<()> {
    let routine = state.with_conn(|c| routines::get(c, routine_id))?;
    let link =
        routine.link.filter(|l| routines::is_allowed_link(l)).ok_or_else(|| AppError::invalid("연결된 바로가기가 없어요"))?;
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
pub fn resize_widget(app: AppHandle, height: f64, width: Option<f64>) -> AppResult<bool> {
    Ok(window::resize_widget(&app, height, width)?)
}

#[tauri::command]
pub fn hide_widget(app: AppHandle) {
    window::hide_widget(&app);
}

/// 두 번째 백업(OneDrive · USB): 지금 상태와, 정하지 않았으면 권할 OneDrive 폴더
#[tauri::command]
pub async fn backup2_info(state: State<'_, AppState>) -> AppResult<offsite::Backup2Info> {
    state.with_conn(|c| offsite::info(c, offsite::onedrive_dir()))
}

/// 두 번째 백업 폴더를 정하거나(곧바로 한 번 백업한다) 끈다(None)
#[tauri::command]
pub async fn set_backup2_dir(state: State<'_, AppState>, dir: Option<String>) -> AppResult<offsite::Backup2Info> {
    let today = state.with_conn(|c| service::today(c, now()))?;
    state
        .with_conn(|c| {
            offsite::set_dir(c, dir.as_deref().map(Path::new), today, &fmt_ts(now()))?;
            offsite::info(c, offsite::onedrive_dir())
        })
        .inspect(|i| log::info!("두 번째 백업 폴더: {}", if i.dir.is_some() { "정함" } else { "끔" }))
}

#[tauri::command]
pub async fn backup2_now(state: State<'_, AppState>) -> AppResult<offsite::Backup2Info> {
    let today = state.with_conn(|c| service::today(c, now()))?;
    state.with_conn(|c| {
        let dir = settings::get(c, settings::BACKUP2_DIR)?.ok_or_else(|| AppError::invalid("백업할 폴더를 먼저 골라 주세요"))?;
        offsite::write_backup(c, Path::new(&dir), today, &fmt_ts(now()))?;
        offsite::info(c, offsite::onedrive_dir())
    })
}

/// 위젯의 OneDrive 백업 권유 카드를 '나중에'로 닫는다 (설정 탭에서는 계속 보인다)
#[tauri::command]
pub async fn dismiss_backup2(app: AppHandle, state: State<'_, AppState>) -> AppResult<()> {
    state.with_conn(|c| settings::set(c, settings::BACKUP2_DISMISSED, "true"))?;
    changed(&app);
    Ok(())
}

/// '문제 보고용 정보 저장': 상태 · 설정 · 데이터 개수 · 최근 로그를 글 파일 하나로 저장한다
#[tauri::command]
pub async fn save_diagnostics(app: AppHandle, state: State<'_, AppState>, path: String) -> AppResult<()> {
    use crate::{diagnostics, logging};
    let status = state.status();
    let version = app.package_info().version.to_string();
    let created_at = now().format("%Y-%m-%d %H:%M:%S").to_string();
    let log_lines = logging::current_dir().map(|d| logging::recent_lines(&d, 400)).unwrap_or_default();
    let text = diagnostics::report(&diagnostics::ReportInput {
        version: &version,
        created_at: &created_at,
        status: &status,
        settings: state.with_conn(diagnostics::settings_text).ok(),
        db: state.with_conn(diagnostics::db_facts).ok(),
        log_lines,
    });
    // 메모장에서 한글이 깨지지 않게 BOM을 붙인다
    std::fs::write(&path, format!("\u{feff}{text}"))?;
    log::info!("문제 보고용 정보를 저장했어요");
    Ok(())
}

/// 위젯의 '지금 설치' 버튼. 받아서 설치하는 동안 앱이 꺼졌다 다시 켜진다.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> AppResult<()> {
    shell::update::install_now(&app).await.map(|_| ()).map_err(|e| {
        log::warn!("업데이트 설치 실패: {e}");
        AppError::invalid("새 버전을 설치하지 못했어요. 잠시 후 다시 시도해 주세요")
    })
}
