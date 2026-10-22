use std::sync::{Mutex, mpsc};
use std::time::{Duration, Instant};

use ch_core::easing::EASE_OUT;
use ch_core::geometry::{Point, Rect};
use ch_core::model::WindowPosition;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, PhysicalPosition, PhysicalSize, Runtime,
    WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

pub const MAIN_WIDTH: f64 = 420.0;
pub const MIN_HEIGHT: f64 = 640.0;
pub const MAX_HEIGHT: f64 = 900.0;
const DURATION: Duration = Duration::from_millis(320);
const FRAME: Duration = Duration::from_millis(16);
const VARIATION_LABEL: &str = "variation";
const VARIATION_SIZE: (f64, f64) = (560.0, 640.0);
const REACHABLE_WIDTH: i32 = 96;
const REACHABLE_HEIGHT: i32 = 24;

pub fn clamp_height(requested: f64, space_below_top: Option<f64>) -> f64 {
    let ceiling = space_below_top.map_or(MAX_HEIGHT, |s| s.min(MAX_HEIGHT));
    requested.clamp(
        MIN_HEIGHT.min(ceiling),
        ceiling.max(MIN_HEIGHT.min(ceiling)),
    )
}

pub fn height_at(from: f64, to: f64, elapsed: Duration) -> f64 {
    let progress = elapsed.as_secs_f64() / DURATION.as_secs_f64();
    from + (to - from) * EASE_OUT.at(progress)
}

#[derive(Debug)]
pub struct HeightAnimator {
    tx: mpsc::Sender<f64>,
    last: Mutex<Option<f64>>,
}

