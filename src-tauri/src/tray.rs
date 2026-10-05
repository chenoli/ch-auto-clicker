//! System tray (TASKS T11.6–T11.9, Tela 09): the state icon, the native menu and the shortcut
//! conflict notification. The menu is rebuilt whenever the state, profiles or settings change.
//! Limitation: the native Windows menu cannot reproduce the handoff's custom colors.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ch_core::engine::state::EngineState;
use ch_core::model::{Hotkey, Modifier, WindowPosition};
use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::native_text::{Text, text};
use crate::state::{AppState, emit_settings};
use crate::{theme, tray_icon};

const TRAY_ID: &str = "main";
/// Running pulse: the dot blinks every 700 ms (Tela 09.A).
const PULSE: Duration = Duration::from_millis(700);

static PULSE_ON: AtomicBool = AtomicBool::new(true);

/// Navigation request from the tray ("Configurações").
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    specta::Type,
    tauri_specta::Event,
)]
#[serde(rename_all = "camelCase")]
pub struct NavigateRequested {
    pub screen: String,
}

/// "Ctrl Alt Esc", for menu accelerator hints and notifications.
pub fn combo_label(hotkey: &Hotkey) -> String {
    let mut parts: Vec<&str> = hotkey
        .mods
        .iter()
        .map(|m| match m {
            Modifier::Ctrl => "Ctrl",
            Modifier::Alt => "Alt",
            Modifier::Shift => "Shift",
            Modifier::Win => "Win",
        })
        .collect();
    let key = match hotkey.key.as_str() {
        "Escape" => "Esc",
        "Left" => "←",
        "Right" => "→",
        "Up" => "↑",
        "Down" => "↓",
        "MouseMiddle" => "M3",
        other => other,
    };
    parts.push(key);
    parts.join(" ")
}

/// Pixel size for the tray at the main window's DPI: 16 px at 100 % up to 32 px at 200 %.
fn icon_size<R: Runtime>(app: &AppHandle<R>) -> u32 {
    let scale = app
        .get_webview_window("main")
        .and_then(|w| w.scale_factor().ok())
        .unwrap_or(1.0);
    match (16.0 * scale).round() as u32 {
        ..=17 => 16,
        18..=21 => 20,
        22..=27 => 24,
        _ => 32,
    }
}

fn icon<R: Runtime>(app: &AppHandle<R>, state: EngineState, pulse_on: bool) -> Image<'static> {
    let settings = app.state::<AppState>().data().settings;
    let palette = theme::palette(theme::resolve(&settings, theme::system_theme(app)));
    let color = match state {
        EngineState::Idle | EngineState::Stopping => palette.fg_3,
        EngineState::Countdown | EngineState::Paused => palette.warn,
        EngineState::Running => palette.accent,
        EngineState::Error => palette.danger,
    };
    let dot = if state == EngineState::Running && !pulse_on {
        None
    } else {
        tray_icon::rgb(color)
    };
    let size = icon_size(app);
    Image::new_owned(tray_icon::render(size, dot), size, size)
}

fn state_label(state: EngineState) -> Text {
    match state {
        EngineState::Idle | EngineState::Stopping => Text::StateIdle,
        EngineState::Countdown => Text::StateCountdown,
        EngineState::Running => Text::StateRunning,
        EngineState::Paused => Text::StatePaused,
        EngineState::Error => Text::StateError,
    }
}

fn with_hint(label: &str, hotkey: Option<&Hotkey>) -> String {
    hotkey.map_or_else(
        || label.to_owned(),
        |h| format!("{label}\t{}", combo_label(h)),
    )
}

fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let app_state = app.state::<AppState>();
    let data = app_state.data();
    let locale = data.settings.locale;
    let t = |key| text(locale, key);
    let engine = app_state.engine.shared().state();
    let active = engine.is_active();
    let bindings = &data.settings.hotkeys;
    let profile_name = data.active_profile().map_or("—", |p| p.name.as_str());

    let header = MenuItem::with_id(
        app,
        "header",
        format!("{profile_name} · {}", t(state_label(engine))),
        false,
        None::<&str>,
    )?;
    let start = MenuItem::with_id(
        app,
        "start",
        with_hint(
            t(if active { Text::Stop } else { Text::Start }),
            bindings.start_stop.as_ref(),
        ),
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        with_hint(
            t(if engine == EngineState::Paused {
                Text::Resume
            } else {
                Text::Pause
            }),
            bindings.pause_resume.as_ref(),
        ),
        matches!(engine, EngineState::Running | EngineState::Paused),
        None::<&str>,
    )?;
    let profiles = ch_core::profiles::ordered(&data.settings, &data.profiles);
    let items = profiles
        .iter()
        .map(|p| {
            CheckMenuItem::with_id(
                app,
                format!("profile:{}", p.id),
                &p.name,
                true,
                data.settings.active_profile_id.as_deref() == Some(p.id.as_str()),
                None::<&str>,
            )
        })
        .collect::<tauri::Result<Vec<_>>>()?;
    let item_refs: Vec<&dyn tauri::menu::IsMenuItem<R>> = items.iter().map(|i| i as _).collect();
    let profile_menu = Submenu::with_items(app, t(Text::Profile), true, &item_refs)?;
    let show = MenuItem::with_id(app, "show", t(Text::ShowWindow), true, None::<&str>)?;
    let hud = CheckMenuItem::with_id(
        app,
        "hud",
        t(Text::ShowHud),
        true,
        data.settings.show_hud,
        None::<&str>,
    )?;
    let hud_reset = MenuItem::with_id(
        app,
        "hud-reset",
        t(Text::ResetHudPosition),
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", t(Text::Settings), true, None::<&str>)?;
    let support = MenuItem::with_id(app, "support", t(Text::Support), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", t(Text::Quit), true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &header,
            &PredefinedMenuItem::separator(app)?,
            &start,
            &pause,
            &profile_menu,
            &PredefinedMenuItem::separator(app)?,
            &show,
            &hud,
            &hud_reset,
            &settings,
            &support,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

/// Shows and focuses the main window (tray click, "Mostrar janela").
pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

/// Stops a running engine and waits for it, so nothing clicks during an exit or an update.
pub fn stop_engine<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    if state.engine.shared().state().is_active() {
        state.engine.stop();
        let _ = state.engine.wait_idle(Duration::from_millis(1_000));
    }
    state.persister.flush();
}

/// Stops the engine safely, saves everything and exits ("Sair", or closing with "Sair").
pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    stop_engine(app);
    app.exit(0);
}

