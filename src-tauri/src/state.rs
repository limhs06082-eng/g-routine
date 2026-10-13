use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub ready: bool,
    pub corrupt: bool,
    /// DB 파일이 있지만 지금은 열 수 없음 (다른 프로그램이 잠금 · 권한 등). 손상과 다르다.
    pub open_failed: bool,
    pub portable: bool,
    pub previous_dir: Option<String>,
    pub suggested_dir: String,
    pub data_dir: Option<String>,
    /// 전역 단축키(Ctrl+Alt+G)를 등록했는지. 다른 프로그램이 쓰고 있으면 false.
    pub shortcut: bool,
    /// 내려받을 수 있는 새 버전 (주기 확인에서 찾았지만 아직 설치하지 않음). 위젯이 '지금 설치'를 보여 준다.
    pub pending_update: Option<String>,
}

pub struct AppState {
    conn: Mutex<Option<Connection>>,
    status: Mutex<AppStatus>,
    pub location_file: PathBuf,
    /// 실행 파일 폴더 (포터블 모드 판단 · 다시 시도에 쓴다)
    pub exe_dir: PathBuf,
    pub move_seq: AtomicU64,
    pub programmatic_move: Mutex<Option<Instant>>,
    /// 시작 과정(boot)이 상태를 통째로 다시 쓰므로 단축키 등록 결과는 따로 둔다
    pub shortcut: AtomicBool,
    /// 설치를 기다리는 새 버전 (같은 이유로 따로 둔다)
    pending_update: Mutex<Option<String>>,
}

impl AppState {
    pub fn new(location_file: PathBuf, exe_dir: PathBuf) -> Self {
        AppState {
            conn: Mutex::new(None),
            status: Mutex::new(AppStatus::default()),
            location_file,
            exe_dir,
            move_seq: AtomicU64::new(0),
            programmatic_move: Mutex::new(None),
            shortcut: AtomicBool::new(false),
            pending_update: Mutex::new(None),
        }
    }

    pub fn set_pending_update(&self, version: Option<String>) {
        if let Ok(mut g) = self.pending_update.lock() {
            *g = version;
        }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self.conn.lock().map_err(|_| AppError::invalid("내부 상태를 읽지 못했어요"))?;
        match guard.as_ref() {
            Some(c) => f(c),
            None => Err(AppError::NotReady),
        }
    }

    /// 연결 잠금을 쥔 채로 지금 연결을 넘겨 새 연결을 만들고 바꾼다.
    /// 그동안 다른 명령은 기다리므로, 중간에 누른 체크가 옛 연결에만 쓰이고 사라지는 일이 없다.
    /// f가 실패하면 기존 연결을 그대로 둔다.
    pub fn swap_conn(&self, f: impl FnOnce(&Connection) -> AppResult<Connection>) -> AppResult<()> {
        let mut guard = self.conn.lock().map_err(|_| AppError::invalid("내부 상태를 읽지 못했어요"))?;
        let current = guard.as_ref().ok_or(AppError::NotReady)?;
        let next = f(current)?;
        *guard = Some(next);
        Ok(())
    }

    pub fn replace_conn(&self, conn: Option<Connection>) {
        if let Ok(mut g) = self.conn.lock() {
            *g = conn;
        }
    }

    pub fn status(&self) -> AppStatus {
        let mut s = self.status.lock().map(|s| s.clone()).unwrap_or_default();
        s.shortcut = self.shortcut.load(Ordering::Relaxed);
        s.pending_update = self.pending_update.lock().ok().and_then(|g| g.clone());
        s
    }

    pub fn update_status(&self, f: impl FnOnce(&mut AppStatus)) {
        if let Ok(mut g) = self.status.lock() {
            f(&mut g);
        }
    }

    pub fn data_dir(&self) -> Option<PathBuf> {
        self.status().data_dir.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_in_memory;

    fn marker(c: &Connection) -> String {
        c.query_row("SELECT value FROM settings WHERE key = 'marker'", [], |r| r.get(0)).unwrap()
    }

    fn conn_with_marker(m: &str) -> Connection {
        let c = open_in_memory().unwrap();
        c.execute("INSERT INTO settings (key, value) VALUES ('marker', ?1)", [m]).unwrap();
        c
    }

    #[test]
    fn swap_conn_replaces_the_connection_while_holding_the_lock() {
        let s = AppState::new(PathBuf::from("loc.json"), PathBuf::from("exe"));
        s.replace_conn(Some(conn_with_marker("old")));
        s.swap_conn(|old| {
            assert_eq!(marker(old), "old");
            Ok(conn_with_marker("new"))
        })
        .unwrap();
        assert_eq!(s.with_conn(|c| Ok(marker(c))).unwrap(), "new");
    }

    #[test]
    fn swap_conn_keeps_the_old_connection_when_it_fails() {
        let s = AppState::new(PathBuf::from("loc.json"), PathBuf::from("exe"));
        s.replace_conn(Some(conn_with_marker("old")));
        assert!(s.swap_conn(|_| Err(AppError::invalid("실패"))).is_err());
        assert_eq!(s.with_conn(|c| Ok(marker(c))).unwrap(), "old");
    }

    #[test]
    fn a_pending_update_shows_in_status_and_survives_a_status_rewrite() {
        let s = AppState::new(PathBuf::from("loc.json"), PathBuf::from("exe"));
        assert_eq!(s.status().pending_update, None);
        s.set_pending_update(Some("0.4.0".into()));
        s.update_status(|st| *st = AppStatus::default()); // boot가 상태를 통째로 다시 쓰는 경우
        assert_eq!(s.status().pending_update.as_deref(), Some("0.4.0"));
        s.set_pending_update(None);
        assert_eq!(s.status().pending_update, None);
    }

    #[test]
    fn swap_conn_requires_an_open_connection() {
        let s = AppState::new(PathBuf::from("loc.json"), PathBuf::from("exe"));
        assert!(matches!(s.swap_conn(|_| Ok(open_in_memory().unwrap())), Err(AppError::NotReady)));
    }
}
