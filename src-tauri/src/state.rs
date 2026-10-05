use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
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
    pub portable: bool,
    pub previous_dir: Option<String>,
    pub suggested_dir: String,
    pub data_dir: Option<String>,
}

pub struct AppState {
    conn: Mutex<Option<Connection>>,
    status: Mutex<AppStatus>,
    pub location_file: PathBuf,
    pub move_seq: AtomicU64,
    pub programmatic_move: Mutex<Option<Instant>>,
}

impl AppState {
    pub fn new(location_file: PathBuf) -> Self {
        AppState {
            conn: Mutex::new(None),
            status: Mutex::new(AppStatus::default()),
            location_file,
            move_seq: AtomicU64::new(0),
            programmatic_move: Mutex::new(None),
        }
    }

    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self.conn.lock().map_err(|_| AppError::invalid("내부 상태를 읽지 못했어요"))?;
        match guard.as_ref() {
            Some(c) => f(c),
            None => Err(AppError::NotReady),
        }
    }

    pub fn replace_conn(&self, conn: Option<Connection>) {
        if let Ok(mut g) = self.conn.lock() {
            *g = conn;
        }
    }

    pub fn status(&self) -> AppStatus {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
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
