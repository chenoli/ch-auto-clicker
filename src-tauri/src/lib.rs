//! Tauri app: commands, events, windows and plugins (ARCHITECTURE §2, §7).

mod commands;
mod events;
mod ipc;
mod logging;
#[allow(dead_code)] // consumed by the tray icons in Phase 11
mod tokens;

use tauri::Manager;

pub fn run() {
    let specta = ipc::builder();
    #[cfg(debug_assertions)]
    ipc::export_bindings(&specta);

    tauri::Builder::default()
        // Must be the first plugin so a second instance exits before anything else initializes.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(logging::plugin())
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            logging::install_panic_hook();
            specta.mount_events(app);
            log::info!(
                "{} {} started (DPI: {})",
                ch_core::PRODUCT_NAME,
                app.package_info().version,
                commands::dpi_awareness()
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start CH. Clicker");
}
