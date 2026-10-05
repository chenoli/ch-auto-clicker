//! UI → Rust commands (ARCHITECTURE §7.1).

use std::time::Duration;

use ch_core::hotkeys::{HotkeyTarget, find_conflict, normalize, set_binding};
use ch_core::model::{Hotkey, HotkeyBindings, Profile, Settings};
use ch_core::validate::{validate_profile, validate_settings};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::capabilities::Capabilities;
use crate::error::{AppError, AppResult};
use crate::events::EngineTick;
use crate::hotkeys::HotkeyStatus;
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
    pub hotkeys: Vec<HotkeyStatus>,
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
        profiles: ch_core::profiles::ordered(&data.settings, &data.profiles),
        settings: data.settings,
        engine: engine_tick(&state),
        capabilities: state.capabilities(),
        hotkeys: state.hotkeys().statuses(),
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
    let previous = state.data().settings;
    let saved = state.with_data(|data| {
        let next = data.settings.merge_user_edit(settings);
        data.settings = next.clone();
        next
    });
    state.persister.save_settings(&saved);
    emit_settings(&app, &saved);
    crate::theme::apply(&app, &saved);
    if previous.autostart != saved.autostart {
        use tauri_plugin_autostart::ManagerExt;
        let launcher = app.autolaunch();
        let result = if saved.autostart {
            launcher.enable()
        } else {
            launcher.disable()
        };
        if let Err(e) = result {
            log::warn!("autostart: {e}");
            return Err(AppError::new("AUTOSTART_FAILED", e.to_string()));
        }
    }
    Ok(saved)
}

pub const ANTI_CHEAT_ACK: &str = "ANTI_CHEAT_ACK";

/// Records that the user read the anti-cheat warning; the next start goes ahead.
#[tauri::command]
#[specta::specta]
pub fn settings_acknowledge_anti_cheat(app: AppHandle, state: State<'_, AppState>) -> Settings {
    let saved = state.with_data(|data| {
        data.settings.anti_cheat_acknowledged = true;
        data.settings.clone()
    });
    state.persister.save_settings(&saved);
    emit_settings(&app, &saved);
    saved
}

/// Opens the Ko-fi page in the default browser ("Apoiar ↗" and the tray item). The URL is fixed
/// here, never taken from the UI.
#[tauri::command]
#[specta::specta]
pub fn app_open_support(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(SUPPORT_URL, None::<&str>)
        .map_err(|e| AppError::new("OPEN_FAILED", e.to_string()))
}

pub const SUPPORT_URL: &str = "https://ko-fi.com/chenoli";

/// The current settings, for windows that only need the theme (variation, HUD, overlays).
#[tauri::command]
#[specta::specta]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.data().settings
}

/// Last version stamp handed out, in epoch ms; keeps stamps strictly increasing in this process.
static LAST_VERSION_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Next profile version stamp: millisecond ISO time, never equal to an earlier one.
pub fn next_version() -> String {
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
    let (saved, data, hotkey_changed) = state.with_data(|data| {
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
        let hotkey_changed = slot.hotkey != profile.hotkey;
        let mut next = profile;
        next.created_at.clone_from(&slot.created_at);
        next.updated_at = next_version();
        *slot = next.clone();
        Ok((next, data.clone(), hotkey_changed))
    })?;
    state.persister.save_profile(&saved);
    emit_profiles(&app, &data);
    if hotkey_changed {
        state.hotkeys().apply(&app, &data);
    }
    Ok(saved)
}

/// Builds and starts the plan for `profile_id` (or the active profile). Shared by the main button,
/// the F6 shortcut and profile shortcuts.
pub fn start_profile(state: &AppState, profile_id: Option<String>) -> AppResult<()> {
    let data = state.data();
    // The anti-cheat warning must be read once before anything clicks (T11.3).
    if !data.settings.anti_cheat_acknowledged {
        return Err(AppError::new(
            ANTI_CHEAT_ACK,
            "the anti-cheat warning has not been accepted yet",
        ));
    }
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
    #[cfg(windows)]
    let screens = ch_engine::plan::Screens::from(&ch_win::display::display_layout());
    #[cfg(not(windows))]
    let screens = ch_engine::plan::Screens {
        desktop: ch_core::geometry::Rect::new(0, 0, 1920, 1080),
        monitors: vec![ch_core::geometry::Rect::new(0, 0, 1920, 1080)],
    };
    let plan = ch_engine::plan::build_plan(profile, data.settings.countdown_sec, screens, guard)?;
    state.engine.start(plan);
    Ok(())
}

/// Starts the given (or active) profile after the configured countdown.
#[tauri::command]
#[specta::specta]
pub fn engine_start(state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    start_profile(&state, profile_id)
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

#[tauri::command]
#[specta::specta]
pub fn hotkey_status(state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    state.hotkeys().statuses()
}

/// Suspends every shortcut and starts capturing keys for the capture field.
#[tauri::command]
#[specta::specta]
pub fn hotkey_begin_record(state: State<'_, AppState>) {
    state.hotkeys().begin_recording();
}

/// Leaves recording without changes and restores every shortcut.
#[tauri::command]
#[specta::specta]
pub fn hotkey_cancel_record(app: AppHandle, state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    state.hotkeys().end_recording();
    state.hotkeys().apply(&app, &state.data())
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "result")]
pub enum HotkeySetResult {
    Saved {
        statuses: Vec<HotkeyStatus>,
    },
    /// Another binding already uses the combo; nothing changed (call again with `replace`).
    Conflict {
        with: HotkeyTarget,
    },
}

