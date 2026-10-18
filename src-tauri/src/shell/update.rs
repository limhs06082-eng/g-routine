//! 자동 업데이트: GitHub 릴리스의 latest.json을 주기적으로 확인한다.
//! 켜진 직후 첫 확인에서 새 버전이 있으면 바로 설치한다 (PC를 막 켠 때라 수업 중일 가능성이 낮고,
//! 복원 프로그램이 예전 버전으로 되돌린 PC도 이때 다시 올라온다).
//! 그 뒤 6시간마다의 확인에서는 설치하지 않고 위젯에 "새 버전 준비됨 · 지금 설치"만 띄운다.
//! 수업 중 TV에 화면을 띄운 상태에서 앱이 갑자기 꺼지고 설치 창이 뜨는 일을 막기 위해서다.
//! Windows에서는 서명을 확인한 설치 파일을 실행하는 순간 앱이 스스로 종료되고,
//! 설치(passive)가 끝나면 NSIS가 앱을 다시 실행한다.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::state::AppState;
use crate::DATA_CHANGED;

/// 앱이 켜진 뒤 첫 확인까지 기다리는 시간 (부팅 직후 네트워크가 준비될 시간)
const FIRST_CHECK: Duration = Duration::from_secs(20);
/// 이후 확인 주기
const INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// 자동 업데이트를 해도 되는 실행인지.
/// 개발 빌드는 건너뛰고, 포터블 모드도 건너뛴다(설치형 업데이트를 받으면 포터블 폴더가 아니라 AppData에 따로 설치되므로).
pub fn auto_update_enabled(debug_build: bool, portable: bool) -> bool {
    !debug_build && !portable
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallAction {
    /// 바로 받아서 설치한다 (앱이 꺼졌다 다시 켜진다)
    InstallNow,
    /// 설치하지 않고 위젯에 알린다. 사용자가 '지금 설치'를 누르면 설치한다
    Notify,
}

/// 켜진 뒤 이 시간 안에 처음 성공한 확인에서만 스스로 설치한다 (수업 중에 갑자기 꺼지지 않게)
const AUTO_INSTALL_WINDOW: Duration = Duration::from_secs(10 * 60);
/// 아직 한 번도 확인하지 못했을 때 다시 시도하는 간격 (학교 네트워크 로그온이 늦는 PC)
const RETRY_SOON: Duration = Duration::from_secs(2 * 60);
const RETRY_LATER: Duration = Duration::from_secs(30 * 60);

/// 켜진 직후(첫 10분)의 첫 성공한 확인에서만 스스로 설치하고, 그 뒤 확인은 알리기만 한다.
/// 복원 프로그램이 예전 버전으로 되돌린 PC가 새 DB를 예전 앱으로 오래 쓰지 않도록, 처음 확인이 실패하면 곧 다시 시도한다.
pub fn install_action(checked_once: bool, since_launch: Duration) -> InstallAction {
    if !checked_once && since_launch < AUTO_INSTALL_WINDOW {
        InstallAction::InstallNow
    } else {
        InstallAction::Notify
    }
}

/// 다음 확인까지 기다릴 시간
pub fn next_wait(checked_once: bool, since_launch: Duration) -> Duration {
    if checked_once {
        INTERVAL
    } else if since_launch < AUTO_INSTALL_WINDOW {
        RETRY_SOON
    } else {
        RETRY_LATER
    }
}

pub fn spawn_update_checker(app: AppHandle) {
    let portable = app.state::<AppState>().status().portable;
    if !auto_update_enabled(cfg!(debug_assertions), portable) {
        return;
    }
    std::thread::spawn(move || {
        let launched = std::time::Instant::now();
        std::thread::sleep(FIRST_CHECK);
        let mut checked_once = false;
        loop {
            let action = install_action(checked_once, launched.elapsed());
            match tauri::async_runtime::block_on(check(&app, action)) {
                Ok(()) => checked_once = true,
                // 네트워크가 없거나 GitHub에 닿지 않으면 곧 다시 시도한다
                Err(e) => log::info!("새 버전을 확인하지 못했어요: {e}"),
            }
            std::thread::sleep(next_wait(checked_once, launched.elapsed()));
        }
    });
}

async fn check(app: &AppHandle, action: InstallAction) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    match action {
        // Windows: 설치 파일을 실행한 뒤 이 호출 안에서 프로세스가 종료된다.
        InstallAction::InstallNow => update.download_and_install(|_, _| {}, || {}).await?,
        InstallAction::Notify => {
            let state = app.state::<AppState>();
            if state.status().pending_update.as_deref() != Some(update.version.as_str()) {
                state.set_pending_update(Some(update.version.clone()));
                let _ = app.emit(DATA_CHANGED, ());
            }
        }
    }
    Ok(())
}

/// 위젯의 '지금 설치': 새 버전을 다시 확인해 받아서 설치한다. 이미 최신이면 안내를 지운다.
pub async fn install_now(app: &AppHandle) -> tauri_plugin_updater::Result<bool> {
    let Some(update) = app.updater()?.check().await? else {
        app.state::<AppState>().set_pending_update(None);
        let _ = app.emit(DATA_CHANGED, ());
        return Ok(false);
    };
    update.download_and_install(|_, _| {}, || {}).await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::{auto_update_enabled, install_action, next_wait, InstallAction, INTERVAL};
    use std::time::Duration;

    #[test]
    fn only_the_first_successful_check_soon_after_launch_installs_by_itself() {
        let min = |m: u64| Duration::from_secs(m * 60);
        // 켜진 직후(PC를 막 켠 때)는 설치해도 된다
        assert_eq!(install_action(false, Duration::from_secs(20)), InstallAction::InstallNow);
        // 첫 확인이 실패했어도 10분 안이면 다시 시도해 설치한다 (늦게 붙는 학교 네트워크)
        assert_eq!(install_action(false, min(6)), InstallAction::InstallNow);
        // 한 번 확인한 뒤나, 켜진 지 오래되면(수업 중일 수 있다) 알리기만 한다
        assert_eq!(install_action(true, min(1)), InstallAction::Notify);
        assert_eq!(install_action(false, min(40)), InstallAction::Notify);
        // 아직 확인을 못 했으면 곧 다시 시도하고, 확인했으면 6시간 뒤
        assert_eq!(next_wait(false, min(1)), min(2));
        assert_eq!(next_wait(false, min(40)), min(30));
        assert_eq!(next_wait(true, min(1)), INTERVAL);
    }

    #[test]
    fn only_installed_release_builds_update_themselves() {
        assert!(auto_update_enabled(false, false));
        assert!(!auto_update_enabled(true, false));
        assert!(!auto_update_enabled(false, true));
    }

    #[test]
    fn updater_points_at_github_latest_release_with_a_public_key() {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        let updater = &conf["plugins"]["updater"];
        assert_eq!(
            updater["endpoints"][0].as_str(),
            Some("https://github.com/limhs06082-eng/g-routine/releases/latest/download/latest.json")
        );
        assert!(updater["pubkey"].as_str().is_some_and(|k| k.len() > 40));
        assert_eq!(updater["windows"]["installMode"].as_str(), Some("passive"));
        assert_eq!(conf["bundle"]["createUpdaterArtifacts"].as_bool(), Some(true));
    }
}
