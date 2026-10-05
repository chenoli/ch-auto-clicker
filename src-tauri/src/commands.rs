//! UI → Rust commands (ARCHITECTURE §7.1).

use std::time::Duration;

use ch_core::model::{Profile, Settings};
use ch_core::validate::{validate_profile, validate_settings};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::capabilities::Capabilities;
use crate::error::{AppError, AppResult};
use crate::events::EngineTick;
use crate::state::{AppState, emit_profiles, emit_settings, refresh_capabilities};
use crate::windowing;

/// Everything a window needs to render its first frame.
#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub version: String,
    pub dpi_awareness: String,
    pub settings: Settings,
    pub profiles: Vec<Profile>,
    pub engine: EngineTick,
    pub capabilities: Capabilities,
}

fn engine_tick(state: &AppState) -> EngineTick {
    #[cfg(windows)]
    let now = ch_core::engine::clock::Clock::now(&ch_win::timing::QpcClock::new());
    #[cfg(not(windows))]
    let now = 0;
    state.engine.shared().snapshot(now).into()
}

#[tauri::command]
#[specta::specta]
pub fn app_bootstrap(app: AppHandle, state: State<'_, AppState>) -> Bootstrap {
    let data = state.data();
    Bootstrap {
        version: app.package_info().version.to_string(),
        dpi_awareness: dpi_awareness().to_owned(),
        settings: data.settings,
        profiles: data.profiles,
        engine: engine_tick(&state),
        capabilities: state.capabilities(),
    }
}

/// Replaces the settings (the UI always sends the full object) and notifies every window.
#[tauri::command]
#[specta::specta]
pub fn settings_update(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    if let Some(e) = validate_settings(&settings).into_iter().next() {
        return Err(e.into());
    }
    let saved = state.with_data(|data| {
        // Profile bookkeeping is owned by the profile commands, never by a settings edit.
        let mut next = settings;
        next.profile_order.clone_from(&data.settings.profile_order);
        next.active_profile_id
            .clone_from(&data.settings.active_profile_id);
        data.settings = next.clone();
        next
    });
    state.persister.save_settings(&saved);
    emit_settings(&app, &saved);
    Ok(saved)
}

/// Last version stamp handed out, in epoch ms; keeps stamps strictly increasing in this process.
static LAST_VERSION_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Next profile version stamp: millisecond ISO time, never equal to an earlier one.
fn next_version() -> String {
    use std::sync::atomic::Ordering;
    let now = ch_core::time::now_epoch_ms();
    let mut last = LAST_VERSION_MS.load(Ordering::Relaxed);
    loop {
        let next = now.max(last + 1);
        match LAST_VERSION_MS.compare_exchange_weak(
            last,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return ch_core::time::iso_utc_ms(next),
            Err(actual) => last = actual,
        }
    }
}

/// Replaces a profile. `base_updated_at` is the version the caller edited; if another window saved
/// in between, the update is refused with `STALE_PROFILE` instead of silently overwriting it.
#[tauri::command]
#[specta::specta]
pub fn profile_update(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: Profile,
    base_updated_at: String,
) -> AppResult<Profile> {
    if let Some(e) = validate_profile(&profile).into_iter().next() {
        return Err(e.into());
    }
    let (saved, data) = state.with_data(|data| {
        let Some(slot) = data.profiles.iter_mut().find(|p| p.id == profile.id) else {
            return Err(AppError::new(
                "PROFILE_NOT_FOUND",
                format!("no profile {}", profile.id),
            ));
        };
        if slot.updated_at != base_updated_at {
            return Err(AppError::new(
                "STALE_PROFILE",
                "the profile changed in another window",
            ));
        }
        let mut next = profile;
        next.created_at.clone_from(&slot.created_at);
        next.updated_at = next_version();
        *slot = next.clone();
        Ok((next, data.clone()))
    })?;
    state.persister.save_profile(&saved);
    emit_profiles(&app, &data);
    Ok(saved)
}

