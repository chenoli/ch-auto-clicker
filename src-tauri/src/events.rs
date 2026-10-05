//! Rust → UI events (ARCHITECTURE §7.2). tauri-specta names them in kebab-case after the type
//! (`EngineTick` → `engine-tick`).

use ch_core::stats::JitterStats;
use ch_engine::telemetry::{TelemetrySink, TickSnapshot};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime};
use tauri_specta::Event;

/// 10 Hz while a run is active: counters and elapsed time for the footer, HUD and tray.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineTick {
    pub fires: u32,
    pub missed: u32,
    pub elapsed_ms: f64,
}

impl From<TickSnapshot> for EngineTick {
    fn from(s: TickSnapshot) -> Self {
        Self {
            fires: s.fires.min(u64::from(u32::MAX)) as u32,
            missed: s.missed.min(u64::from(u32::MAX)) as u32,
            elapsed_ms: s.elapsed_ns as f64 / 1e6,
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
            samples: s.samples.min(u64::from(u32::MAX)) as u32,
            p50_us: s.p50_us,
            p95_us: s.p95_us,
            p99_us: s.p99_us,
            max_us: s.max_us,
            missed: s.missed.min(u64::from(u32::MAX)) as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum DegradedReason {
    Timing,
}

/// The requested rate is not being met (or recovered); drives the warning callout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, specta::Type, tauri_specta::Event)]
#[serde(rename_all = "camelCase")]
pub struct EngineDegraded {
    pub degraded: bool,
    pub reason: DegradedReason,
}

/// Forwards engine telemetry to every window.
pub struct EventSink<R: Runtime>(pub AppHandle<R>);

impl<R: Runtime> TelemetrySink for EventSink<R> {
    fn tick(&mut self, snapshot: TickSnapshot) {
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
    use super::*;

    #[test]
    fn tick_converts_nanoseconds_to_milliseconds() {
        let t = EngineTick::from(TickSnapshot {
            fires: 12,
            missed: 1,
            elapsed_ns: 2_500_000,
        });
        assert_eq!(
            t,
            EngineTick {
                fires: 12,
                missed: 1,
                elapsed_ms: 2.5
            }
        );
    }

    #[test]
    fn counters_saturate_instead_of_wrapping() {
        let t = EngineTick::from(TickSnapshot {
            fires: u64::MAX,
            missed: 0,
            elapsed_ns: 0,
        });
        assert_eq!(t.fires, u32::MAX);
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
