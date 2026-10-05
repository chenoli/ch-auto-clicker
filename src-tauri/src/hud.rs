//! Mini HUD window (TASKS T11.10–T11.12, Tela 04, decision D5): an always-on-top pill that
//! clicks pass through and that never takes focus. It fades to 40 % under the cursor; holding
//! Ctrl over it makes it draggable, and the position is saved.

use std::time::Duration;

use ch_core::model::{Settings, WindowPosition};
use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};
use tauri_specta::Event;

use crate::state::AppState;

pub const LABEL: &str = "hud";
/// Room for the widest state (Rodando with counter, remaining time and chip) at any locale.
const DEFAULT_SIZE: (f64, f64) = (460.0, 56.0);
/// Gap to the work area's top-right corner.
const MARGIN: i32 = 16;
const POLL: Duration = Duration::from_millis(100);

/// Cursor over the HUD (it fades) and whether Ctrl currently makes it draggable.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    specta::Type,
    tauri_specta::Event,
)]
#[serde(rename_all = "camelCase")]
pub struct HudHover {
    pub hover: bool,
    pub draggable: bool,
}

fn window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(LABEL)
}

/// Creates the hidden HUD window once, at startup.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let (w, h) = DEFAULT_SIZE;
    let hud = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("hud.html".into()))
        .title(ch_core::PRODUCT_NAME)
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .visible(false)
        .build()?;
    hud.set_ignore_cursor_events(true)?;
    #[cfg(windows)]
    if let Ok(hwnd) = hud.hwnd() {
        ch_win::window::set_no_activate(hwnd.0 as isize);
    }
    let handle = app.clone();
    hud.on_window_event(move |event| {
        if let tauri::WindowEvent::Moved(pos) = event {
            save_position(&handle, *pos);
        }
    });
    place(app, &app.state::<AppState>().data().settings);
    spawn_poll(app.clone());
    Ok(())
}

/// Default spot: top-right corner of the primary monitor's work area.
fn default_position<R: Runtime>(
    app: &AppHandle<R>,
    hud: &WebviewWindow<R>,
) -> Option<PhysicalPosition<i32>> {
    let monitor = app.primary_monitor().ok().flatten()?;
    let area = monitor.work_area();
    let width = hud.outer_size().ok()?.width as i32;
    Some(PhysicalPosition::new(
        area.position.x + area.size.width as i32 - width - MARGIN,
        area.position.y + MARGIN,
    ))
}

/// Puts the HUD where the user left it, or in the default corner.
pub fn place<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    let Some(hud) = window(app) else {
        return;
    };
    let pos = match (settings.hud.x, settings.hud.y) {
        (Some(x), Some(y)) => Some(PhysicalPosition::new(x, y)),
        _ => default_position(app, &hud),
    };
    if let Some(pos) = pos {
        let _ = hud.set_position(pos);
    }
}

fn save_position<R: Runtime>(app: &AppHandle<R>, pos: PhysicalPosition<i32>) {
    let state = app.state::<AppState>();
    // Only moves made by the user (Ctrl + drag) are saved; programmatic placement is skipped.
    if !state.hud_dragging() {
        return;
    }
    let settings = state.with_data(|d| {
        d.settings.hud = WindowPosition {
            x: Some(pos.x),
            y: Some(pos.y),
        };
        d.settings.clone()
    });
    state.persister.save_settings(&settings);
}

/// Shows the HUD while a run is active and "Mostrar HUD" is on; hides it otherwise.
pub fn sync<R: Runtime>(app: &AppHandle<R>) {
    let (Some(hud), Some(state)) = (window(app), app.try_state::<AppState>()) else {
        return;
    };
    let settings = state.data().settings;
    let want = settings.show_hud && state.engine.shared().state().is_active();
    let visible = hud.is_visible().unwrap_or(false);
    if want && !visible {
        // D5: the "Segure Ctrl para mover" hint shows the first 3 times.
        if settings.hud_hints_shown < 3 {
            let saved = state.with_data(|d| {
                d.settings.hud_hints_shown += 1;
                d.settings.clone()
            });
            state.persister.save_settings(&saved);
            crate::state::emit_settings(app, &saved);
        }
        let _ = hud.show();
    } else if !want && visible {
        let _ = hud.hide();
    }
}

/// The HUD page reports the pill's size; the window follows it, keeping the right edge.
#[tauri::command]
#[specta::specta]
pub fn hud_resize(app: AppHandle, width: f64, height: f64) {
    let Some(hud) = window(&app) else {
        return;
    };
    let (Ok(pos), Ok(size), Ok(scale)) =
        (hud.outer_position(), hud.outer_size(), hud.scale_factor())
    else {
        return;
    };
    let right = pos.x + size.width as i32;
    let _ = hud.set_size(LogicalSize::new(width.max(1.0), height.max(1.0)));
    let new_width = (width * scale).round() as i32;
    let _ = hud.set_position(PhysicalPosition::new(right - new_width, pos.y));
}

/// Hover fade and Ctrl-drag: the HUD only accepts the mouse while Ctrl is held over it.
fn spawn_poll<R: Runtime>(app: AppHandle<R>) {
    let spawn = std::thread::Builder::new()
        .name("ch-hud-poll".into())
        .spawn(move || {
            let mut last = HudHover {
                hover: false,
                draggable: false,
            };
            loop {
                std::thread::sleep(POLL);
                let Some(hud) = window(&app) else {
                    continue;
                };
                if !hud.is_visible().unwrap_or(false) {
                    continue;
                }
                let now = hover_state(&hud);
                if now != last {
                    let _ = hud.set_ignore_cursor_events(!now.draggable);
                    if let Some(state) = app.try_state::<AppState>() {
                        state.set_hud_dragging(now.draggable);
                    }
                    let _ = now.emit(&app);
                    last = now;
                }
            }
        });
    if let Err(e) = spawn {
        log::warn!("HUD poll thread: {e}");
    }
}

#[cfg(windows)]
fn hover_state<R: Runtime>(hud: &WebviewWindow<R>) -> HudHover {
    use ch_win::input::InputSink;
    let (Some(cursor), Ok(pos), Ok(size)) = (
        ch_win::input::SystemSink.cursor_pos(),
        hud.outer_position(),
        hud.outer_size(),
    ) else {
        return HudHover {
            hover: false,
            draggable: false,
        };
    };
    let hover = cursor.x >= pos.x
        && cursor.y >= pos.y
        && cursor.x < pos.x + size.width as i32
        && cursor.y < pos.y + size.height as i32;
    HudHover {
        hover,
        draggable: hover && ch_win::window::ctrl_down(),
    }
}

#[cfg(not(windows))]
fn hover_state<R: Runtime>(_hud: &WebviewWindow<R>) -> HudHover {
    HudHover {
        hover: false,
        draggable: false,
    }
}
