use serde::{Serialize, Serializer};
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Db(rusqlite::Error),
    Io(std::io::Error),
    /// 원인은 로그(Debug)에만 쓴다
    #[allow(dead_code)]
    Json(serde_json::Error),
    Invalid(String),
    /// 창을 다루다 실패 (자세한 내용은 로그에만 남긴다)
    #[allow(dead_code)]
    Window(String),
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
            AppError::Db(_) => write!(f, "데이터를 저장하거나 읽지 못했어요. 잠시 후 다시 시도해 주세요"),
            AppError::Io(e) => write!(f, "{}", io_message(e)),
            AppError::Json(_) => write!(f, "백업 파일 형식이 올바르지 않아요"),
            AppError::Invalid(m) => write!(f, "{m}"),
            AppError::Window(_) => write!(f, "창을 다루지 못했어요. 잠시 후 다시 시도해 주세요"),
            AppError::Corrupt => write!(f, "데이터 파일이 손상되었어요"),
            AppError::NotReady => write!(f, "아직 시작 설정을 마치지 않았어요"),
        }
    }
}

impl std::error::Error for AppError {}

/// 파일 오류를 선생님이 이해할 수 있는 문장으로 (영어 원문은 로그에만)
fn io_message(e: &std::io::Error) -> &'static str {
    use std::io::ErrorKind;
    match e.kind() {
        ErrorKind::NotFound => "파일이나 폴더를 찾지 못했어요",
        ErrorKind::PermissionDenied => "그 폴더에 쓸 권한이 없어요. 다른 폴더를 골라 주세요",
        ErrorKind::StorageFull => "디스크 공간이 부족해요",
        _ => "파일을 다루지 못했어요. 잠시 후 다시 시도해 주세요",
    }
}

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
        AppError::Window(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// 화면으로 보내는 오류는 한국어 문장만 보내고, 원인(영어 원문 포함)은 로그에 남긴다.
impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            AppError::Invalid(_) | AppError::NotReady => log::info!("안내: {self}"),
            _ => log::warn!("오류: {self} | {self:?}"),
        }
        s.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_shown_to_teachers_have_no_library_text() {
        let db = AppError::from(rusqlite::Error::InvalidQuery);
        let io = AppError::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access is denied. (os error 5)"));
        let json = AppError::from(serde_json::from_str::<u32>("x").unwrap_err());
        for e in [db, io, json, AppError::Window("WebView2 error".into())] {
            let shown = serde_json::to_string(&e).unwrap();
            assert!(!shown.chars().any(|c| c.is_ascii_alphabetic()), "{shown}");
        }
    }
}
