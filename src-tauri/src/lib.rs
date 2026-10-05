//! Tauri app: commands, events, windows and plugins (ARCHITECTURE §2, §7).

mod capabilities;
mod commands;
mod error;
mod events;
mod hotkeys;
mod ipc;
mod logging;
mod overlay;
mod paths;
mod persist;
mod state;
#[allow(dead_code)] // consumed by the tray icons in Phase 11
mod tokens;
mod windowing;

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
        .register_uri_scheme_protocol(overlay::SCHEME, |ctx, request| {
            overlay::handle_protocol(ctx.app_handle(), request.uri().path())
        })
        .invoke_handler(specta.invoke_handler())
        .setup(move |app| {
            logging::install_panic_hook();
            specta.mount_events(app);
            let state = state::AppState::new(app.handle())?;
            app.manage(state);
            let state = app.state::<state::AppState>();
            state.hotkeys().apply(app.handle(), &state.data());
            state::measure_if_needed(app.handle());
            log::info!(
                "{} {} started (DPI: {})",
                ch_core::PRODUCT_NAME,
                app.package_info().version,
                commands::dpi_awareness()
            );
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start CH. Clicker")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = app.try_state::<state::AppState>()
            {
                state.engine.stop();
                state.persister.flush();
            }
        });
}
