//! Rotating file log in `%LOCALAPPDATA%\Chenoli\CH Clicker\logs` (5 MB x 5) and panic capture.

use std::path::PathBuf;

use tauri::Runtime;
use tauri::plugin::TauriPlugin;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};

const MAX_FILE_BYTES: u128 = 5 * 1024 * 1024;
const KEEP_FILES: usize = 5;

pub fn log_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map_or_else(std::env::temp_dir, PathBuf::from);
    base.join("Chenoli").join("CH Clicker").join("logs")
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .target(Target::new(TargetKind::Folder {
            path: log_dir(),
            file_name: Some("ch-clicker".into()),
        }))
        .target(Target::new(TargetKind::Stdout))
        .max_file_size(MAX_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(KEEP_FILES))
        .level(if cfg!(debug_assertions) {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .build()
}

/// Logs panics with a backtrace before running the default hook.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("panic: {info}\n{backtrace}");
        default_hook(info);
    }));
}
