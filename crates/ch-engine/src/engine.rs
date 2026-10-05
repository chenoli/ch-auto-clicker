//! The engine thread (ARCHITECTURE §3): receives commands, runs the scheduler loop and publishes
//! counters through atomics, never through locks or allocation on the hot path.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ch_core::engine::clock::Clock;
use ch_core::engine::run::{RunControl, RunEnd, run};
use ch_core::engine::scheduler::{FireReport, Scheduler};
use ch_core::engine::wait::{CoarseWaiter, Waker};
use ch_core::input::InputBackend;
use ch_core::stats::AtomicHistogram;

#[derive(Debug)]
pub struct RunPlan {
    pub scheduler: Scheduler,
    /// Shortest intended gap between actions; the degraded-timing check compares jitter to it.
    pub nominal_interval_ns: u64,
}

enum Command {
    Start(RunPlan),
    Stop,
    Shutdown,
}

/// State shared between the engine thread, the telemetry thread and the app.
#[derive(Debug, Default)]
pub struct Shared {
    emergency: AtomicBool,
    running: AtomicBool,
    fires: AtomicU64,
    missed: AtomicU64,
    run_started_ns: AtomicU64,
    nominal_interval_ns: AtomicU64,
    runs_finished: AtomicU64,
    pub(crate) histogram: AtomicHistogram,
    last_end: Mutex<Option<RunEnd>>,
}

impl Shared {
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    pub fn fires(&self) -> u64 {
        self.fires.load(Ordering::Relaxed)
    }

    pub fn missed(&self) -> u64 {
        self.missed.load(Ordering::Relaxed)
    }

    pub fn run_started_ns(&self) -> u64 {
        self.run_started_ns.load(Ordering::Relaxed)
    }

    pub fn nominal_interval_ns(&self) -> u64 {
        self.nominal_interval_ns.load(Ordering::Relaxed)
    }

    pub fn runs_finished(&self) -> u64 {
        self.runs_finished.load(Ordering::Acquire)
    }

    pub fn last_end(&self) -> Option<RunEnd> {
        self.last_end.lock().ok().and_then(|g| g.clone())
    }

    fn begin(&self, now: u64, nominal_interval_ns: u64) {
        self.fires.store(0, Ordering::Relaxed);
        self.missed.store(0, Ordering::Relaxed);
        let _ = self.histogram.drain();
        self.run_started_ns.store(now, Ordering::Relaxed);
        self.nominal_interval_ns
            .store(nominal_interval_ns, Ordering::Relaxed);
        self.running.store(true, Ordering::Release);
    }

    fn finish(&self, end: RunEnd) {
        if let Ok(mut slot) = self.last_end.lock() {
            *slot = Some(end);
        }
        self.running.store(false, Ordering::Release);
        self.runs_finished.fetch_add(1, Ordering::AcqRel);
    }
}

struct EngineControl<'a> {
    rx: &'a mpsc::Receiver<Command>,
    shared: &'a Shared,
    pending: &'a mut Option<Command>,
}

impl RunControl for EngineControl<'_> {
    fn should_stop(&mut self) -> bool {
        // Checked first and without the channel, so a flooded queue can never delay it.
        if self.shared.emergency.load(Ordering::Acquire) {
            return true;
        }
        match self.rx.try_recv() {
            Ok(Command::Stop) => true,
            Ok(other) => {
                *self.pending = Some(other);
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                *self.pending = Some(Command::Shutdown);
                true
            }
        }
    }

    fn on_fire(&mut self, report: &FireReport) {
        self.shared.fires.fetch_add(1, Ordering::Relaxed);
        if report.missed > 0 {
            self.shared
                .missed
                .fetch_add(report.missed, Ordering::Relaxed);
            self.shared.histogram.record_missed(report.missed);
        }
        self.shared.histogram.record(report.lateness_ns);
    }
}

pub struct Engine {
    tx: mpsc::Sender<Command>,
    waker: Waker,
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("shared", &self.shared)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Debug for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Start(_) => "Start",
            Self::Stop => "Stop",
            Self::Shutdown => "Shutdown",
        })
    }
}

impl Engine {
    /// Spawns the engine thread. `init` runs first on that thread (e.g. raising its priority) and
    /// whatever it returns stays alive until the thread exits.
    pub fn spawn<C, W, B, G>(
        clock: C,
        mut waiter: W,
        mut backend: B,
        init: impl FnOnce() -> G + Send + 'static,
    ) -> Self
    where
        C: Clock + Send + 'static,
        W: CoarseWaiter + Send + 'static,
        B: InputBackend + Send + 'static,
        G: 'static,
    {
        let (tx, rx) = mpsc::channel();
        let waker = waiter.waker();
        let shared = Arc::new(Shared::default());
        let thread_shared = Arc::clone(&shared);

        let thread = std::thread::Builder::new()
            .name("ch-engine".into())
            .spawn(move || {
                let _init = init();
                let mut pending: Option<Command> = None;
                loop {
                    let command = match pending.take() {
                        Some(c) => c,
                        None => match rx.recv() {
                            Ok(c) => c,
                            Err(_) => break,
                        },
                    };
                    match command {
                        Command::Shutdown => break,
                        Command::Stop => {}
                        Command::Start(mut plan) => {
                            thread_shared.emergency.store(false, Ordering::Release);
                            thread_shared.begin(clock.now(), plan.nominal_interval_ns);
                            let mut control = EngineControl {
                                rx: &rx,
                                shared: &thread_shared,
                                pending: &mut pending,
                            };
                            let end = run(
                                &mut plan.scheduler,
                                &clock,
                                &mut waiter,
                                &mut backend,
                                &mut control,
                            );
                            thread_shared.finish(end);
                        }
                    }
                }
            })
            .expect("failed to spawn the engine thread");

        Self {
            tx,
            waker,
            shared,
            thread: Some(thread),
        }
    }

