//! Capture overlays (TASKS T5.2–T5.5 and T7.3–T7.4, Telas 03.A/03.B): one always-on-top window
//! per monitor over a frozen capture of that monitor.
//!
//! - Ponto (D6): every click adds a point to the active profile right away; Enter or Esc ends the
//!   session, Ctrl+Z removes the last point.
//! - Área (D7): drag a rectangle inside one monitor; Enter saves it as the scan area (with a
//!   thumbnail), Esc cancels.

use std::sync::{Mutex, MutexGuard, mpsc};

use ch_core::geometry::{DisplayLayout, Monitor};
use ch_core::model::{Area, Position, SCAN_AREA_MIN, ScreenPoint};
use serde::{Deserialize, Serialize};
use tauri::http::{Response, StatusCode};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Runtime, WebviewUrl, WebviewWindowBuilder,
};
use tauri_specta::Event;

use crate::error::{AppError, AppResult};
use crate::state::{AppState, emit_profiles};
use crate::thumbs;

pub const SCHEME: &str = "chcap";

/// A monitor as the overlay page needs it (physical pixels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayMonitor {
    pub index: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub scale: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPoint {
    pub number: u32,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum OverlayMode {
    Points,
    Area,
}

/// Everything one overlay window renders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub monitor: OverlayMonitor,
    pub mode: OverlayMode,
    pub points: Vec<OverlayPoint>,
    /// Number the next click will get ("Clique para adicionar o ponto 05").
    pub next_number: u32,
    /// The profile's scan area (desktop pixels), shown pre-selected in area mode.
    pub area: Option<Area>,
}

/// The overlay mode changed (Área ↔ Ponto); every overlay window re-reads its state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct OverlayChanged {
    pub mode: OverlayMode,
}

pub struct Session {
    profile_id: String,
    mode: OverlayMode,
    monitors: Vec<Monitor>,
    fingerprint: String,
    captures: Vec<Vec<u8>>,
    added: Vec<String>,
    done: Option<mpsc::Sender<()>>,
}

#[derive(Default)]
pub struct OverlayManager {
    session: Mutex<Option<Session>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn label(index: usize) -> String {
    format!("overlay-{index}")
}

/// Physical desktop pixel for a CSS position inside overlay `monitor`.
pub fn to_physical(monitor: &Monitor, css_x: f64, css_y: f64) -> (i32, i32) {
    let scale = f64::from(monitor.scale);
    (
        monitor.rect.left + (css_x * scale).round() as i32,
        monitor.rect.top + (css_y * scale).round() as i32,
    )
}

impl OverlayManager {
    pub fn is_open(&self) -> bool {
        lock(&self.session).is_some()
    }

    /// BMP bytes of a monitor's capture, served through the `chcap` protocol.
    pub fn capture(&self, index: usize) -> Option<Vec<u8>> {
        lock(&self.session).as_ref()?.captures.get(index).cloned()
    }

