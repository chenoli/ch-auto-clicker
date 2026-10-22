use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use ch_core::engine::state::EngineState;
use ch_core::model::{Hotkey, Modifier, WindowPosition};
use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::native_text::{Text, text};
use crate::state::{AppState, emit_settings};
use crate::{theme, tray_icon};

const TRAY_ID: &str = "main";
const PULSE: Duration = Duration::from_millis(700);

static PULSE_ON: AtomicBool = AtomicBool::new(true);

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
    let settings = app.state::<AppState>().settings();
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

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    crate::windowing::show_main(app);
}

pub fn stop_engine<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    if state.engine.shared().state().is_active() {
        state.engine.stop();
        let _ = state.engine.wait_idle(Duration::from_millis(1_000));
    }
    state.persister.flush();
}

pub fn quit<R: Runtime>(app: &AppHandle<R>) {
    stop_engine(app);
    app.exit(0);
}

pub fn run_action<R: Runtime>(app: &AppHandle<R>, id: &str) {
    let state = app.state::<AppState>();
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
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button,
                button_state: MouseButtonState::Up,
                position,
                ..
            } = event
            {
                match button {
                    MouseButton::Left => show_main(tray.app_handle()),
                    MouseButton::Right => crate::tray_menu::open(tray.app_handle(), position),
                    MouseButton::Middle => {}
                }
            }
        })
        .build(app)?;
    spawn_pulse(app.clone());
    Ok(tray)
}

pub fn refresh<R: Runtime>(app: &AppHandle<R>) {
    let (Some(tray), Some(state)) = (app.tray_by_id(TRAY_ID), app.try_state::<AppState>()) else {
        return;
    };
    let engine = state.engine.shared().state();
    let _ = tray.set_icon(Some(icon(app, engine, PULSE_ON.load(Ordering::Relaxed))));
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

pub fn notify_hotkey_taken<R: Runtime>(app: &AppHandle<R>, combo: &Hotkey) {
    use tauri_plugin_notification::NotificationExt;
    let locale = app.state::<AppState>().settings().locale;
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
    }
}
