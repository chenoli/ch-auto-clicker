//! UI → Rust commands (ARCHITECTURE §7.1).

use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri_specta::Event;

use crate::events::DevPing;

/// Initial state for a window; grows settings, profiles and engine state in later phases.
#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub version: String,
    pub dpi_awareness: String,
}

#[tauri::command]
#[specta::specta]
pub fn app_bootstrap(app: tauri::AppHandle) -> Bootstrap {
    Bootstrap {
        version: app.package_info().version.to_string(),
        dpi_awareness: dpi_awareness().to_owned(),
    }
}

/// Emits `DevPing` back to the UI (Phase 0 event channel probe).
#[tauri::command]
#[specta::specta]
pub fn dev_ping(app: tauri::AppHandle) -> Result<(), String> {
    let at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64() * 1000.0);
    DevPing { at_ms }.emit(&app).map_err(|e| e.to_string())
}

pub fn dpi_awareness() -> &'static str {
    #[cfg(windows)]
    return ch_win::display::current_dpi_awareness().as_str();
    #[cfg(not(windows))]
    return "n/a";
}