    fn state_for(&self, app_state: &AppState, index: usize) -> AppResult<OverlayState> {
        let session = lock(&self.session);
        let session = session
            .as_ref()
            .ok_or_else(|| AppError::new("OVERLAY_CLOSED", "no capture session"))?;
        let monitor = session
            .monitors
            .get(index)
            .ok_or_else(|| AppError::new("OVERLAY_MONITOR", format!("no monitor {index}")))?;
        let data = app_state.data();
        let profile = data.profile(&session.profile_id);
        let points: Vec<OverlayPoint> = profile
            .map(|p| {
                p.modes
                    .click
                    .points
                    .iter()
                    .enumerate()
                    .map(|(i, pt)| OverlayPoint {
                        number: i as u32 + 1,
                        x: pt.x,
                        y: pt.y,
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(OverlayState {
            monitor: OverlayMonitor {
                index: index as u32,
                x: monitor.rect.left,
                y: monitor.rect.top,
                width: monitor.rect.width,
                height: monitor.rect.height,
                scale: f64::from(monitor.scale),
            },
            mode: session.mode,
            next_number: points.len() as u32 + 1,
            points,
            area: profile.and_then(|p| p.modes.scan.area),
        })
    }
}

/// The monitors and the arrangement fingerprint, for the points minimap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ScreenLayout {
    pub monitors: Vec<OverlayMonitor>,
    pub fingerprint: String,
}

pub fn screen_layout() -> ScreenLayout {
    let layout = display_layout();
    ScreenLayout {
        fingerprint: layout.fingerprint(),
        monitors: layout
            .monitors
            .iter()
            .enumerate()
            .map(|(index, m)| OverlayMonitor {
                index: index as u32,
                x: m.rect.left,
                y: m.rect.top,
                width: m.rect.width,
                height: m.rect.height,
                scale: f64::from(m.scale),
            })
            .collect(),
    }
}

/// Opens the overlays in `mode` and waits until the user finishes. Results are saved to the
/// profile as they happen (points) or on confirm (area), so the caller only learns that it ended.
pub async fn pick<R: Runtime>(app: AppHandle<R>, mode: OverlayMode) -> AppResult<()> {
    let rx = open(&app, mode)?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().unwrap_or_default())
        .await
        .map_err(|e| AppError::new("OVERLAY_FAILED", e.to_string()))
}

fn open<R: Runtime>(app: &AppHandle<R>, mode: OverlayMode) -> AppResult<mpsc::Receiver<()>> {
    let state = app.state::<AppState>();
    if state.overlays().is_open() {
        return Err(AppError::new(
            "OVERLAY_OPEN",
            "a capture session is already open",
        ));
    }
    // Picking points while the engine clicks would be chaos; the overlay always starts idle.
    if state.engine.shared().state().is_active() {
        state.engine.stop();
        let _ = state
            .engine
            .wait_idle(std::time::Duration::from_millis(500));
    }
    let profile_id = state
        .data()
        .settings
        .active_profile_id
        .ok_or_else(|| AppError::new("PROFILE_NOT_FOUND", "no active profile"))?;

    let layout = display_layout();
    let captures = layout
        .monitors
        .iter()
        .map(capture)
        .collect::<Result<Vec<_>, _>>()?;
    let (tx, rx) = mpsc::channel();
    *lock(&state.overlays().session) = Some(Session {
        profile_id,
        mode,
        fingerprint: layout.fingerprint(),
        monitors: layout.monitors.clone(),
        captures,
        added: Vec::new(),
        done: Some(tx),
    });

    let cursor = cursor_position();
    for (index, monitor) in layout.monitors.iter().enumerate() {
        if !overlay_allowed(monitor) {
            continue;
        }
        let url = WebviewUrl::App(format!("overlay.html?m={index}").into());
        let window = WebviewWindowBuilder::new(app, label(index), url)
            .title("CH. Clicker — Captura")
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .shadow(false)
            .visible(false)
            .background_color(tauri::window::Color(0x14, 0x14, 0x12, 0xFF))
            .build()
            .map_err(|e| AppError::new("OVERLAY_FAILED", e.to_string()))?;
        let r = monitor.rect;
        let _ = window.set_position(PhysicalPosition::new(r.left, r.top));
        let _ = window.set_size(PhysicalSize::new(r.width as u32, r.height as u32));
        let _ = window.show();
        if cursor.is_some_and(|c| r.contains(c)) {
            let _ = window.set_focus();
        }
        let close_app = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                finish(&close_app);
            }
        });
    }
    Ok(rx)
}

/// Debug builds honour `CH_TEST_MONITORS` (`primary` or a comma list of device names), so a
/// developer with games on other screens can try the overlay without covering them.
fn overlay_allowed(monitor: &Monitor) -> bool {
    if !cfg!(debug_assertions) {
        return true;
    }
    monitor_filter_allows(std::env::var("CH_TEST_MONITORS").ok().as_deref(), monitor)
}

fn monitor_filter_allows(filter: Option<&str>, monitor: &Monitor) -> bool {
    match filter.map(str::trim) {
        None | Some("") => true,
        Some("primary") => monitor.primary,
        Some(list) => list
            .split(',')
            .any(|name| name.trim().eq_ignore_ascii_case(&monitor.id)),
    }
}

