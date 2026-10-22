use std::collections::HashMap;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, MutexGuard};

use ch_core::engine::state::EngineState;
use ch_core::hotkeys::{HotkeyAction, HotkeyTarget, RecordStep, all_bindings};
use ch_core::model::{Hotkey, Modifier};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::state::{AppData, AppState, emit_profiles, emit_settings};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyStatus {
    pub target: HotkeyTarget,
    pub combo: Hotkey,
    pub ok: bool,
    pub error_code: Option<String>,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, tauri_specta::Event,
)]
pub struct HotkeyStatusChanged(pub Vec<HotkeyStatus>);

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, tauri_specta::Event,
)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyRecording {
    pub mods: Vec<Modifier>,
    pub combo: Option<Hotkey>,
    pub cancelled: bool,
}

#[derive(
    Debug, Clone, PartialEq, Eq, Serialize, Deserialize, specta::Type, tauri_specta::Event,
)]
pub struct HotkeyActionFailed {
    pub code: String,
}

#[derive(Default)]
struct Registry {
    ids: HashMap<u32, HotkeyTarget>,
    statuses: Vec<HotkeyStatus>,
    notified: Option<Hotkey>,
}

pub struct HotkeyManager {
    #[cfg(windows)]
    service: ch_win::hotkeys::HotkeyService,
    registry: Mutex<Registry>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

enum Dispatch {
    Triggered(u32),
    Recording(RecordStep),
}

impl HotkeyManager {
    #[cfg(windows)]
    pub fn spawn<R: Runtime>(app: &AppHandle<R>) -> Result<Self, String> {
        use ch_win::hotkeys::{HotkeyService, ServiceEvent};

        let (tx, rx) = mpsc::channel::<Dispatch>();
        let tx = Mutex::new(tx);
        let emergency_app = app.clone();
        let service = HotkeyService::spawn(
            Box::new(move |event| {
                let message = match event {
                    ServiceEvent::Triggered(id) => Dispatch::Triggered(id),
                    ServiceEvent::Recording(step) => Dispatch::Recording(step),
                };
                let _ = lock(&tx).send(message);
            }),
            Arc::new(move || {
                if let Some(state) = emergency_app.try_state::<AppState>() {
                    state.engine.emergency_stop();
                }
            }),
        )
        .map_err(|e| format!("hotkeys: {e}"))?;

        let dispatch_app = app.clone();
        std::thread::Builder::new()
            .name("ch-hotkey-dispatch".into())
            .spawn(move || {
                while let Ok(message) = rx.recv() {
                    match message {
                        Dispatch::Triggered(id) => on_triggered(&dispatch_app, id),
                        Dispatch::Recording(step) => on_recording(&dispatch_app, step),
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            service,
            registry: Mutex::default(),
        })
    }

    pub fn statuses(&self) -> Vec<HotkeyStatus> {
        lock(&self.registry).statuses.clone()
    }

    pub fn apply<R: Runtime>(&self, app: &AppHandle<R>, data: &AppData) -> Vec<HotkeyStatus> {
        let mut registry = lock(&self.registry);
        registry.ids.clear();
        registry.statuses.clear();
        #[cfg(windows)]
        self.service.unregister_all();
        for (index, (target, combo)) in all_bindings(&data.settings.hotkeys, &data.profiles)
            .into_iter()
            .enumerate()
        {
            let id = index as u32 + 1;
            #[cfg(windows)]
            let result = if target == HotkeyTarget::Action(HotkeyAction::EmergencyStop) {
                self.service.set_emergency(&combo)
            } else {
                self.service.register(id, &combo)
            };
            #[cfg(not(windows))]
            let result: Result<(), String> = Ok(());
            if let Err(e) = &result {
                log::warn!("shortcut {combo:?} for {target:?} not registered: {e}");
            }
            registry.ids.insert(id, target.clone());
            registry.statuses.push(HotkeyStatus {
                target,
                combo,
                ok: result.is_ok(),
                #[cfg(windows)]
                error_code: result.err().map(|e| e.code().to_owned()),
                #[cfg(not(windows))]
                error_code: None,
            });
        }
        let statuses = registry.statuses.clone();
        let taken = statuses
            .iter()
            .find(|s| s.target == HotkeyTarget::Action(HotkeyAction::StartStop) && !s.ok)
            .map(|s| s.combo.clone());
        let announce = taken
            .clone()
            .filter(|c| registry.notified.as_ref() != Some(c));
        registry.notified = taken;
        drop(registry);
        let _ = HotkeyStatusChanged(statuses.clone()).emit(app);
        if let Some(combo) = announce {
            crate::tray::notify_hotkey_taken(app, &combo);
        }
        statuses
    }

    pub fn begin_recording(&self) {
        #[cfg(windows)]
        {
            self.service.unregister_all();
            self.service.set_recording(true);
        }
    }

    pub fn end_recording(&self) {
        #[cfg(windows)]
        self.service.set_recording(false);
    }

    pub fn feed_key(&self, key: &str, down: bool) -> bool {
        #[cfg(windows)]
        return self.service.feed_key(key, down);
        #[cfg(not(windows))]
        {
            let _ = (key, down);
            false
        }
    }

    fn target_of(&self, id: u32) -> Option<HotkeyTarget> {
        lock(&self.registry).ids.get(&id).cloned()
    }
}

fn on_recording<R: Runtime>(app: &AppHandle<R>, step: RecordStep) {
    let state = app.state::<AppState>();
    let event = match step {
        RecordStep::Partial(mods) => HotkeyRecording {
            mods,
            combo: None,
            cancelled: false,
        },
        RecordStep::Done(combo) => {
            state.hotkeys().end_recording();
            HotkeyRecording {
                mods: combo.mods.clone(),
                combo: Some(combo),
                cancelled: false,
            }
        }
        RecordStep::Cancelled => {
            state.hotkeys().end_recording();
            state.hotkeys().apply(app, &state.data());
            HotkeyRecording {
                mods: Vec::new(),
                combo: None,
                cancelled: true,
            }
        }
    };
    let _ = event.emit(app);
}

fn on_triggered<R: Runtime>(app: &AppHandle<R>, id: u32) {
    let state = app.state::<AppState>();
    let Some(target) = state.hotkeys().target_of(id) else {
        return;
    };
    let engine_state = state.engine.shared().state();
    let active = engine_state.is_active();
    let result = match target {
        HotkeyTarget::Action(HotkeyAction::StartStop) => match start_stop(engine_state) {
            StartStop::Resume => {
                state.engine.toggle_pause();
                Ok(())
            }
            StartStop::Stop => {
                state.engine.stop();
                Ok(())
            }
            StartStop::Start => crate::commands::start_profile(&state, None),
        },
        HotkeyTarget::Action(HotkeyAction::PauseResume) => {
            if matches!(engine_state, EngineState::Running | EngineState::Paused) {
                state.engine.toggle_pause();
            }
            Ok(())
        }
        HotkeyTarget::Action(HotkeyAction::NextProfile) => {
            if active {
                state.engine.stop();
            }
            switch_profile(app, &state, None);
            Ok(())
        }
        HotkeyTarget::Action(HotkeyAction::CapturePoint) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) =
                    crate::overlay::pick(app.clone(), crate::overlay::OverlayMode::Points).await
                {
                    let _ = HotkeyActionFailed { code: e.code }.emit(&app);
                }
            });
            Ok(())
        }
        HotkeyTarget::Action(HotkeyAction::EmergencyStop) => Ok(()),
        HotkeyTarget::Profile(id) => {
            let is_current =
                state.data().settings.active_profile_id.as_deref() == Some(id.as_str());
            if active && is_current {
                state.engine.stop();
                Ok(())
            } else {
                if active {
                    state.engine.stop();
                    let _ = state
                        .engine
                        .wait_idle(std::time::Duration::from_millis(500));
                }
                switch_profile(app, &state, Some(&id));
                crate::commands::start_profile(&state, Some(id))
            }
        }
    };
    if let Err(e) = result {
        if e.code == crate::commands::ANTI_CHEAT_ACK {
            crate::windowing::show_main(app);
        }
        let _ = HotkeyActionFailed { code: e.code }.emit(app);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StartStop {
    Start,
    Stop,
    Resume,
}

fn start_stop(state: EngineState) -> StartStop {
    match state {
        EngineState::Paused => StartStop::Resume,
        s if s.is_active() => StartStop::Stop,
        _ => StartStop::Start,
    }
}

pub fn switch_profile<R: Runtime>(app: &AppHandle<R>, state: &AppState, id: Option<&str>) {
    let data = state.with_data(|data| {
        let order = &data.settings.profile_order;
        let next = match id {
            Some(id) => data
                .profiles
                .iter()
                .any(|p| p.id == id)
                .then(|| id.to_owned()),
            None => {
                let current = data.settings.active_profile_id.as_deref();
                let index = order.iter().position(|p| Some(p.as_str()) == current);
                index
                    .and_then(|i| order.get((i + 1) % order.len()))
                    .or(order.first())
                    .cloned()
            }
        };
        if next.is_some() && next != data.settings.active_profile_id {
            data.settings.active_profile_id = next;
        }
        data.clone()
    });
    state.persister.save_settings(&data.settings);
    emit_settings(app, &data.settings);
    emit_profiles(app, &data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_stop_resumes_a_paused_run_like_the_main_button() {
        assert_eq!(start_stop(EngineState::Paused), StartStop::Resume);
        assert_eq!(start_stop(EngineState::Running), StartStop::Stop);
        assert_eq!(start_stop(EngineState::Countdown), StartStop::Stop);
        assert_eq!(start_stop(EngineState::Idle), StartStop::Start);
        assert_eq!(start_stop(EngineState::Error), StartStop::Start);
    }
}
