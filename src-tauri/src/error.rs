//! Error shape for every command (ARCHITECTURE §7): a stable `code` the UI translates
//! (`errors.<code>`) plus an English message for logs.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, specta::Type)]
pub struct AppError {
    pub code: String,
    pub message: String,
}

impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<ch_engine::plan::PlanError> for AppError {
    fn from(e: ch_engine::plan::PlanError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<ch_core::validate::ValidationError> for AppError {
    fn from(e: ch_core::validate::ValidationError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

impl From<ch_storage::StorageError> for AppError {
    fn from(e: ch_storage::StorageError) -> Self {
        Self::new(e.code(), e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_domain_errors_keeping_their_codes() {
        let e: AppError = ch_core::validate::ValidationError::IntervalRange.into();
        assert_eq!(e.code, "INTERVAL_RANGE");
        let e: AppError = ch_engine::plan::PlanError::NotAvailableYet("scan").into();
        assert_eq!(e.code, "MODE_NOT_AVAILABLE");
        let e: AppError = ch_storage::StorageError::Io {
            path: "x".into(),
            source: std::io::Error::other("disk"),
        }
        .into();
        assert_eq!(e.code, "STORAGE_IO");
        assert_eq!(AppError::new("X", "y").to_string(), "X: y");
    }

    #[test]
    fn serializes_code_and_message() {
        let json = serde_json::to_string(&AppError::new("STALE_PROFILE", "changed elsewhere"))
            .expect("json");
        assert_eq!(
            json,
            r#"{"code":"STALE_PROFILE","message":"changed elsewhere"}"#
        );
    }
}
