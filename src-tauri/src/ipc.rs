//! Typed command/event registry (tauri-specta) and TypeScript bindings export.

use tauri_specta::{Builder, collect_commands, collect_events};

use crate::{capabilities, commands, events, hotkeys};

const BINDINGS_PATH: &str = "../ui/src/ipc/bindings.ts";
const BINDINGS_HEADER: &str = "// Regenerate with `cargo test -p ch-clicker export_bindings`.";

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_bootstrap,
            commands::settings_update,
            commands::profile_update,
            commands::engine_start,
            commands::engine_stop,
            commands::engine_toggle_pause,
            commands::engine_ack_error,
            commands::engine_snapshot,
            commands::engine_capabilities,
            commands::engine_run_benchmark,
            commands::hotkey_status,
            commands::hotkey_begin_record,
            commands::hotkey_cancel_record,
            commands::hotkey_set,
            commands::hotkey_reset_defaults,
            commands::hotkey_retry,
            commands::screen_layout,
            commands::overlay_pick_points,
            commands::overlay_state,
            commands::overlay_add_point,
            commands::overlay_undo,
            commands::overlay_finish,
            commands::window_set_content_height,
            commands::window_open_variation,
            commands::debug_click_once
        ])
        .events(collect_events![
            events::EngineTick,
            events::EngineStatus,
            events::EngineJitter,
            events::EngineDegraded,
            events::SettingsChanged,
            events::ProfilesChanged,
            hotkeys::HotkeyStatusChanged,
            hotkeys::HotkeyRecording,
            hotkeys::HotkeyActionFailed,
            capabilities::Capabilities
        ])
}

pub fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(
            specta_typescript::Typescript::default().header(BINDINGS_HEADER),
            BINDINGS_PATH,
        )
        .expect("failed to export TypeScript bindings");
}

#[cfg(test)]
mod tests {
    /// Regenerates `ui/src/ipc/bindings.ts`; CI fails if the result differs from the committed file.
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::builder());
    }
}
