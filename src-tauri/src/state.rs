//! App-wide state managed by Tauri.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use ch_engine::engine::Engine;
use ch_engine::telemetry::Telemetry;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::capabilities::{self, Capabilities};

pub struct AppState {
    #[expect(dead_code, reason = "start/stop commands arrive in Phase 3 (T3.13)")]
    pub engine: Engine,
    _telemetry: Telemetry,
    capabilities: Mutex<Capabilities>,
    benchmarking: AtomicBool,
}

impl AppState {
    #[cfg(windows)]
    pub fn new<R: Runtime>(app: &AppHandle<R>) -> Result<Self, String> {
        let engine = Engine::spawn_system().map_err(|e| format!("engine: {e}"))?;
        let telemetry = Telemetry::spawn(
            engine.shared(),
            ch_win::timing::QpcClock::new(),
            Box::new(crate::events::EventSink(app.clone())),
            ch_engine::telemetry::TICK_PERIOD,
        );
        let fingerprint = capabilities::fingerprint();
        let capabilities = capabilities::load_cached(&capabilities::cache_path(), &fingerprint)
            .unwrap_or_else(|| Capabilities::provisional(fingerprint));
        Ok(Self {
            engine,
            _telemetry: telemetry,
            capabilities: Mutex::new(capabilities),
            benchmarking: AtomicBool::new(false),
        })
    }

    pub fn capabilities(&self) -> Capabilities {
        self.capabilities
            .lock()
            .map(|c| c.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    fn set_capabilities(&self, c: Capabilities) {
        match self.capabilities.lock() {
            Ok(mut slot) => *slot = c,
            Err(e) => *e.into_inner() = c,
        }
    }
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