impl HeightAnimator {
    pub fn spawn<R: Runtime>(window: WebviewWindow<R>) -> Self {
        let (tx, rx) = mpsc::channel::<f64>();
        std::thread::Builder::new()
            .name("ch-window-height".into())
            .spawn(move || {
                let scale = window.scale_factor().unwrap_or(1.0);
                let mut current = window
                    .inner_size()
                    .map_or(MIN_HEIGHT, |s| f64::from(s.height) / scale);
                while let Ok(mut target) = rx.recv() {
                    let mut from = current;
                    let mut started = Instant::now();
                    loop {
                        let elapsed = started.elapsed();
                        current = if elapsed >= DURATION {
                            target
                        } else {
                            height_at(from, target, elapsed)
                        };
                        let _ = window.set_size(LogicalSize::new(MAIN_WIDTH, current));
                        if elapsed >= DURATION {
                            break;
                        }
                        match rx.recv_timeout(FRAME) {
                            Ok(next) => {
                                target = next;
                                from = current;
                                started = Instant::now();
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                }
            })
            .expect("failed to spawn the window animator");
        Self {
            tx,
            last: Mutex::new(None),
        }
    }

    pub fn animate_to(&self, height: f64) {
        if let Ok(mut last) = self.last.lock() {
            *last = Some(height);
        }
        let _ = self.tx.send(height);
    }

    pub fn last_target(&self) -> Option<f64> {
        self.last.lock().ok().and_then(|last| *last)
    }
}

fn overlap(a: &Rect, b: &Rect) -> (i32, i32) {
    let width = a.right().min(b.right()) - a.left.max(b.left);
    let height = a.bottom().min(b.bottom()) - a.top.max(b.top);
    (width.max(0), height.max(0))
}

pub fn rescue_rect(
    window: Rect,
    collapsed_below: (i32, i32),
    desired: (i32, i32),
    work_areas: &[Rect],
    home: Rect,
) -> Option<Rect> {
    let collapsed = window.width < collapsed_below.0 || window.height < collapsed_below.1;
    let top_band = Rect::new(
        window.left,
        window.top,
        window.width,
        REACHABLE_HEIGHT.min(window.height),
    );
    let reachable = work_areas.iter().any(|work| {
        let (w, h) = overlap(&top_band, work);
        w >= REACHABLE_WIDTH.min(window.width) && h >= top_band.height
    });
    if !collapsed && reachable {
        return None;
    }
    let (width, height) = (desired.0.min(home.width), desired.1.min(home.height));
    Some(Rect::new(
        home.left + (home.width - width) / 2,
        home.top + (home.height - height) / 2,
        width,
        height,
    ))
}

fn work_rect(monitor: &tauri::Monitor) -> Rect {
    let area = monitor.work_area();
    Rect::new(
        area.position.x,
        area.position.y,
        area.size.width as i32,
        area.size.height as i32,
    )
}

pub fn ensure_on_screen<R: Runtime>(window: &WebviewWindow<R>, height: f64) {
    let (Ok(pos), Ok(size), Ok(monitors)) = (
        window.outer_position(),
        window.inner_size(),
        window.available_monitors(),
    ) else {
        return;
    };
    let Some(home) = window
        .primary_monitor()
        .ok()
        .flatten()
        .or_else(|| monitors.first().cloned())
    else {
        return;
    };
    let scale = home.scale_factor();
    let physical = |v: f64| (v * scale).round() as i32;
    let current = Rect::new(pos.x, pos.y, size.width as i32, size.height as i32);
    let works: Vec<Rect> = monitors.iter().map(work_rect).collect();
    let Some(target) = rescue_rect(
        current,
        (physical(MAIN_WIDTH / 2.0), physical(MIN_HEIGHT / 2.0)),
        (physical(MAIN_WIDTH), physical(height)),
        &works,
        work_rect(&home),
    ) else {
        return;
    };
    log::warn!("main window unreachable at {current:?}, moved to {target:?}");
    let _ = window.set_size(PhysicalSize::new(target.width as u32, target.height as u32));
    let _ = window.set_position(PhysicalPosition::new(target.left, target.top));
}

pub fn inside_any(areas: &[Rect], p: Point) -> bool {
    areas.iter().any(|area| area.contains(p))
}

pub fn on_a_monitor<R: Runtime>(app: &AppHandle<R>, x: i32, y: i32) -> bool {
    app.available_monitors().is_ok_and(|monitors| {
        let areas: Vec<Rect> = monitors
            .iter()
            .map(|m| {
                let (p, s) = (m.position(), m.size());
                Rect::new(p.x, p.y, s.width as i32, s.height as i32)
            })
            .collect();
        inside_any(&areas, Point::new(x, y))
    })
}

pub fn restore_main_position<R: Runtime>(window: &WebviewWindow<R>, saved: WindowPosition) {
    if let (Some(x), Some(y)) = (saved.x, saved.y)
        && on_a_monitor(window.app_handle(), x, y)
    {
        let _ = window.set_position(PhysicalPosition::new(x, y));
        ensure_on_screen(window, MIN_HEIGHT);
    }
}

pub fn save_main_position<R: Runtime>(app: &AppHandle<R>, pos: PhysicalPosition<i32>) {
    if !on_a_monitor(app, pos.x, pos.y) {
        return;
    }
    let Some(state) = app.try_state::<crate::state::AppState>() else {
        return;
    };
    let next = WindowPosition {
        x: Some(pos.x),
        y: Some(pos.y),
    };
    let changed = state.with_data(|d| {
        (d.settings.window != next).then(|| {
            d.settings.window = next;
            d.settings.clone()
        })
    });
    if let Some(settings) = changed {
        state.persister.save_settings(&settings);
    }
}

fn main_height<R: Runtime>(app: &AppHandle<R>) -> f64 {
    app.try_state::<crate::state::AppState>()
        .and_then(|state| state.animator.get().and_then(HeightAnimator::last_target))
        .unwrap_or(MIN_HEIGHT)
}

pub fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        ensure_on_screen(&main, main_height(app));
        let _ = main.show();
        let _ = main.set_focus();
    }
}

pub fn rehome<R: Runtime>(app: &AppHandle<R>) {
    if let Some(main) = app.get_webview_window("main")
        && main.is_visible().unwrap_or(false)
        && !main.is_minimized().unwrap_or(false)
    {
        ensure_on_screen(&main, main_height(app));
    }
    if let Some(state) = app.try_state::<crate::state::AppState>() {
        crate::hud::place(app, &state.settings());
    }
}

pub fn space_below_top<R: Runtime>(window: &WebviewWindow<R>) -> Option<f64> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let top = window.outer_position().ok()?.y;
    let bottom = work.position.y + work.size.height as i32;
    Some(f64::from(bottom - top) / scale)
}

pub fn open_variation<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(existing) = app.get_webview_window(VARIATION_LABEL) {
        existing.set_focus()?;
        return Ok(());
    }
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| tauri::Error::WindowNotFound)?;
    let scale = main.scale_factor()?;
    let pos = main.outer_position()?.to_logical::<f64>(scale);
    let size = main.outer_size()?.to_logical::<f64>(scale);
    let (w, h) = VARIATION_SIZE;
    let x = pos.x + (size.width - w) / 2.0;
    let y = pos.y + ((size.height - h) / 2.0).max(0.0);

    let window = WebviewWindowBuilder::new(
        app,
        VARIATION_LABEL,
        WebviewUrl::App("variation.html".into()),
    )
    .title(crate::native_text::text(
        app.state::<crate::state::AppState>().data().settings.locale,
        crate::native_text::Text::VariationTitle,
    ))
    .inner_size(w, h)
    .position(x, y)
    .resizable(false)
    .maximizable(false)
    .minimizable(false)
    .decorations(false)
    .shadow(true)
    .skip_taskbar(true)
    .background_color(crate::theme::background(
        app,
        &app.state::<crate::state::AppState>().data().settings,
    ))
    .parent(&main)?
    .build()?;
    let _ = window.set_position(LogicalPosition::new(x, y));

    let _ = main.set_enabled(false);
    let main_for_close = main.clone();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            let _ = main_for_close.set_enabled(true);
            let _ = main_for_close.set_focus();
        }
    });
    Ok(())
}

