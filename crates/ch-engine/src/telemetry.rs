//! Telemetry thread (ARCHITECTURE §5.3, §7.2): turns the engine's atomics into 10 Hz ticks, 1 Hz
//! jitter summaries and degraded-timing transitions for the UI.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use ch_core::engine::clock::{Clock, NANOS_PER_US};
use ch_core::engine::state::EngineState;
use ch_core::stats::{DegradedDetector, JitterStats};

use crate::engine::{Shared, Snapshot};

pub const TICK_PERIOD: Duration = Duration::from_millis(100);
const TICKS_PER_JITTER: u32 = 10;

pub trait TelemetrySink: Send {
    /// 10 Hz during countdown, running and pause, plus one final tick when the run ends.
    fn tick(&mut self, snapshot: Snapshot);
    fn jitter(&mut self, stats: JitterStats);
    fn degraded(&mut self, degraded: bool);
}

#[derive(Debug)]
pub struct Telemetry {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Telemetry {
    pub fn spawn<C: Clock + Send + 'static>(
        shared: Arc<Shared>,
        clock: C,
        mut sink: Box<dyn TelemetrySink>,
        period: Duration,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("ch-telemetry".into())
            .spawn(move || {
                let mut detector = DegradedDetector::default();
                let mut was_active = false;
                let mut running_ticks = 0u32;
                while !thread_stop.load(Ordering::Acquire) {
                    std::thread::sleep(period);
                    let snapshot = shared.snapshot(clock.now());
                    let active = snapshot.state.is_active();
                    if active && !was_active {
                        detector = DegradedDetector::default();
                        running_ticks = 0;
                    }
                    if active || was_active {
                        sink.tick(snapshot);
                    }
                    if snapshot.state == EngineState::Running {
                        running_ticks += 1;
                        if running_ticks.is_multiple_of(TICKS_PER_JITTER) {
                            let stats = shared.histogram.drain().stats();
                            sink.jitter(stats);
                            let interval_us =
                                shared.nominal_interval_ns() as f64 / NANOS_PER_US as f64;
                            if let Some(change) = detector.update(stats.p95_us, interval_us) {
                                sink.degraded(change);
                            }
                        }
                    }
                    if !active && was_active && detector.is_degraded() {
                        detector = DegradedDetector::default();
                        sink.degraded(false);
                    }
                    was_active = active;
                }
            })
            .expect("failed to spawn the telemetry thread");
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    use std::sync::Mutex;
    use std::time::Instant;

    use ch_core::engine::clock::NANOS_PER_MS;
    use ch_core::engine::scheduler::Scheduler;
    use ch_core::engine::timeline::IntervalTimeline;
    use ch_core::input::NoopBackend;
    use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};

    use super::*;
    use crate::engine::{Engine, RunPlan};

    #[derive(Default)]
    struct Recorded {
        ticks: Vec<Snapshot>,
        jitter: Vec<JitterStats>,
        degraded: Vec<bool>,
    }

    struct Sink(Arc<Mutex<Recorded>>);

    impl TelemetrySink for Sink {
        fn tick(&mut self, s: Snapshot) {
            self.0.lock().expect("lock").ticks.push(s);
        }
        fn jitter(&mut self, s: JitterStats) {
            self.0.lock().expect("lock").jitter.push(s);
        }
        fn degraded(&mut self, d: bool) {
            self.0.lock().expect("lock").degraded.push(d);
        }
    }

    #[test]
    fn emits_ten_ticks_and_one_jitter_summary_per_second() {
        let clock = QpcClock::new();
        let engine = Engine::spawn(
            clock,
            HighResWaiter::new().expect("waiter"),
            NoopBackend,
            RealtimeGuard::acquire,
            Box::new(|_| {}),
        );
        let recorded = Arc::new(Mutex::new(Recorded::default()));
        let _telemetry = Telemetry::spawn(
            engine.shared(),
            clock,
            Box::new(Sink(Arc::clone(&recorded))),
            TICK_PERIOD,
        );

        engine.start(RunPlan::new(
            300 * NANOS_PER_MS,
            NANOS_PER_MS,
            None,
            |start, _| Scheduler::new(vec![Box::new(IntervalTimeline::noop(start, NANOS_PER_MS))]),
        ));
        let started = Instant::now();
        std::thread::sleep(Duration::from_millis(2_350));
        engine.stop();
        assert!(engine.wait_idle(Duration::from_secs(1)));
        std::thread::sleep(Duration::from_millis(250));
        let seconds = started.elapsed().as_secs_f64();

        let r = recorded.lock().expect("lock");
        let per_second = r.ticks.len() as f64 / seconds;
        assert!(
            (8.0..=11.5).contains(&per_second),
            "{} ticks in {seconds:.2}s",
            r.ticks.len()
        );
        assert!(
            r.ticks
                .iter()
                .any(|t| t.state == EngineState::Countdown && t.countdown_remaining_ns.is_some()),
            "countdown ticks carry the remaining time"
        );
        assert_eq!(
            r.ticks.last().map(|t| t.state),
            Some(EngineState::Idle),
            "final tick after the stop"
        );
        assert!(
            (1..=3).contains(&r.jitter.len()),
            "{} jitter summaries",
            r.jitter.len()
        );
        let first = r.jitter[0];
        assert!(
            first.samples > 800,
            "1 ms run should fire ~1000 times per second: {first:?}"
        );
        assert!(
            first.p99_us < 500.0,
            "p99 {} µs on an idle machine",
            first.p99_us
        );
        assert!(
            r.ticks
                .windows(2)
                .all(|w| w[1].fires >= w[0].fires || w[1].state == EngineState::Idle)
        );
        assert!(
            r.degraded.is_empty(),
            "no degraded transitions expected: {:?}",
            r.degraded
        );
    }
}
