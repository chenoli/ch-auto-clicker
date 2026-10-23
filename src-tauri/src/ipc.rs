use tauri_specta::{Builder, collect_commands, collect_events};

use crate::{
    capabilities, commands, events, hotkeys, hud, overlay, profiles, tray, tray_menu, updater,
};

#[cfg(any(debug_assertions, test))]
const BINDINGS_PATH: &str = "../ui/src/ipc/bindings.ts";

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_bootstrap,
            commands::settings_update,
            commands::settings_get,
            commands::settings_acknowledge_anti_cheat,
            commands::app_restart_as_admin,
            commands::app_open_profiles_folder,
            commands::app_open_support,
            hud::hud_resize,
            tray_menu::tray_menu_resize,
            tray_menu::tray_menu_close,
            tray_menu::tray_menu_action,
            updater::update_check,
            updater::update_install,
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
            commands::hotkey_record_key,
            commands::hotkey_cancel_record,
            commands::hotkey_set,
            commands::hotkey_reset_defaults,
            commands::hotkey_retry,
            commands::screen_layout,
            profiles::profile_create,
            profiles::profile_duplicate,
            profiles::profile_rename,
            profiles::profile_delete,
            profiles::profile_reorder,
            profiles::profile_activate,
            profiles::profile_export,
            profiles::profile_import,
            profiles::profile_import_text,
            commands::overlay_pick_points,
            commands::overlay_pick_area,
            commands::overlay_set_mode,
            commands::overlay_confirm_area,
            commands::overlay_state,
            commands::overlay_add_point,
            commands::overlay_undo,
            commands::overlay_finish,
            commands::overlay_cancel,
            commands::overlay_focus,
            commands::overlay_focus_cursor,
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
            capabilities::Capabilities,
            overlay::OverlayChanged,
            hud::HudHover,
            tray_menu::TrayMenuOpened,
            tray::NavigateRequested,
            updater::UpdateAvailable
        ])
}

#[cfg(any(debug_assertions, test))]
pub fn export_bindings(builder: &Builder<tauri::Wry>) {
    builder
        .export(specta_typescript::Typescript::default(), BINDINGS_PATH)
        .expect("failed to export TypeScript bindings");
    let generated = std::fs::read_to_string(BINDINGS_PATH).expect("read the TypeScript bindings");
    std::fs::write(BINDINGS_PATH, without_comments(&generated))
        .expect("write the TypeScript bindings");
}

#[cfg(any(debug_assertions, test))]
fn without_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut in_block = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if in_block {
            in_block = !trimmed.contains("*/");
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("/*") {
            in_block = !trimmed.contains("*/");
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim_start().to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn export_bindings() {
        super::export_bindings(&super::builder());
    }

    #[test]
    fn strips_line_and_block_comments() {
        let source = "// header\n\n/** Commands */\nexport const a = 1;\n/*\n * multi\n */\nexport const b = \"//x\";\n";
        assert_eq!(
            super::without_comments(source),
            "export const a = 1;\nexport const b = \"//x\";\n"
        );
    }
}