fn on_menu<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    let state = app.state::<AppState>();
    let id = event.id().as_ref();
    match id {
        "start" => {
            if state.engine.shared().state().is_active() {
                state.engine.stop();
            } else if let Err(e) = crate::commands::start_profile(&state, None) {
                if e.code == crate::commands::ANTI_CHEAT_ACK {
                    show_main(app);
                }
                let _ = crate::hotkeys::HotkeyActionFailed { code: e.code }.emit(app);
            }
        }
        "pause" => state.engine.toggle_pause(),
        "show" => show_main(app),
        "hud" => {
            let settings = state.with_data(|d| {
                d.settings.show_hud = !d.settings.show_hud;
                d.settings.clone()
            });
            state.persister.save_settings(&settings);
            emit_settings(app, &settings);
        }
        "hud-reset" => {
            let settings = state.with_data(|d| {
                d.settings.hud = WindowPosition { x: None, y: None };
                d.settings.clone()
            });
            state.persister.save_settings(&settings);
            crate::hud::place(app, &settings);
        }
        "settings" => {
            show_main(app);
            let _ = NavigateRequested {
                screen: "settings".into(),
            }
            .emit(app);
        }
        "support" => {
            use tauri_plugin_opener::OpenerExt;
            let _ = app
                .opener()
                .open_url(crate::commands::SUPPORT_URL, None::<&str>);
        }
        "quit" => quit(app),
        other => {
            if let Some(profile) = other.strip_prefix("profile:") {
                // Decision D12: switching stops the run and leaves the new profile idle.
                if state.engine.shared().state().is_active() {
                    state.engine.stop();
                }
                crate::hotkeys::switch_profile(app, &state, Some(profile));
            }
        }
    }
    refresh(app);
}

pub fn install<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon(app, EngineState::Idle, true))
        .tooltip(ch_core::PRODUCT_NAME)
        .menu(&build_menu(app)?)
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    spawn_pulse(app.clone());
    Ok(tray)
}

/// Re-reads state, profiles and settings into the icon and the menu.
pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let (Some(tray), Some(state)) = (app.tray_by_id(TRAY_ID), app.try_state::<AppState>()) else {
        return;
    };
    let engine = state.engine.shared().state();
    let _ = tray.set_icon(Some(icon(app, engine, PULSE_ON.load(Ordering::Relaxed))));
    if let Ok(menu) = build_menu(app) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn spawn_pulse<R: Runtime>(app: AppHandle<R>) {
    std::thread::Builder::new()
        .name("ch-tray-pulse".into())
        .spawn(move || {
            loop {
                std::thread::sleep(PULSE);
                let Some(state) = app.try_state::<AppState>() else {
                    continue;
                };
                if state.engine.shared().state() != EngineState::Running {
                    PULSE_ON.store(true, Ordering::Relaxed);
                    continue;
                }
                let on = !PULSE_ON.fetch_xor(true, Ordering::Relaxed);
                if let Some(tray) = app.tray_by_id(TRAY_ID) {
                    let _ = tray.set_icon(Some(icon(&app, EngineState::Running, on)));
                }
            }
        })
        .map(|_| ())
        .unwrap_or_else(|e| log::warn!("tray pulse thread: {e}"));
}

/// Native notification when Iniciar / Parar lost its shortcut to another app (Tela 09.B).
/// Fallback: toast actions need the installed app identity (Phase 12), so this toast only
/// informs; the main window shows the fix (decision D17).
pub fn notify_hotkey_taken<R: Runtime>(app: &AppHandle<R>, combo: &Hotkey) {
    use tauri_plugin_notification::NotificationExt;
    let locale = app.state::<AppState>().data().settings.locale;
    let _ = app
        .notification()
        .builder()
        .title(crate::native_text::hotkey_taken_title(
            locale,
            &combo_label(combo),
        ))
        .body(text(locale, Text::HotkeyTakenBody))
        .show();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combos_read_like_the_ui() {
        assert_eq!(combo_label(&Hotkey::new(&[], "F6")), "F6");
        assert_eq!(
            combo_label(&Hotkey::new(&[Modifier::Ctrl, Modifier::Alt], "Escape")),
            "Ctrl Alt Esc"
        );
        assert_eq!(
            combo_label(&Hotkey::new(&[Modifier::Shift], "Right")),
            "Shift →"
        );
        assert_eq!(
            with_hint("Iniciar", Some(&Hotkey::new(&[], "F6"))),
            "Iniciar\tF6"
        );
        assert_eq!(with_hint("Pausar", None), "Pausar");
    }

    #[test]
    fn every_state_has_a_label() {
        assert_eq!(state_label(EngineState::Stopping), Text::StateIdle);
        assert_eq!(state_label(EngineState::Error), Text::StateError);
        assert_eq!(state_label(EngineState::Countdown), Text::StateCountdown);
    }
}
