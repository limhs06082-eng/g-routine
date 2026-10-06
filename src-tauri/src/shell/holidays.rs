//! 공휴일 표 내려받기: GitHub main 브랜치의 data/holidays.json을 하루 한 번 받아 내장 표 위에 덮어쓴다.
//! 받은 표는 DB(settings의 holidays_cache)에 보관해, 인터넷이 없는 날이나 다음 실행에도 쓴다.
//! 받지 못하면 내장 표(앱에 들어 있는 같은 파일)를 그대로 쓴다.

use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::db::settings;
use crate::domain::holidays::{self, HolidayTable};
use crate::state::AppState;
use crate::DATA_CHANGED;

const URL: &str = "https://raw.githubusercontent.com/limhs06082-eng/g-routine/main/src-tauri/data/holidays.json";
pub const CACHE_KEY: &str = "holidays_cache";
const FIRST_CHECK: Duration = Duration::from_secs(30);
const INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// 공휴일 표는 몇 KB다. 이보다 크면 받지 않는다.
const MAX_BYTES: usize = 64 * 1024;

/// 보관해 둔 표가 있으면 곧바로 쓴다 (부팅 직후, 데이터 폴더가 준비된 뒤).
pub fn load_cached(app: &AppHandle) {
    let cached = app.state::<AppState>().with_conn(|c| settings::get(c, CACHE_KEY)).ok().flatten();
    if let Some(table) = cached.as_deref().and_then(|json| HolidayTable::parse(json).ok()) {
        holidays::use_downloaded(&table);
    }
}

pub fn spawn_holiday_refresher(app: AppHandle) {
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            if let Ok(json) = tauri::async_runtime::block_on(download()) {
                apply(&app, &json);
            }
            std::thread::sleep(INTERVAL);
        }
    });
}

/// 받은 표가 올바르면 쓰고, 보관한 것과 다르면 보관한 뒤 화면을 새로 고친다.
fn apply(app: &AppHandle, json: &str) {
    let Ok(table) = HolidayTable::parse(json) else { return };
    holidays::use_downloaded(&table);
    let state = app.state::<AppState>();
    let changed = state
        .with_conn(|c| {
            let same = settings::get(c, CACHE_KEY)?.as_deref() == Some(json);
            if !same {
                settings::set(c, CACHE_KEY, json)?;
            }
            Ok(!same)
        })
        .unwrap_or(false);
    if changed {
        let _ = app.emit(DATA_CHANGED, ());
    }
}

async fn download() -> Result<String, String> {
    // 자동 업데이트(updater 플러그인)와 같은 방식으로 TLS를 준비한다
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent("G-routine")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let res = client.get(URL).send().await.map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status()));
    }
    if res.content_length().is_some_and(|n| n > MAX_BYTES as u64) {
        return Err("너무 큼".into());
    }
    let mut res = res;
    let mut body = Vec::new();
    while let Some(chunk) = res.chunk().await.map_err(|e| e.to_string())? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_BYTES {
            return Err("너무 큼".into());
        }
    }
    String::from_utf8(body).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 실제 GitHub에서 받아 본다 (네트워크가 필요해 평소에는 건너뛴다: cargo test -- --ignored)
    #[test]
    #[ignore]
    fn downloads_a_valid_table_from_github() {
        let json = tauri::async_runtime::block_on(download()).expect("download");
        let table = HolidayTable::parse(&json).expect("valid table");
        assert_eq!(table, holidays::built_in());
    }

    #[test]
    fn downloads_the_same_file_that_is_built_in() {
        // 내장 표(include_str!)와 내려받는 주소가 같은 파일을 가리켜야 한다
        assert!(URL.ends_with("/main/src-tauri/data/holidays.json"));
        assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/holidays.json").exists());
    }
}
