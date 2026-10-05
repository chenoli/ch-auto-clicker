//! App-wide state managed by Tauri: profiles and settings (source of truth for every window), the
//! engine, telemetry, persistence and timing capabilities.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use ch_core::model::{Locale, Profile, Settings};
use ch_engine::engine::Engine;
use ch_engine::telemetry::Telemetry;
use ch_storage::Store;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::capabilities::{self, Capabilities};
use crate::events::{EngineStatus, ProfilesChanged, SettingsChanged};
use crate::persist::{DEBOUNCE, Persister};
use crate::windowing::HeightAnimator;

#[derive(Debug, Clone, PartialEq)]
pub struct AppData {
    pub settings: Settings,
    pub profiles: Vec<Profile>,
}

impl AppData {
    pub fn active_profile(&self) -> Option<&Profile> {
        let id = self.settings.active_profile_id.as_deref()?;
        self.profiles.iter().find(|p| p.id == id)
    }

    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }
}

pub struct AppState {
    pub engine: Engine,
    hotkeys: crate::hotkeys::HotkeyManager,
    _telemetry: Telemetry,
    data: Mutex<AppData>,
    pub persister: Persister,
    capabilities: Mutex<Capabilities>,
    benchmarking: AtomicBool,
    pub animator: OnceLock<HeightAnimator>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl AppState {
    #[cfg(windows)]
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Result<Self, String> {
        let store = Store::open(Store::default_root()).map_err(|e| e.to_string())?;
        let loaded = store
            .load_or_init(
                || Settings::new_default(Locale::PtBr),
                || {
                    Profile::new_default(
                        uuid::Uuid::new_v4().to_string(),
                        "Meu perfil",
                        &ch_core::time::iso_utc(ch_core::time::now_epoch_ms()),
                    )
                },
            )
            .map_err(|e| e.to_string())?;
        for issue in &loaded.issues {
            log::warn!(
                "could not load {}: {} (moved to {:?})",
                issue.file.display(),
                issue.error,
                issue.quarantined_to
            );
        }

        let hook_app = app.clone();
        let engine = Engine::spawn_system(Box::new(move |change| {
            let _ = EngineStatus::from(change).emit(&hook_app);
        }))
        .map_err(|e| format!("engine: {e}"))?;
        let telemetry = Telemetry::spawn(
            engine.shared(),
            ch_win::timing::QpcClock::new(),
            Box::new(crate::events::EventSink(app.clone())),
            ch_engine::telemetry::TICK_PERIOD,
        );
        let hotkeys = crate::hotkeys::HotkeyManager::spawn(app)?;
        let fingerprint = capabilities::fingerprint();
        let capabilities = capabilities::load_cached(&capabilities::cache_path(), &fingerprint)
            .unwrap_or_else(|| Capabilities::provisional(fingerprint));
        Ok(Self {
            engine,
            hotkeys,
            _telemetry: telemetry,
            data: Mutex::new(AppData {
                settings: loaded.settings,
                profiles: loaded.profiles,
            }),
            persister: Persister::spawn(store, DEBOUNCE),
            capabilities: Mutex::new(capabilities),
            benchmarking: AtomicBool::new(false),
            animator: OnceLock::new(),
        })
    }

    pub fn hotkeys(&self) -> &crate::hotkeys::HotkeyManager {
        &self.hotkeys
    }

    pub fn data(&self) -> AppData {
        lock(&self.data).clone()
    }

    /// Applies `f` to the data under the lock; `f` decides what changed.
    pub fn with_data<T>(&self, f: impl FnOnce(&mut AppData) -> T) -> T {
        f(&mut lock(&self.data))
    }

    pub fn capabilities(&self) -> Capabilities {
        lock(&self.capabilities).clone()
    }

    fn set_capabilities(&self, c: Capabilities) {
        *lock(&self.capabilities) = c;
    }
}

pub fn emit_settings<R: Runtime>(app: &AppHandle<R>, settings: &Settings) {
    let _ = SettingsChanged(settings.clone()).emit(app);
}

pub fn emit_profiles<R: Runtime>(app: &AppHandle<R>, data: &AppData) {
    let _ = ProfilesChanged {
        profiles: data.profiles.clone(),
        active_profile_id: data.settings.active_profile_id.clone(),
    }
    .emit(app);
}

/// Runs the timing benchmark (blocking, ~3 s), stores the result and notifies every window.
/// Refuses to start a second benchmark while one is running.
pub fn refresh_capabilities<R: Runtime>(app: &AppHandle<R>) -> Result<Capabilities, String> {
    let state = app.state::<AppState>();
    if state.benchmarking.swap(true, Ordering::AcqRel) {
        return Err("a timing benchmark is already running".into());
    }
    let result = capabilities::measure();
    state.benchmarking.store(false, Ordering::Release);
    let measured = result?;
    state.set_capabilities(measured.clone());
    let _ = measured.emit(app);
    Ok(measured)
}

/// First launch on this machine (or after a Windows update): measure in the background.
pub fn measure_if_needed<R: Runtime>(app: &AppHandle<R>) {
    if app.state::<AppState>().capabilities().measured {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Err(e) = refresh_capabilities(&app) {
            log::warn!("timing benchmark failed: {e}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_the_active_profile() {
        let mut settings = Settings::new_default(Locale::PtBr);
        let a = Profile::new_default("a", "A", "t");
        let b = Profile::new_default("b", "B", "t");
        settings.active_profile_id = Some("b".into());
        let data = AppData {
            settings,
            profiles: vec![a, b],
        };
        assert_eq!(data.active_profile().map(|p| p.name.as_str()), Some("B"));
        assert!(data.profile("a").is_some());
        assert!(data.profile("zzz").is_none());
    }
}
