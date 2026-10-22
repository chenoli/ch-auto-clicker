use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, LogicalSize, Manager, PhysicalPosition, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, WindowEvent,
};
use tauri_specta::Event;

pub const LABEL: &str = "tray-menu";
const TITLE: &str = "CH Auto Clicker menu";
const DEFAULT_SIZE: (f64, f64) = (498.0, 470.0);
const EDGE: f64 = 16.0;
const REOPEN_GUARD: Duration = Duration::from_millis(300);

static SIZE: Mutex<(f64, f64)> = Mutex::new(DEFAULT_SIZE);
static HIDDEN_AT: Mutex<Option<Instant>> = Mutex::new(None);

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
pub struct TrayMenuOpened;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

pub fn place(anchor: (i32, i32), size: (i32, i32), edge: i32, work: Rect) -> (i32, i32) {
    let (w, h) = size;
    let mut x = anchor.0 - w + edge;
    let mut y = anchor.1 - h + edge;
    if y < work.top {
        y = anchor.1 - edge;
    }
    x = x.clamp(work.left, (work.left + work.width - w).max(work.left));
    y = y.clamp(work.top, (work.top + work.height - h).max(work.top));
    (x, y)
}

fn window<R: Runtime>(app: &AppHandle<R>) -> Option<WebviewWindow<R>> {
    app.get_webview_window(LABEL)
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let (w, h) = DEFAULT_SIZE;
    let menu = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("tray-menu.html".into()))
        .title(TITLE)
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
    let handle = app.clone();
    menu.on_window_event(move |event| {
        if matches!(event, WindowEvent::Focused(false)) {
            hide(&handle);
        }
    });
    Ok(())
}

pub fn open<R: Runtime>(app: &AppHandle<R>, anchor: PhysicalPosition<f64>) {
    if lock(&HIDDEN_AT).is_some_and(|at| at.elapsed() < REOPEN_GUARD) {
        return;
    }
    let Some(menu) = window(app) else {
        return;
    };
    if menu.is_visible().unwrap_or(false) {
        hide(app);
        return;
    }
    let Ok(Some(monitor)) = app.monitor_from_point(anchor.x, anchor.y) else {
        return;
    };
    let scale = monitor.scale_factor();
    let (w, h) = *lock(&SIZE);
    let size = ((w * scale).round() as i32, (h * scale).round() as i32);
    let area = monitor.work_area();
    let work = Rect {
        left: area.position.x,
        top: area.position.y,
        width: area.size.width as i32,
        height: area.size.height as i32,
    };
    let (x, y) = place(
        (anchor.x.round() as i32, anchor.y.round() as i32),
        size,
        (EDGE * scale).round() as i32,
        work,
    );
    let _ = menu.set_size(LogicalSize::new(w, h));
    let _ = menu.set_position(PhysicalPosition::new(x, y));
    let _ = menu.show();
    let _ = TrayMenuOpened.emit(app);
    crate::windowing::bring_to_front(&menu);
}

pub fn hide<R: Runtime>(app: &AppHandle<R>) {
    if let Some(menu) = window(app)
        && menu.is_visible().unwrap_or(false)
    {
        let _ = menu.hide();
        *lock(&HIDDEN_AT) = Some(Instant::now());
    }
}

pub fn resize<R: Runtime>(app: &AppHandle<R>, width: f64, height: f64) {
    *lock(&SIZE) = (width, height);
    if let Some(menu) = window(app) {
        let _ = menu.set_size(LogicalSize::new(width, height));
    }
}

#[tauri::command]
#[specta::specta]
pub fn tray_menu_resize(app: AppHandle, width: f64, height: f64) {
    resize(&app, width.max(1.0), height.max(1.0));
}

#[tauri::command]
#[specta::specta]
pub fn tray_menu_close(app: AppHandle) {
    hide(&app);
}

#[tauri::command]
#[specta::specta]
pub fn tray_menu_action(app: AppHandle, action: String) {
    hide(&app);
    crate::tray::run_action(&app, &action);
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: Rect = Rect {
        left: 0,
        top: 0,
        width: 1920,
        height: 1040,
    };

    #[test]
    fn opens_above_and_left_of_a_taskbar_click() {
        assert_eq!(place((1800, 1060), (500, 470), 16, WORK), (1316, 570));
    }

    #[test]
    fn opens_below_the_cursor_when_the_taskbar_is_on_top() {
        let top_bar = Rect { top: 40, ..WORK };
        assert_eq!(place((1800, 20), (500, 470), 16, top_bar), (1316, 40));
        assert_eq!(place((1800, 300), (500, 470), 16, WORK), (1316, 284));
    }

    #[test]
    fn stays_inside_the_work_area_of_any_monitor() {
        let left_monitor = Rect {
            left: -1680,
            top: 0,
            width: 1680,
            height: 1010,
        };
        assert_eq!(
            place((-1670, 1000), (500, 470), 16, left_monitor),
            (-1680, 540)
        );
        let tiny = Rect {
            left: 0,
            top: 0,
            width: 300,
            height: 200,
        };
        assert_eq!(place((10, 10), (500, 470), 16, tiny), (0, 0));
    }
}
