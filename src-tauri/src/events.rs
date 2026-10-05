//! Rust → UI events (ARCHITECTURE §7.2).

use serde::{Deserialize, Serialize};

/// Phase 0 probe for the typed Rust → UI path; removed once real events exist.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct DevPing {
    pub at_ms: f64,
}