/// Assigns `combo` (or clears it with `None`) to `target`. With `replace`, the binding that held
/// the combo loses it and shows a dash, as in the "Substituir" action of Tela 06.
#[tauri::command]
#[specta::specta]
pub fn hotkey_set(
    app: AppHandle,
    state: State<'_, AppState>,
    target: HotkeyTarget,
    combo: Option<Hotkey>,
    replace: bool,
) -> AppResult<HotkeySetResult> {
    let combo = combo
        .map(|c| normalize(&c))
        .transpose()
        .map_err(|e| AppError::new(e.code(), e.to_string()))?;
    state.hotkeys().end_recording();
    let outcome = state.with_data(|data| -> AppResult<Option<HotkeyTarget>> {
        if let Some(c) = &combo
            && let Some(other) = find_conflict(&data.settings.hotkeys, &data.profiles, &target, c)
        {
            if !replace {
                return Ok(Some(other));
            }
            assign(data, &other, None)?;
        }
        assign(data, &target, combo.clone())?;
        Ok(None)
    });
    let data = state.data();
    match outcome {
        Ok(Some(with)) => {
            state.hotkeys().apply(&app, &data);
            Ok(HotkeySetResult::Conflict { with })
        }
        Ok(None) => {
            state.persister.save_settings(&data.settings);
            for p in &data.profiles {
                state.persister.save_profile(p);
            }
            emit_settings(&app, &data.settings);
            emit_profiles(&app, &data);
            Ok(HotkeySetResult::Saved {
                statuses: state.hotkeys().apply(&app, &data),
            })
        }
        Err(e) => {
            state.hotkeys().apply(&app, &data);
            Err(e)
        }
    }
}

/// Writes one binding into the app data.
fn assign(
    data: &mut crate::state::AppData,
    target: &HotkeyTarget,
    combo: Option<Hotkey>,
) -> AppResult<()> {
    match target {
        HotkeyTarget::Action(action) => set_binding(&mut data.settings.hotkeys, *action, combo)
            .map_err(|e| AppError::new(e.code(), e.to_string())),
        HotkeyTarget::Profile(id) => {
            let profile = data
                .profiles
                .iter_mut()
                .find(|p| p.id == *id)
                .ok_or_else(|| AppError::new("PROFILE_NOT_FOUND", format!("no profile {id}")))?;
            profile.hotkey = combo;
            profile.updated_at = next_version();
            Ok(())
        }
    }
}

/// Restores the handoff's default shortcuts (F6, F7, Ctrl Shift P, Ctrl Alt Right, Ctrl Alt Esc).
#[tauri::command]
#[specta::specta]
pub fn hotkey_reset_defaults(app: AppHandle, state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    let data = state.with_data(|data| {
        data.settings.hotkeys = HotkeyBindings::default();
        data.clone()
    });
    state.persister.save_settings(&data.settings);
    emit_settings(&app, &data.settings);
    state.hotkeys().apply(&app, &data)
}

/// Tries every registration again (after the user freed a combo in another app).
#[tauri::command]
#[specta::specta]
pub fn hotkey_retry(app: AppHandle, state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    state.hotkeys().apply(&app, &state.data())
}

/// Opens the point capture overlays (Tela 03.B); resolves when the user finishes. Points are saved
/// to the active profile as they are added.
#[tauri::command]
#[specta::specta]
pub async fn overlay_pick_points(app: AppHandle) -> AppResult<()> {
    crate::overlay::pick(app, crate::overlay::OverlayMode::Points).await
}

/// Opens the area selection overlays (Tela 03.A); resolves when the user confirms or cancels.
#[tauri::command]
#[specta::specta]
pub async fn overlay_pick_area(app: AppHandle) -> AppResult<()> {
    crate::overlay::pick(app, crate::overlay::OverlayMode::Area).await
}

#[tauri::command]
#[specta::specta]
pub fn overlay_set_mode(app: AppHandle, mode: crate::overlay::OverlayMode) -> AppResult<()> {
    crate::overlay::set_mode(&app, mode)
}

/// Saves the rectangle drawn on overlay `monitor` (CSS pixels) as the scan area and closes.
#[tauri::command]
#[specta::specta]
pub fn overlay_confirm_area(
    app: AppHandle,
    monitor: u32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> AppResult<ch_core::model::Area> {
    crate::overlay::confirm_area(&app, monitor as usize, (x, y, width, height))
}

#[tauri::command]
#[specta::specta]
pub fn screen_layout() -> crate::overlay::ScreenLayout {
    crate::overlay::screen_layout()
}

#[tauri::command]
#[specta::specta]
pub fn overlay_state(app: AppHandle, monitor: u32) -> AppResult<crate::overlay::OverlayState> {
    crate::overlay::overlay_state(&app, monitor as usize)
}

/// Adds a point at a CSS position of overlay `monitor`.
#[tauri::command]
#[specta::specta]
pub fn overlay_add_point(
    app: AppHandle,
    monitor: u32,
    x: f64,
    y: f64,
) -> AppResult<crate::overlay::OverlayState> {
    crate::overlay::add_point(&app, monitor as usize, x, y)
}

#[tauri::command]
#[specta::specta]
pub fn overlay_undo(app: AppHandle) -> AppResult<()> {
    crate::overlay::undo(&app)
}

#[tauri::command]
#[specta::specta]
pub fn overlay_finish(app: AppHandle) {
    crate::overlay::finish(&app);
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
