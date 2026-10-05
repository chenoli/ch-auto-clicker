//! Rust → UI events (ARCHITECTURE §7.2). tauri-specta names them in kebab-case after the type
//! (`EngineTick` → `engine-tick`).

use ch_core::engine::state::{EngineState, StopReason};
use ch_core::model::{Profile, Settings};
use ch_core::stats::JitterStats;
use ch_engine::engine::{Snapshot, StateChange};
use ch_engine::telemetry::TelemetrySink;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

const NANOS_PER_MS: f64 = 1e6;

fn saturating_u32(v: u64) -> u32 {
    v.min(u64::from(u32::MAX)) as u32
}

/// 10 Hz while active: counters and times for the footer, HUD and tray. Also the snapshot shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineTick {
    pub state: EngineState,
    pub fires: u32,
    pub clicks: u32,
    pub skipped_self: u32,
    pub missed: u32,
    pub elapsed_ms: f64,
    pub countdown_remaining_ms: Option<f64>,
    pub remaining_ms: Option<f64>,
}

impl From<Snapshot> for EngineTick {
    fn from(s: Snapshot) -> Self {
        Self {
            state: s.state,
            fires: saturating_u32(s.fires),
            clicks: saturating_u32(s.clicks),
            skipped_self: saturating_u32(s.skipped_self),
            missed: saturating_u32(s.missed),
            elapsed_ms: s.elapsed_ns as f64 / NANOS_PER_MS,
            countdown_remaining_ms: s.countdown_remaining_ns.map(|n| n as f64 / NANOS_PER_MS),
            remaining_ms: s.remaining_ns.map(|n| n as f64 / NANOS_PER_MS),
        }
    }
}

/// Every engine state transition (the main button, footer, HUD and tray follow it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineStatus {
    pub state: EngineState,
    pub prev_state: EngineState,
    pub reason: Option<StopReason>,
    pub countdown_total_ms: Option<f64>,
    pub error_code: Option<String>,
}

impl From<&StateChange> for EngineStatus {
    fn from(c: &StateChange) -> Self {
        Self {
            state: c.state,
            prev_state: c.prev,
            reason: c.reason,
            countdown_total_ms: c.countdown_total_ns.map(|n| n as f64 / NANOS_PER_MS),
            error_code: c.error.as_ref().map(|e| match e {
                ch_core::input::InputError::Blocked { .. } => "INPUT_BLOCKED".to_owned(),
                ch_core::input::InputError::OutsideDesktop { .. } => "OUTSIDE_DESKTOP".to_owned(),
            }),
        }
    }
}

/// 1 Hz lateness summary (diagnostics).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineJitter {
    pub samples: u32,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub missed: u32,
}

impl From<JitterStats> for EngineJitter {
    fn from(s: JitterStats) -> Self {
        Self {
            samples: saturating_u32(s.samples),
            p50_us: s.p50_us,
            p95_us: s.p95_us,
            p99_us: s.p99_us,
            max_us: s.max_us,
            missed: saturating_u32(s.missed),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DegradedReason {
    Timing,
    /// A monitor was unplugged, moved or rescaled during the run; it was paused.
    DisplayChanged,
}

/// The requested rate is not being met (or recovered); drives the warning callout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineDegraded {
    pub degraded: bool,
    pub reason: DegradedReason,
}

/// Settings changed (from any window); every window re-renders from this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
pub struct SettingsChanged(pub Settings);

/// Profiles or the active profile changed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct ProfilesChanged {
    pub profiles: Vec<Profile>,
    pub active_profile_id: Option<String>,
}

/// Forwards engine telemetry to every window.
pub struct EventSink<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> TelemetrySink for EventSink<R> {
    fn tick(&mut self, snapshot: Snapshot) {
        let _ = EngineTick::from(snapshot).emit(&self.0);
    }

    fn jitter(&mut self, stats: JitterStats) {
        let _ = EngineJitter::from(stats).emit(&self.0);
    }

    fn degraded(&mut self, degraded: bool) {
        let _ = EngineDegraded {
            degraded,
            reason: DegradedReason::Timing,
        }
        .emit(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use ch_core::input::InputError;

    use super::*;

    #[test]
    fn tick_converts_units_and_saturates() {
        let t = EngineTick::from(Snapshot {
            state: EngineState::Running,
            fires: 12,
            clicks: u64::MAX,
            skipped_self: 1,
            missed: 0,
            elapsed_ns: 2_500_000,
            countdown_remaining_ns: None,
            remaining_ns: Some(1_000_000_000),
        });
        assert_eq!(t.elapsed_ms, 2.5);
        assert_eq!(t.clicks, u32::MAX);
        assert_eq!(t.remaining_ms, Some(1_000.0));
        let json = serde_json::to_value(&t).expect("json");
        assert_eq!(json["state"], "running");
        assert_eq!(json["countdownRemainingMs"], serde_json::Value::Null);
    }

    #[test]
    fn status_carries_reason_countdown_and_error_code() {
        let s = EngineStatus::from(&StateChange {
            state: EngineState::Error,
            prev: EngineState::Stopping,
            reason: Some(StopReason::Error),
            countdown_total_ns: Some(3_000_000_000),
            error: Some(InputError::Blocked {
                sent: 0,
                expected: 2,
            }),
        });
        assert_eq!(s.error_code.as_deref(), Some("INPUT_BLOCKED"));
        assert_eq!(s.countdown_total_ms, Some(3_000.0));
        let outside = EngineStatus::from(&StateChange {
            state: EngineState::Error,
            prev: EngineState::Stopping,
            reason: None,
            countdown_total_ns: None,
            error: Some(InputError::OutsideDesktop { x: 0, y: 0 }),
        });
        assert_eq!(outside.error_code.as_deref(), Some("OUTSIDE_DESKTOP"));
    }

    #[test]
    fn jitter_keeps_the_percentiles() {
        let j = EngineJitter::from(JitterStats {
            samples: 1_000,
            p50_us: 1.0,
            p95_us: 2.0,
            p99_us: 3.0,
            max_us: 40.0,
            missed: 0,
        });
        assert_eq!((j.samples, j.p99_us, j.max_us), (1_000, 3.0, 40.0));
    }

    #[test]
    fn degraded_reason_serializes_in_camel_case() {
        let json = serde_json::to_string(&EngineDegraded {
            degraded: true,
            reason: DegradedReason::Timing,
        })
        .expect("json");
        assert_eq!(json, r#"{"degraded":true,"reason":"timing"}"#);
    }
}
