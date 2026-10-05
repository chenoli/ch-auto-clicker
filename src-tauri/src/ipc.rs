//! Typed command/event registry (tauri-specta) and TypeScript bindings export.

use tauri_specta::{Builder, collect_commands, collect_events};

use crate::{capabilities, commands, events};

const BINDINGS_PATH: &str = "../ui/src/ipc/bindings.ts";
const BINDINGS_HEADER: &str = "// Regenerate with `cargo test -p ch-clicker export_bindings`.";

pub fn builder() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new()
        .commands(collect_commands![
            commands::app_bootstrap,
            commands::engine_capabilities,
            commands::engine_run_benchmark,
            commands::debug_click_once
        ])
        .events(collect_events![
            events::EngineTick,
            events::EngineJitter,
            events::EngineDegraded,
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