    pub fn shared(&self) -> Arc<Shared> {
        Arc::clone(&self.shared)
    }

    /// Starts a run, replacing the current one if any.
    pub fn start(&self, plan: RunPlan) {
        let _ = self.tx.send(Command::Start(plan));
        (self.waker)();
    }

    pub fn stop(&self) {
        let _ = self.tx.send(Command::Stop);
        (self.waker)();
    }

    /// Stops the current run through an atomic flag that bypasses the command queue.
    pub fn emergency_stop(&self) {
        self.shared.emergency.store(true, Ordering::Release);
        (self.waker)();
    }

    /// Blocks until no run is active or the timeout passes. Returns whether it went idle.
    pub fn wait_idle(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while self.shared.is_running() {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::yield_now();
        }
        true
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        (self.waker)();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
impl Engine {
    /// The production engine: QPC clock, high-resolution timer, `SendInput`, real-time priority.
    pub fn spawn_system() -> windows::core::Result<Self> {
        use ch_win::input::SendInputBackend;
        use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};
        Ok(Self::spawn(
            QpcClock::new(),
            HighResWaiter::new()?,
            SendInputBackend::system(),
            RealtimeGuard::acquire,
        ))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use ch_core::engine::clock::NANOS_PER_MS;
    use ch_core::engine::timeline::IntervalTimeline;
    use ch_core::input::NoopBackend;
    use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};

    use super::*;

    const MS: u64 = NANOS_PER_MS;

    fn engine() -> Engine {
        Engine::spawn(
            QpcClock::new(),
            HighResWaiter::new().expect("waiter"),
            NoopBackend,
            RealtimeGuard::acquire,
        )
    }

    fn noop_plan(interval: u64, limit: Option<u64>) -> RunPlan {
        let clock = QpcClock::new();
        let mut timeline = IntervalTimeline::noop(clock.now(), interval);
        if let Some(n) = limit {
            timeline = timeline.with_limit(n);
        }
        RunPlan {
            scheduler: Scheduler::new(vec![Box::new(timeline)]),
            nominal_interval_ns: interval,
        }
    }

    /// Starts `plan` and waits until the engine picked it up (running, or already finished).
    fn start_and_wait(engine: &Engine, plan: RunPlan) {
        let finished_before = engine.shared.runs_finished();
        engine.start(plan);
        let deadline = Instant::now() + Duration::from_secs(2);
        while !engine.shared.is_running() && engine.shared.runs_finished() == finished_before {
            assert!(Instant::now() < deadline, "run never started");
            std::thread::yield_now();
        }
    }

    #[test]
    fn runs_a_plan_to_completion() {
        let engine = engine();
        start_and_wait(&engine, noop_plan(MS, Some(100)));
        assert!(engine.wait_idle(Duration::from_secs(5)));
        let shared = engine.shared();
        assert_eq!(shared.fires(), 100);
        assert_eq!(shared.last_end(), Some(RunEnd::Completed));
        assert_eq!(shared.nominal_interval_ns(), MS);
        assert!(shared.run_started_ns() > 0);
    }

    #[test]
    fn emergency_stop_halts_within_a_few_milliseconds() {
        let engine = engine();
        start_and_wait(&engine, noop_plan(MS, None));
        std::thread::sleep(Duration::from_millis(50));

        // Flood the queue first: the flag must not wait behind these commands.
        for _ in 0..1_000 {
            let _ = engine.tx.send(Command::Stop);
        }
        let pressed = Instant::now();
        engine.emergency_stop();
        assert!(engine.wait_idle(Duration::from_secs(1)));
        let latency = pressed.elapsed();
        assert!(
            latency < Duration::from_millis(5),
            "stopped after {latency:?}"
        );
        assert_eq!(engine.shared().last_end(), Some(RunEnd::Stopped));
    }

    #[test]
    fn stop_command_ends_the_run_and_start_replaces_it() {
        let engine = engine();
        start_and_wait(&engine, noop_plan(MS, None));
        engine.start(noop_plan(2 * MS, Some(10)));
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine.shared().runs_finished() < 2 {
            assert!(Instant::now() < deadline, "replacement run never finished");
            std::thread::yield_now();
        }
        assert_eq!(
            engine.shared().fires(),
            10,
            "counters reset for the new run"
        );

        start_and_wait(&engine, noop_plan(MS, None));
        engine.stop();
        assert!(engine.wait_idle(Duration::from_secs(1)));
        assert_eq!(engine.shared().last_end(), Some(RunEnd::Stopped));
    }

    #[test]
    fn wait_idle_times_out_while_running() {
        let engine = engine();
        start_and_wait(&engine, noop_plan(MS, None));
        assert!(!engine.wait_idle(Duration::from_millis(20)));
        engine.emergency_stop();
        assert!(engine.wait_idle(Duration::from_secs(1)));
        assert!(format!("{engine:?}").contains("Engine"));
    }
}
