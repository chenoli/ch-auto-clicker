//! UI → Rust commands (ARCHITECTURE §7.1).

use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};

use crate::capabilities::Capabilities;
use crate::state::{AppState, refresh_capabilities};

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

/// Shortest reliable interval on this machine (provisional 4 ms until measured).
#[tauri::command]
#[specta::specta]
pub fn engine_capabilities(state: State<'_, AppState>) -> Capabilities {
    state.capabilities()
}

/// Re-runs the timing benchmark (~3 s) and returns the new capabilities.
#[tauri::command]
#[specta::specta]
pub async fn engine_run_benchmark(app: AppHandle) -> Result<Capabilities, String> {
    tauri::async_runtime::spawn_blocking(move || refresh_capabilities(&app))
        .await
        .map_err(|e| e.to_string())?
}

/// Delay that lets the developer move the cursor off the button before the test click.
const DEBUG_CLICK_DELAY: Duration = Duration::from_secs(2);

/// Dev-only (T1.7): one left click at the cursor after a short delay.
#[tauri::command]
#[specta::specta]
pub fn debug_click_once() -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("debug_click_once is only available in dev builds".into());
    }
    std::thread::spawn(|| {
        std::thread::sleep(DEBUG_CLICK_DELAY);
        match debug_click() {
            Ok(()) => log::info!("debug click sent"),
            Err(e) => log::warn!("debug click failed: {e}"),
        }
    });
    Ok(())
}

#[cfg(windows)]
fn debug_click() -> Result<(), ch_core::input::InputError> {
    use ch_core::input::{ClickKind, InputBackend, MouseButton};
    ch_win::input::SendInputBackend::system().click(MouseButton::Left, ClickKind::Single)
}

#[cfg(not(windows))]
fn debug_click() -> Result<(), ch_core::input::InputError> {
    Ok(())
}

pub fn dpi_awareness() -> &'static str {
    #[cfg(windows)]
    return ch_win::display::current_dpi_awareness().as_str();
    #[cfg(not(windows))]
    return "n/a";
}