pub fn bring_to_front<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    #[cfg(windows)]
    if let Ok(hwnd) = window.hwnd() {
        let hwnd = hwnd.0 as isize;
        let _ = window.run_on_main_thread(move || {
            ch_win::window::bring_to_front(hwnd);
        });
    }
    #[cfg(not(windows))]
    let _ = window.set_focus();
    let webview: &tauri::Webview<R> = window.as_ref();
    let _ = webview.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_between_the_design_limits_and_the_work_area() {
        assert_eq!(clamp_height(500.0, None), 640.0);
        assert_eq!(clamp_height(700.0, None), 700.0);
        assert_eq!(clamp_height(1200.0, None), 900.0);
        assert_eq!(clamp_height(1200.0, Some(800.0)), 800.0, "short screen");
        assert_eq!(
            clamp_height(500.0, Some(600.0)),
            600.0,
            "never taller than the space available"
        );
    }

    const PRIMARY: Rect = Rect::new(0, 0, 1920, 1040);
    const SECOND: Rect = Rect::new(1920, 0, 1920, 1040);

    fn rescue(window: Rect, works: &[Rect]) -> Option<Rect> {
        rescue_rect(window, (210, 320), (420, 760), works, works[0])
    }

    #[test]
    fn finds_points_on_any_screen_including_negative_ones() {
        let screens = [
            Rect::new(0, 0, 1920, 1080),
            Rect::new(-1080, -400, 1080, 1920),
        ];
        assert!(inside_any(&screens, Point::new(1919, 1079)));
        assert!(inside_any(&screens, Point::new(-500, -300)));
        assert!(!inside_any(&screens, Point::new(1920, 0)));
        assert!(!inside_any(&screens, Point::new(-32000, -32000)));
        assert!(!inside_any(&[], Point::new(0, 0)));
    }

    #[test]
    fn leaves_reachable_windows_alone() {
        assert_eq!(
            rescue(Rect::new(700, 200, 420, 760), &[PRIMARY, SECOND]),
            None
        );
        assert_eq!(
            rescue(Rect::new(1700, 10, 420, 760), &[PRIMARY, SECOND]),
            None,
            "straddling two screens"
        );
        assert_eq!(
            rescue(Rect::new(-300, 100, 420, 760), &[PRIMARY]),
            None,
            "partly off the left edge but still grabbable"
        );
    }

    #[test]
    fn restores_the_minimized_placeholder_to_the_center_of_home() {
        assert_eq!(
            rescue(Rect::new(-32000, -32000, 160, 28), &[PRIMARY]),
            Some(Rect::new(750, 140, 420, 760))
        );
    }

    #[test]
    fn brings_back_windows_left_on_a_disconnected_screen() {
        assert_eq!(
            rescue(Rect::new(2500, 100, 436, 649), &[PRIMARY]),
            Some(Rect::new(750, 140, 420, 760)),
            "back to the size the app asked for"
        );
        assert!(
            rescue(Rect::new(500, -30, 420, 760), &[PRIMARY]).is_some(),
            "title strip above the screen"
        );
        assert!(
            rescue(Rect::new(1850, 100, 420, 760), &[PRIMARY]).is_some(),
            "only a sliver visible"
        );
    }

    #[test]
    fn never_grows_past_the_home_work_area() {
        let short = Rect::new(0, 0, 1366, 600);
        assert_eq!(
            rescue_rect(
                Rect::new(-32000, -32000, 160, 28),
                (210, 320),
                (420, 760),
                &[short],
                short
            ),
            Some(Rect::new(473, 0, 420, 600))
        );
    }

    #[test]
    fn remembers_the_last_requested_height() {
        let (tx, _rx) = mpsc::channel();
        let animator = HeightAnimator {
            tx,
            last: Mutex::new(None),
        };
        assert_eq!(animator.last_target(), None);
        animator.animate_to(720.0);
        assert_eq!(animator.last_target(), Some(720.0));
    }

    #[test]
    fn animation_follows_the_ease_out_curve() {
        assert_eq!(height_at(640.0, 900.0, Duration::ZERO), 640.0);
        assert_eq!(height_at(640.0, 900.0, DURATION), 900.0);
        let quarter = height_at(640.0, 900.0, DURATION / 4);
        assert!(
            (quarter - (640.0 + 260.0 * 0.7274)).abs() < 0.5,
            "{quarter}"
        );
        assert!(
            height_at(900.0, 640.0, DURATION / 2) < 700.0,
            "shrinking eases too"
        );
    }
}