/// Starts the given (or active) profile after the configured countdown.
#[tauri::command]
#[specta::specta]
pub fn engine_start(state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    let data = state.data();
    let profile = match profile_id.as_deref() {
        Some(id) => data.profile(id),
        None => data.active_profile(),
    }
    .ok_or_else(|| AppError::new("PROFILE_NOT_FOUND", "no profile to start"))?;
    #[cfg(windows)]
    let guard: Box<dyn ch_core::input::TargetGuard> =
        Box::new(ch_win::guard::OwnWindowGuard::new());
    #[cfg(not(windows))]
    let guard: Box<dyn ch_core::input::TargetGuard> = Box::new(ch_core::input::AllowAll);
    let plan = ch_engine::plan::build_plan(profile, data.settings.countdown_sec, guard)?;
    state.engine.start(plan);
    Ok(())
}

/// Stops the run or cancels the countdown.
#[tauri::command]
#[specta::specta]
pub fn engine_stop(state: State<'_, AppState>) {
    state.engine.stop();
}

#[tauri::command]
#[specta::specta]
pub fn engine_toggle_pause(state: State<'_, AppState>) {
    state.engine.toggle_pause();
}

#[tauri::command]
#[specta::specta]
pub fn engine_ack_error(state: State<'_, AppState>) {
    state.engine.ack_error();
}

#[tauri::command]
#[specta::specta]
pub fn engine_snapshot(state: State<'_, AppState>) -> EngineTick {
    engine_tick(&state)
}

/// Shortest reliable interval on this machine (provisional 4 ms until measured).
#[tauri::command]
#[specta::specta]
pub fn engine_capabilities(state: State<'_, AppState>) -> Capabilities {
    state.capabilities()
}

/// Re-runs the timing benchmark (~3 s) and returns the new capabilities.
#[tauri::command]
#[specta::specta]
pub async fn engine_run_benchmark(app: AppHandle) -> Result<Capabilities, String> {
    tauri::async_runtime::spawn_blocking(move || refresh_capabilities(&app))
        .await
        .map_err(|e| e.to_string())?
}

/// The main window asks for the height its content needs; Rust animates to it (640–900 px).
#[tauri::command]
#[specta::specta]
pub fn window_set_content_height(
    app: AppHandle,
    state: State<'_, AppState>,
    height: f64,
) -> AppResult<f64> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| AppError::new("WINDOW_NOT_FOUND", "main window is gone"))?;
    let target = windowing::clamp_height(height, windowing::space_below_top(&window));
    state
        .animator
        .get_or_init(|| windowing::HeightAnimator::spawn(window))
        .animate_to(target);
    Ok(target)
}

/// Async on purpose: building a webview inside a synchronous command deadlocks WebView2 on
/// Windows (the window frame appears but its content never loads).
#[tauri::command]
#[specta::specta]
pub async fn window_open_variation(app: AppHandle) -> AppResult<()> {
    windowing::open_variation(&app).map_err(|e| AppError::new("WINDOW_OPEN_FAILED", e.to_string()))
}

/// Delay that lets the developer move the cursor off the button before the test click.
const DEBUG_CLICK_DELAY: Duration = Duration::from_secs(2);

/// Dev-only (T1.7): one left click at the cursor after a short delay.
#[tauri::command]
#[specta::specta]
pub fn debug_click_once() -> Result<(), String> {
    if !cfg!(debug_assertions) {
        return Err("debug_click_once is only available in dev builds".into());
    }
    std::thread::spawn(|| {
        std::thread::sleep(DEBUG_CLICK_DELAY);
        match debug_click() {
            Ok(()) => log::info!("debug click sent"),
            Err(e) => log::warn!("debug click failed: {e}"),
        }
    });
    Ok(())
}

#[cfg(windows)]
fn debug_click() -> Result<(), ch_core::input::InputError> {
    use ch_core::input::{ClickKind, InputBackend, MouseButton};
    ch_win::input::SendInputBackend::system().click(MouseButton::Left, ClickKind::Single)
}

#[cfg(not(windows))]
fn debug_click() -> Result<(), ch_core::input::InputError> {
    Ok(())
}

pub fn dpi_awareness() -> &'static str {
    #[cfg(windows)]
    return ch_win::display::current_dpi_awareness().as_str();
    #[cfg(not(windows))]
    return "n/a";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_always_move_forward() {
        let stamps: Vec<_> = (0..50).map(|_| next_version()).collect();
        assert!(stamps.windows(2).all(|w| w[1] > w[0]), "{stamps:?}");
        assert!(stamps[0].ends_with('Z') && stamps[0].contains('.'));
    }
}
