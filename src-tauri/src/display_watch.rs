use std::time::Duration;

use ch_core::engine::state::EngineState;
use tauri::{AppHandle, Manager, Runtime};
use tauri_specta::Event;

use crate::events::{DegradedReason, EngineDegraded};
use crate::state::AppState;

const POLL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reaction {
    None,
    Pause,
    Stop,
}

pub fn reaction(state: EngineState, changed: bool) -> Reaction {
    match (state, changed) {
        (_, false) => Reaction::None,
        (EngineState::Running, true) => Reaction::Pause,
        (EngineState::Countdown, true) => Reaction::Stop,
        _ => Reaction::None,
    }
}

#[cfg(windows)]
fn fingerprint() -> String {
    ch_win::display::display_layout().fingerprint()
}

#[cfg(not(windows))]
fn fingerprint() -> String {
    String::new()
}

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    let spawned = std::thread::Builder::new()
        .name("ch-display-watch".into())
        .spawn(move || {
            let mut baseline: Option<String> = None;
            let mut layout = fingerprint();
            loop {
                std::thread::sleep(POLL);
                let Some(state) = app.try_state::<AppState>() else {
                    continue;
                };
                let now = fingerprint();
                if now != layout {
                    log::info!("display layout changed: {layout} → {now}");
                    layout = now.clone();
                    crate::windowing::rehome(&app);
                }
                let engine = state.engine.shared().state();
                if !engine.is_active() {
                    baseline = None;
                    continue;
                }
                if engine == EngineState::Paused {
                    baseline = Some(now);
                    continue;
                }
                let start = baseline.get_or_insert_with(|| now.clone());
                let changed = *start != now;
                match reaction(engine, changed) {
                    Reaction::None => continue,
                    Reaction::Pause => state.engine.toggle_pause(),
                    Reaction::Stop => state.engine.stop(),
                }
                log::warn!("display layout changed during a run: {start} → {now}");
                baseline = Some(now);
                let _ = EngineDegraded {
                    degraded: true,
                    reason: DegradedReason::DisplayChanged,
                }
                .emit(&app);
            }
        });
    if let Err(e) = spawned {
        log::warn!("display watch thread: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_runs_and_cancels_countdowns_only_on_change() {
        assert_eq!(reaction(EngineState::Running, true), Reaction::Pause);
        assert_eq!(reaction(EngineState::Countdown, true), Reaction::Stop);
        assert_eq!(reaction(EngineState::Paused, true), Reaction::None);
        assert_eq!(reaction(EngineState::Running, false), Reaction::None);
        assert_eq!(reaction(EngineState::Idle, true), Reaction::None);
    }
}
