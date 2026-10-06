//! 자동 업데이트: GitHub 릴리스의 latest.json을 주기적으로 확인해 새 버전이 있으면 받아서 설치한다.
//! Windows에서는 서명을 확인한 설치 파일을 실행하는 순간 앱이 스스로 종료되고,
//! 설치(passive)가 끝나면 NSIS가 앱을 다시 실행한다.

use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::state::AppState;

/// 앱이 켜진 뒤 첫 확인까지 기다리는 시간 (부팅 직후 네트워크가 준비될 시간)
const FIRST_CHECK: Duration = Duration::from_secs(20);
/// 이후 확인 주기
const INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// 자동 업데이트를 해도 되는 실행인지.
/// 개발 빌드는 건너뛰고, 포터블 모드도 건너뛴다(설치형 업데이트를 받으면 포터블 폴더가 아니라 AppData에 따로 설치되므로).
pub fn auto_update_enabled(debug_build: bool, portable: bool) -> bool {
    !debug_build && !portable
}

pub fn spawn_update_checker(app: AppHandle) {
    let portable = app.state::<AppState>().status().portable;
    if !auto_update_enabled(cfg!(debug_assertions), portable) {
        return;
    }
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            // 네트워크가 없거나 GitHub에 닿지 않으면 다음 주기에 다시 시도한다.
            let _ = tauri::async_runtime::block_on(check_and_install(&app));
            std::thread::sleep(INTERVAL);
        }
    });
}

async fn check_and_install(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    if let Some(update) = app.updater()?.check().await? {
        // Windows: 설치 파일을 실행한 뒤 이 호출 안에서 프로세스가 종료된다.
        update.download_and_install(|_, _| {}, || {}).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::auto_update_enabled;

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