/// Closes every overlay window, drops the captures and resolves the waiting command.
pub fn finish<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let Some(mut session) = lock(&state.overlays().session).take() else {
        return;
    };
    for index in 0..session.monitors.len() {
        if let Some(window) = app.get_webview_window(&label(index)) {
            let _ = window.destroy();
        }
    }
    if let Some(done) = session.done.take() {
        let _ = done.send(());
    }
}

/// Área ↔ Ponto from the overlay's top bar.
pub fn set_mode<R: Runtime>(app: &AppHandle<R>, mode: OverlayMode) -> AppResult<()> {
    let state = app.state::<AppState>();
    lock(&state.overlays().session)
        .as_mut()
        .ok_or_else(|| AppError::new("OVERLAY_CLOSED", "no capture session"))?
        .mode = mode;
    let _ = OverlayChanged { mode }.emit(app);
    Ok(())
}

/// Desktop rectangle for two CSS corners inside `monitor`, kept on that monitor (D7).
pub fn area_from_css(monitor: &Monitor, x: f64, y: f64, w: f64, h: f64) -> Option<Area> {
    let r = monitor.rect;
    let (left, top) = to_physical(monitor, x.min(x + w), y.min(y + h));
    let (right, bottom) = to_physical(monitor, x.max(x + w), y.max(y + h));
    let left = left.clamp(r.left, r.right() - 1);
    let top = top.clamp(r.top, r.bottom() - 1);
    let right = right.clamp(left, r.right());
    let bottom = bottom.clamp(top, r.bottom());
    let area = Area {
        x: left,
        y: top,
        w: u32::try_from(right - left).ok()?,
        h: u32::try_from(bottom - top).ok()?,
    };
    (area.w >= SCAN_AREA_MIN && area.h >= SCAN_AREA_MIN).then_some(area)
}

/// Saves the rectangle drawn on overlay `index` as the scan area, writes its thumbnail and closes
/// the overlays.
pub fn confirm_area<R: Runtime>(
    app: &AppHandle<R>,
    index: usize,
    rect: (f64, f64, f64, f64),
) -> AppResult<Area> {
    let state = app.state::<AppState>();
    let (profile_id, area, thumb) = {
        let session = lock(&state.overlays().session);
        let session = session
            .as_ref()
            .ok_or_else(|| AppError::new("OVERLAY_CLOSED", "no capture session"))?;
        let monitor = session
            .monitors
            .get(index)
            .ok_or_else(|| AppError::new("OVERLAY_MONITOR", format!("no monitor {index}")))?;
        let area = area_from_css(monitor, rect.0, rect.1, rect.2, rect.3).ok_or_else(|| {
            AppError::new(
                "SCAN_AREA_TOO_SMALL",
                format!("the area must be at least {SCAN_AREA_MIN} px on each side"),
            )
        })?;
        let thumb = session.captures.get(index).and_then(|bmp| {
            thumbs::crop_scaled(
                bmp,
                (area.x - monitor.rect.left) as u32,
                (area.y - monitor.rect.top) as u32,
                area.w,
                area.h,
                thumbs::MAX_W,
                thumbs::MAX_H,
            )
        });
        (session.profile_id.clone(), area, thumb)
    };
    // The thumbnail is a nicety: the area is saved even if it cannot be written.
    if let Some(bytes) = thumb
        && let Err(e) = thumbs::save(&profile_id, &bytes)
    {
        log::warn!("could not write the scan thumbnail: {e}");
    }
    update_profile(app, &profile_id, |profile| {
        profile.modes.scan.area = Some(area);
    })?;
    finish(app);
    Ok(area)
}

pub fn overlay_state<R: Runtime>(app: &AppHandle<R>, index: usize) -> AppResult<OverlayState> {
    let state = app.state::<AppState>();
    state.overlays().state_for(&state, index)
}

