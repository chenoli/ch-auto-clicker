//! Native window behavior: the main window's animated height (TASKS T3.17, decision 7) and the
//! variation editor window (T3.18, decision D13).

use std::sync::mpsc;
use std::time::{Duration, Instant};

use ch_core::easing::EASE_OUT;
use tauri::{
    AppHandle, LogicalPosition, LogicalSize, Manager, Runtime, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const MAIN_WIDTH: f64 = 420.0;
pub const MIN_HEIGHT: f64 = 640.0;
pub const MAX_HEIGHT: f64 = 900.0;
const DURATION: Duration = Duration::from_millis(320);
const FRAME: Duration = Duration::from_millis(16);
const VARIATION_LABEL: &str = "variation";
const VARIATION_SIZE: (f64, f64) = (560.0, 640.0);

/// Clamps a requested height to 640–900 px and to the space left below the window's top edge.
pub fn clamp_height(requested: f64, space_below_top: Option<f64>) -> f64 {
    let ceiling = space_below_top.map_or(MAX_HEIGHT, |s| s.min(MAX_HEIGHT));
    requested.clamp(
        MIN_HEIGHT.min(ceiling),
        ceiling.max(MIN_HEIGHT.min(ceiling)),
    )
}

/// Height at `elapsed` of an animation from `from` to `to`.
pub fn height_at(from: f64, to: f64, elapsed: Duration) -> f64 {
    let progress = elapsed.as_secs_f64() / DURATION.as_secs_f64();
    from + (to - from) * EASE_OUT.at(progress)
}

/// Animates the main window's height on its own thread; new targets retarget smoothly.
#[derive(Debug)]
pub struct HeightAnimator {
    tx: mpsc::Sender<f64>,
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
        Self { tx }
    }

    pub fn animate_to(&self, height: f64) {
        let _ = self.tx.send(height);
    }
}

/// Logical pixels between the window's top edge and the bottom of its monitor's work area.
pub fn space_below_top<R: Runtime>(window: &WebviewWindow<R>) -> Option<f64> {
    let monitor = window.current_monitor().ok().flatten()?;
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let top = window.outer_position().ok()?.y;
    let bottom = work.position.y + work.size.height as i32;
    Some(f64::from(bottom - top) / scale)
}

/// Opens (or focuses) the variation editor, centered over the main window and owned by it.
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

    // Modal relative to the main window: block it until the editor closes.
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
