//! Tauri app: commands, events, windows and plugins (ARCHITECTURE §2, §7).

mod capabilities;
mod commands;
mod display_watch;
mod error;
mod events;
mod hotkeys;
mod hud;
mod ipc;
mod logging;
mod native_text;
mod overlay;
mod paths;
mod persist;
mod profiles;
mod state;
mod theme;
mod thumbs;
mod tokens;
mod tray;
mod tray_icon;
mod windowing;

use tauri::Manager;

/// Argument the autostart entry passes so the app opens in the tray.
const STARTED_MINIMIZED: &str = "--minimized";

/// "Ao fechar a janela" and "Minimizar para a bandeja" (T11.8).
fn on_main_event(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    event: &tauri::WindowEvent,
) {
    let settings = app.state::<state::AppState>().data().settings;
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            if settings.close_action == ch_core::model::CloseAction::Quit {
                api.prevent_close();
                tray::quit(app);
            } else {
                api.prevent_close();
                if settings.minimize_to_tray {
                    let _ = window.hide();
                } else {
                    let _ = window.minimize();
                }
            }
        }
        tauri::WindowEvent::Resized(_)
            if settings.minimize_to_tray && window.is_minimized().unwrap_or(false) =>
        {
            let _ = window.hide();
        }
        _ => {}
    }
}

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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![STARTED_MINIMIZED]),
        ))
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
            theme::apply(app.handle(), &state.data().settings);
            tray::install(app.handle())?;
            hud::create(app.handle())?;
            display_watch::spawn(app.handle().clone());
            // Autostart passes --minimized: stay in the tray (T11.5).
            let minimized = std::env::args().any(|a| a == STARTED_MINIMIZED);
            if let Some(main) = app.get_webview_window("main") {
                if !minimized {
                    let _ = main.show();
                }
                let handle = app.handle().clone();
                let window = main.clone();
                main.on_window_event(move |event| on_main_event(&handle, &window, event));
            }
            // "Seguir o Windows": repaint when the Windows app theme flips.
            if let Some(main) = app.get_webview_window("main") {
                let handle = app.handle().clone();
                main.on_window_event(move |event| {
                    if let tauri::WindowEvent::ThemeChanged(_) = event {
                        let state = handle.state::<state::AppState>();
                        theme::apply(&handle, &state.data().settings);
                    }
                });
            }
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
