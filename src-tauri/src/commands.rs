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
    if previous.locale != saved.locale {
        localize_default_profiles(&app, &state, saved.locale);
    }
    if previous.autostart != saved.autostart
        && let Err(e) = set_autostart(&app, saved.autostart)
    {
        log::warn!("autostart: {e}");
        return Err(AppError::new("AUTOSTART_FAILED", e));
    }
    Ok(saved)
}

fn localize_default_profiles(app: &AppHandle, state: &AppState, locale: ch_core::model::Locale) {
    let renamed = state.with_data(|data| {
        let names: Vec<&str> = data.profiles.iter().map(|p| p.name.as_str()).collect();
        let indexes = crate::native_text::localize_default_names(&names, locale);
        let name = crate::native_text::text(locale, crate::native_text::Text::DefaultProfileName);
        let changed: Vec<Profile> = indexes
            .into_iter()
            .map(|i| {
                let profile = &mut data.profiles[i];
                profile.name = name.to_owned();
                profile.updated_at = next_version();
                profile.clone()
            })
            .collect();
        (!changed.is_empty()).then(|| (changed, data.clone()))
    });
    if let Some((changed, data)) = renamed {
        for profile in &changed {
            state.persister.save_profile(profile);
        }
        emit_profiles(app, &data);
    }
}

fn set_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| e.to_string())
}

pub fn autostart_fix(wanted: bool, registered: bool) -> Option<bool> {
    (wanted != registered).then_some(wanted)
}

pub fn sync_autostart(app: &AppHandle, wanted: bool) {
    use tauri_plugin_autostart::ManagerExt;
    let registered = match app.autolaunch().is_enabled() {
        Ok(registered) => registered,
        Err(e) => {
            log::warn!("autostart: could not read the Run entry: {e}");
            return;
        }
    };
    if let Some(enabled) = autostart_fix(wanted, registered) {
        match set_autostart(app, enabled) {
            Ok(()) => log::info!(
                "autostart: Run entry {}",
                if enabled { "restored" } else { "removed" }
            ),
            Err(e) => log::warn!("autostart: {e}"),
        }
    }
}

pub const ANTI_CHEAT_ACK: &str = "ANTI_CHEAT_ACK";

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

#[tauri::command]
#[specta::specta]
pub fn app_open_support(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(SUPPORT_URL, None::<&str>)
        .map_err(|e| AppError::new("OPEN_FAILED", e.to_string()))
}

pub const SUPPORT_URL: &str = "https://ko-fi.com/chenoli";

#[tauri::command]
#[specta::specta]
pub fn settings_get(state: State<'_, AppState>) -> Settings {
    state.data().settings
}

static LAST_VERSION_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

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

pub fn start_profile(state: &AppState, profile_id: Option<String>) -> AppResult<()> {
    let data = state.data();
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

#[tauri::command]
#[specta::specta]
pub fn engine_start(state: State<'_, AppState>, profile_id: Option<String>) -> AppResult<()> {
    start_profile(&state, profile_id)
}

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

#[tauri::command]
#[specta::specta]
pub fn engine_capabilities(state: State<'_, AppState>) -> Capabilities {
    state.capabilities()
}

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

#[tauri::command]
#[specta::specta]
pub fn hotkey_begin_record(state: State<'_, AppState>) {
    state.hotkeys().begin_recording();
}

#[tauri::command]
#[specta::specta]
pub fn hotkey_record_key(state: State<'_, AppState>, key: String, down: bool) -> bool {
    state.hotkeys().feed_key(&key, down)
}

#[tauri::command]
#[specta::specta]
pub fn hotkey_cancel_record(app: AppHandle, state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    state.hotkeys().end_recording();
    state.hotkeys().apply(&app, &state.data())
}

#[derive(Debug, Serialize, specta::Type)]
#[serde(rename_all = "camelCase", tag = "result")]
pub enum HotkeySetResult {
    Saved { statuses: Vec<HotkeyStatus> },
    Conflict { with: HotkeyTarget },
}

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

#[tauri::command]
#[specta::specta]
pub fn hotkey_retry(app: AppHandle, state: State<'_, AppState>) -> Vec<HotkeyStatus> {
    state.hotkeys().apply(&app, &state.data())
}

#[tauri::command]
#[specta::specta]
pub async fn overlay_pick_points(app: AppHandle) -> AppResult<()> {
    crate::overlay::pick(app, crate::overlay::OverlayMode::Points).await
}

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
pub fn overlay_focus(app: AppHandle, monitor: u32) {
    crate::overlay::focus(&app, monitor as usize);
}

#[tauri::command]
#[specta::specta]
pub fn overlay_focus_cursor(app: AppHandle) {
    crate::overlay::focus_cursor_monitor(&app);
}

#[tauri::command]
#[specta::specta]
pub fn overlay_cancel(app: AppHandle) {
    crate::overlay::cancel(&app);
}

#[tauri::command]
#[specta::specta]
pub fn overlay_finish(app: AppHandle) {
    crate::overlay::finish(&app);
}

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

#[tauri::command]
#[specta::specta]
pub async fn window_open_variation(app: AppHandle) -> AppResult<()> {
    windowing::open_variation(&app).map_err(|e| AppError::new("WINDOW_OPEN_FAILED", e.to_string()))
}

const DEBUG_CLICK_DELAY: Duration = Duration::from_secs(2);

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
    fn autostart_follows_the_saved_setting() {
        assert_eq!(autostart_fix(true, false), Some(true));
        assert_eq!(autostart_fix(false, true), Some(false));
        assert_eq!(autostart_fix(true, true), None);
        assert_eq!(autostart_fix(false, false), None);
    }

    #[test]
    fn versions_always_move_forward() {
        let stamps: Vec<_> = (0..50).map(|_| next_version()).collect();
        assert!(stamps.windows(2).all(|w| w[1] > w[0]), "{stamps:?}");
        assert!(stamps[0].ends_with('Z') && stamps[0].contains('.'));
    }
}