/// Adds a point at a CSS position of overlay `index`, saves the profile and refreshes every window.
pub fn add_point<R: Runtime>(
    app: &AppHandle<R>,
    index: usize,
    css_x: f64,
    css_y: f64,
) -> AppResult<OverlayState> {
    let state = app.state::<AppState>();
    let (profile_id, point, fingerprint) = {
        let session = lock(&state.overlays().session);
        let session = session
            .as_ref()
            .ok_or_else(|| AppError::new("OVERLAY_CLOSED", "no capture session"))?;
        let monitor = session
            .monitors
            .get(index)
            .ok_or_else(|| AppError::new("OVERLAY_MONITOR", format!("no monitor {index}")))?;
        let (x, y) = to_physical(monitor, css_x, css_y);
        let point = ScreenPoint {
            id: uuid::Uuid::new_v4().to_string(),
            x,
            y,
            delay_ms: 0,
        };
        (
            session.profile_id.clone(),
            point,
            session.fingerprint.clone(),
        )
    };
    let id = point.id.clone();
    update_points(app, &profile_id, move |points, profile_fingerprint| {
        if points.len() < ch_core::model::POINTS_MAX {
            points.push(point);
            *profile_fingerprint = Some(fingerprint);
        }
    })?;
    if let Some(session) = lock(&state.overlays().session).as_mut() {
        session.added.push(id);
    }
    state.overlays().state_for(&state, index)
}

/// Removes the last point added in this session (Ctrl+Z).
pub fn undo<R: Runtime>(app: &AppHandle<R>) -> AppResult<()> {
    let state = app.state::<AppState>();
    let (profile_id, last) = {
        let mut session = lock(&state.overlays().session);
        let session = session
            .as_mut()
            .ok_or_else(|| AppError::new("OVERLAY_CLOSED", "no capture session"))?;
        (session.profile_id.clone(), session.added.pop())
    };
    if let Some(id) = last {
        update_points(app, &profile_id, move |points, _| {
            points.retain(|p| p.id != id)
        })?;
    }
    Ok(())
}

fn update_points<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    mutate: impl FnOnce(&mut Vec<ScreenPoint>, &mut Option<String>),
) -> AppResult<()> {
    update_profile(app, profile_id, |profile| {
        mutate(
            &mut profile.modes.click.points,
            &mut profile.modes.click.layout_fingerprint,
        );
        profile.modes.click.position = Position::Points;
    })
}

fn update_profile<R: Runtime>(
    app: &AppHandle<R>,
    profile_id: &str,
    mutate: impl FnOnce(&mut ch_core::model::Profile),
) -> AppResult<()> {
    let state = app.state::<AppState>();
    let (profile, data) = state.with_data(|data| {
        let profile = data
            .profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| AppError::new("PROFILE_NOT_FOUND", "profile is gone"))?;
        mutate(profile);
        profile.updated_at = crate::commands::next_version();
        Ok::<_, AppError>((profile.clone(), data.clone()))
    })?;
    state.persister.save_profile(&profile);
    emit_profiles(app, &data);
    Ok(())
}

/// Serves `chcap://localhost/<monitor>.bmp` from the open session and
/// `chcap://localhost/thumb/<profile>.bmp` from the thumbnail cache.
pub fn handle_protocol<R: Runtime>(app: &AppHandle<R>, path: &str) -> Response<Vec<u8>> {
    let path = path.trim_start_matches('/');
    let body = if let Some(id) = path
        .strip_prefix("thumb/")
        .and_then(|p| p.strip_suffix(".bmp"))
    {
        thumbs::path_for(id).and_then(|file| std::fs::read(file).ok())
    } else {
        path.strip_suffix(".bmp")
            .and_then(|i| i.parse::<usize>().ok())
            .and_then(|i| app.state::<AppState>().overlays().capture(i))
    };
    let builder = Response::builder()
        // The loupe reads pixels through a canvas; the page origin differs from this scheme's.
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-store");
    match body {
        Some(bytes) => builder.header("Content-Type", "image/bmp").body(bytes),
        None => builder.status(StatusCode::NOT_FOUND).body(Vec::new()),
    }
    .unwrap_or_else(|_| Response::new(Vec::new()))
}

