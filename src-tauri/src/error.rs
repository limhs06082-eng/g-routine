use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
    /// DB 파일은 열렸지만 무결성 검사(quick_check)를 통과하지 못함
    Corrupt,
    NotReady,
}

impl AppError {
    pub fn invalid(msg: impl Into<String>) -> Self {
        AppError::Invalid(msg.into())
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Db(e) => write!(f, "데이터를 저장하거나 읽지 못했어요 ({e})"),
            AppError::Io(e) => write!(f, "파일을 다루지 못했어요 ({e})"),
            AppError::Json(e) => write!(f, "백업 파일 형식이 올바르지 않아요 ({e})"),
            AppError::Invalid(m) => write!(f, "{m}"),
            AppError::Corrupt => write!(f, "데이터 파일이 손상되었어요"),
            AppError::NotReady => write!(f, "아직 시작 설정을 마치지 않았어요"),
        }
    }
}

impl std::error::Error for AppError {}

/// 파일이 손상되었다는 뜻의 오류인지 (잠김 · 권한 같은 "지금은 못 여는" 오류와 구분).
pub fn is_corruption(err: &AppError) -> bool {
    use rusqlite::ErrorCode;
    match err {
        AppError::Corrupt => true,
        AppError::Db(rusqlite::Error::SqliteFailure(e, _)) => {
            matches!(e.code, ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt)
        }
        _ => false,
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Db(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Io(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Json(e)
    }
}

impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        AppError::Invalid(format!("창을 다루지 못했어요 ({e})"))
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