#[cfg(windows)]
fn display_layout() -> DisplayLayout {
    ch_win::display::display_layout()
}

#[cfg(not(windows))]
fn display_layout() -> DisplayLayout {
    DisplayLayout {
        virtual_desktop: ch_core::geometry::Rect::new(0, 0, 0, 0),
        monitors: Vec::new(),
    }
}

#[cfg(windows)]
fn capture(monitor: &Monitor) -> AppResult<Vec<u8>> {
    ch_win::capture::capture_bmp(monitor.rect)
        .map_err(|e| AppError::new("CAPTURE_FAILED", e.to_string()))
}

#[cfg(not(windows))]
fn capture(_monitor: &Monitor) -> AppResult<Vec<u8>> {
    Err(AppError::new(
        "CAPTURE_FAILED",
        "screen capture needs Windows",
    ))
}

#[cfg(windows)]
fn cursor_position() -> Option<ch_core::geometry::Point> {
    use ch_win::input::InputSink;
    ch_win::input::SystemSink.cursor_pos()
}

#[cfg(not(windows))]
fn cursor_position() -> Option<ch_core::geometry::Point> {
    None
}

#[cfg(test)]
mod tests {
    use ch_core::geometry::Rect;

    use super::*;

    fn monitor(left: i32, scale: f32) -> Monitor {
        Monitor {
            id: "m".into(),
            rect: Rect::new(left, 0, 1920, 1080),
            work_area: Rect::new(left, 0, 1920, 1040),
            scale,
            primary: left == 0,
        }
    }

    #[test]
    fn converts_css_positions_to_desktop_pixels() {
        assert_eq!(to_physical(&monitor(0, 1.0), 412.0, 236.0), (412, 236));
        assert_eq!(
            to_physical(&monitor(1920, 1.5), 100.0, 200.0),
            (2070, 300),
            "scaled second monitor"
        );
        assert_eq!(
            to_physical(&monitor(-1680, 1.25), 10.4, 0.0),
            (-1667, 0),
            "negative origin, rounded"
        );
    }

    #[test]
    fn test_monitor_filter_keeps_other_screens_free() {
        let primary = monitor(0, 1.0);
        let mut second = monitor(1920, 1.0);
        second.id = r"\\.\DISPLAY2".into();
        assert!(monitor_filter_allows(None, &second));
        assert!(monitor_filter_allows(Some(" "), &second));
        assert!(monitor_filter_allows(Some("primary"), &primary));
        assert!(!monitor_filter_allows(Some("primary"), &second));
        assert!(monitor_filter_allows(
            Some(r"\\.\display1, \\.\DISPLAY2"),
            &second
        ));
        assert!(!monitor_filter_allows(Some(r"\\.\DISPLAY1"), &second));
    }

    #[test]
    fn areas_stay_on_their_monitor_and_have_a_minimum_size() {
        let m = monitor(1920, 1.5);
        let a = area_from_css(&m, 100.0, 50.0, 200.0, 100.0).expect("area");
        assert_eq!((a.x, a.y, a.w, a.h), (2070, 75, 300, 150));
        // Dragged up-left: corners are normalised.
        let b = area_from_css(&m, 300.0, 150.0, -200.0, -100.0).expect("area");
        assert_eq!(a, b);
        // Past the right edge: clamped to the monitor.
        let c = area_from_css(&m, 1200.0, 0.0, 400.0, 100.0).expect("area");
        assert_eq!(c.x + c.w as i32, 1920 + 1920);
        assert!(
            area_from_css(&m, 0.0, 0.0, 5.0, 100.0).is_none(),
            "too narrow"
        );
    }

    #[test]
    fn labels_are_per_monitor() {
        assert_eq!(label(0), "overlay-0");
        assert_eq!(label(2), "overlay-2");
        assert!(!OverlayManager::default().is_open());
        assert!(OverlayManager::default().capture(0).is_none());
    }
}
