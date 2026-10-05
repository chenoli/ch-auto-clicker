//! The engine thread (ARCHITECTURE §3, §5): countdown, scheduler loop, pause and stop, driven by
//! commands and published through atomics. Nothing on the hot path locks or allocates.

use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ch_core::engine::clock::{Clock, NANOS_PER_MS};
use ch_core::engine::run::{Poll, RunControl, RunEnd, run};
use ch_core::engine::scheduler::{FireReport, Scheduler};
use ch_core::engine::state::{EngineEvent, EngineState, StopReason, transition};
use ch_core::engine::wait::{CoarseWaiter, Waker};
use ch_core::input::{InputBackend, InputError};
use ch_core::stats::AtomicHistogram;

/// Countdown progress is published at this rate.
const COUNTDOWN_STEP_NS: u64 = 100 * NANOS_PER_MS;
/// While paused nothing is scheduled, so commands are polled at a relaxed pace.
const PAUSE_POLL: Duration = Duration::from_millis(20);

type Build = Box<dyn FnOnce(u64, Arc<Shared>) -> Scheduler + Send>;

/// What to run. Timelines are built when the countdown ends, so they start at that instant.
pub struct RunPlan {
    pub countdown_ns: u64,
    /// Shortest intended gap between actions; the degraded-timing check compares jitter to it.
    pub nominal_interval_ns: u64,
    /// Length of the stop timer, if any (drives the "remaining" readout).
    pub stop_after_ns: Option<u64>,
    build: Build,
}

impl RunPlan {
    pub fn new(
        countdown_ns: u64,
        nominal_interval_ns: u64,
        stop_after_ns: Option<u64>,
        build: impl FnOnce(u64, Arc<Shared>) -> Scheduler + Send + 'static,
    ) -> Self {
        Self {
            countdown_ns,
            nominal_interval_ns,
            stop_after_ns,
            build: Box::new(build),
        }
    }
}

#[cfg(test)]
impl RunPlan {
    pub(crate) fn build_for_test(self) -> Build {
        self.build
    }
}

impl std::fmt::Debug for RunPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunPlan")
            .field("countdown_ns", &self.countdown_ns)
            .field("nominal_interval_ns", &self.nominal_interval_ns)
            .field("stop_after_ns", &self.stop_after_ns)
            .finish_non_exhaustive()
    }
}

enum Command {
    Start(RunPlan),
    Stop,
    TogglePause,
    Ack,
    Shutdown,
}

/// One state change, reported from the engine thread (allocation is fine here: transitions are rare).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateChange {
    pub state: EngineState,
    pub prev: EngineState,
    pub reason: Option<StopReason>,
    pub countdown_total_ns: Option<u64>,
    pub error: Option<InputError>,
}

pub type StateHook = Box<dyn Fn(&StateChange) + Send + Sync>;

/// State shared between the engine thread, the telemetry thread and the app.
#[derive(Debug, Default)]
pub struct Shared {
    emergency: AtomicBool,
    state: AtomicU8,
    fires: AtomicU64,
    clicks: AtomicU64,
    skipped_self: AtomicU64,
    missed: AtomicU64,
    run_started_ns: AtomicU64,
    paused_at_ns: AtomicU64,
    paused_total_ns: AtomicU64,
    countdown_end_ns: AtomicU64,
    stop_after_ns: AtomicU64,
    nominal_interval_ns: AtomicU64,
    runs_finished: AtomicU64,
    pub(crate) histogram: AtomicHistogram,
    last_end: Mutex<Option<RunEnd>>,
}

/// Point-in-time readout for telemetry and the bootstrap snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub state: EngineState,
    pub fires: u64,
    pub clicks: u64,
    pub skipped_self: u64,
    pub missed: u64,
    pub elapsed_ns: u64,
    pub countdown_remaining_ns: Option<u64>,
    pub remaining_ns: Option<u64>,
}

impl Shared {
    pub fn state(&self) -> EngineState {
        EngineState::from_u8(self.state.load(Ordering::Acquire))
    }

    /// Anything but idle or error.
    pub fn is_running(&self) -> bool {
        self.state().is_active() || self.state() == EngineState::Stopping
    }

    pub fn fires(&self) -> u64 {
        self.fires.load(Ordering::Relaxed)
    }

    pub fn clicks(&self) -> u64 {
        self.clicks.load(Ordering::Relaxed)
    }

    /// Counts one delivered tick of `clicks` clicks (2 for a double click).
    pub fn add_clicks(&self, clicks: u64) {
        self.clicks.fetch_add(clicks, Ordering::Relaxed);
    }

    /// Counts a click skipped because it would have landed on our own window.
    pub fn add_skipped_self(&self) {
        self.skipped_self.fetch_add(1, Ordering::Relaxed);
    }

    pub fn missed(&self) -> u64 {
        self.missed.load(Ordering::Relaxed)
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

    /// Running time without pauses; frozen while paused, so the stop timer pauses too.
    fn elapsed_ns(&self, now: u64, state: EngineState) -> u64 {
        let started = self.run_started_ns.load(Ordering::Relaxed);
        if started == 0
            || !matches!(
                state,
                EngineState::Running | EngineState::Paused | EngineState::Stopping
            )
        {
            return 0;
        }
        let mut paused = self.paused_total_ns.load(Ordering::Relaxed);
        if state == EngineState::Paused {
            paused += now.saturating_sub(self.paused_at_ns.load(Ordering::Relaxed));
        }
        now.saturating_sub(started).saturating_sub(paused)
    }

    pub fn snapshot(&self, now: u64) -> Snapshot {
        let state = self.state();
        let elapsed_ns = self.elapsed_ns(now, state);
        let stop_after = self.stop_after_ns.load(Ordering::Relaxed);
        Snapshot {
            state,
            fires: self.fires(),
            clicks: self.clicks(),
            skipped_self: self.skipped_self.load(Ordering::Relaxed),
            missed: self.missed(),
            elapsed_ns,
            countdown_remaining_ns: (state == EngineState::Countdown).then(|| {
                self.countdown_end_ns
                    .load(Ordering::Relaxed)
                    .saturating_sub(now)
            }),
            remaining_ns: (stop_after > 0
                && matches!(state, EngineState::Running | EngineState::Paused))
            .then(|| stop_after.saturating_sub(elapsed_ns)),
        }
    }

    fn reset_counters(&self, plan: &RunPlan) {
        for counter in [
            &self.fires,
            &self.clicks,
            &self.skipped_self,
            &self.missed,
            &self.run_started_ns,
            &self.paused_total_ns,
        ] {
            counter.store(0, Ordering::Relaxed);
        }
        let _ = self.histogram.drain();
        self.nominal_interval_ns
            .store(plan.nominal_interval_ns, Ordering::Relaxed);
        self.stop_after_ns
            .store(plan.stop_after_ns.unwrap_or(0), Ordering::Relaxed);
    }

    fn record_end(&self, end: RunEnd) {
        if let Ok(mut slot) = self.last_end.lock() {
            *slot = Some(end);
        }
        self.runs_finished.fetch_add(1, Ordering::AcqRel);
    }
}

/// The engine thread's view of the state machine.
struct Machine<'a> {
    shared: &'a Shared,
    hook: &'a StateHook,
}

impl Machine<'_> {
    fn apply(
        &self,
        event: EngineEvent,
        reason: Option<StopReason>,
        countdown_total_ns: Option<u64>,
        error: Option<InputError>,
    ) {
        let prev = self.shared.state();
        let Some(state) = transition(prev, event) else {
            return;
        };
        self.shared.state.store(state.to_u8(), Ordering::Release);
        (self.hook)(&StateChange {
            state,
            prev,
            reason,
            countdown_total_ns,
            error,
        });
    }

    fn go(&self, event: EngineEvent) {
        self.apply(event, None, None, None);
    }
}

struct EngineControl<'a, C: Clock> {
    rx: &'a mpsc::Receiver<Command>,
    shared: &'a Shared,
    machine: &'a Machine<'a>,
    clock: &'a C,
    pending: &'a mut Option<Command>,
    stop_reason: Option<StopReason>,
}

impl<C: Clock> EngineControl<'_, C> {
    fn stop_with(&mut self, reason: StopReason) -> Poll {
        self.stop_reason = Some(reason);
        Poll::Stop
    }

    fn handle(&mut self, command: Command) -> Poll {
        match command {
            Command::Stop => self.stop_with(StopReason::User),
            Command::TogglePause => Poll::Pause,
            Command::Ack => Poll::Continue,
            other => {
                // A new Start replaces this run; Shutdown ends it. Either way, finish this one first.
                *self.pending = Some(other);
                self.stop_with(StopReason::User)
            }
        }
    }
}

impl<C: Clock> RunControl for EngineControl<'_, C> {
    fn poll(&mut self) -> Poll {
        // Checked first and without the channel, so a flooded queue can never delay it.
        if self.shared.emergency.load(Ordering::Acquire) {
            return self.stop_with(StopReason::Emergency);
        }
        match self.rx.try_recv() {
            Ok(command) => self.handle(command),
            Err(mpsc::TryRecvError::Empty) => Poll::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                *self.pending = Some(Command::Shutdown);
                self.stop_with(StopReason::User)
            }
        }
    }

    fn wait_while_paused(&mut self) -> bool {
        self.shared
            .paused_at_ns
            .store(self.clock.now(), Ordering::Relaxed);
        self.machine.go(EngineEvent::TogglePause);
        let resumed = loop {
            if self.shared.emergency.load(Ordering::Acquire) {
                self.stop_reason = Some(StopReason::Emergency);
                break false;
            }
            match self.rx.recv_timeout(PAUSE_POLL) {
                Ok(Command::TogglePause) => break true,
                Ok(Command::Ack) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Ok(command) => {
                    let _ = self.handle(command);
                    break false;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    *self.pending = Some(Command::Shutdown);
                    break false;
                }
            }
        };
        let paused_for = self
            .clock
            .now()
            .saturating_sub(self.shared.paused_at_ns.load(Ordering::Relaxed));
        self.shared
            .paused_total_ns
            .fetch_add(paused_for, Ordering::Relaxed);
        if resumed {
            self.machine.go(EngineEvent::TogglePause);
        }
        resumed
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

enum CountdownEnd {
    Elapsed,
    Cancelled(StopReason),
}

fn countdown<C: Clock, W: CoarseWaiter>(
    clock: &C,
    waiter: &mut W,
    rx: &mpsc::Receiver<Command>,
    shared: &Shared,
    pending: &mut Option<Command>,
    end: u64,
) -> CountdownEnd {
    loop {
        if shared.emergency.load(Ordering::Acquire) {
            return CountdownEnd::Cancelled(StopReason::Emergency);
        }
        match rx.try_recv() {
            // F6 during the countdown cancels it; pausing has no meaning yet.
            Ok(Command::Stop) => return CountdownEnd::Cancelled(StopReason::User),
            Ok(Command::TogglePause | Command::Ack) | Err(mpsc::TryRecvError::Empty) => {}
            Ok(other) => {
                *pending = Some(other);
                return CountdownEnd::Cancelled(StopReason::User);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                *pending = Some(Command::Shutdown);
                return CountdownEnd::Cancelled(StopReason::User);
            }
        }
        let now = clock.now();
        if now >= end {
            return CountdownEnd::Elapsed;
        }
        let _ = waiter.sleep_until(end.min(now + COUNTDOWN_STEP_NS));
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

impl Engine {
    /// Spawns the engine thread. `init` runs first on that thread (e.g. raising its priority) and
    /// whatever it returns stays alive until the thread exits. `hook` sees every state change.
    pub fn spawn<C, W, B, G>(
        clock: C,
        mut waiter: W,
        mut backend: B,
        init: impl FnOnce() -> G + Send + 'static,
        hook: StateHook,
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
                let shared = thread_shared;
                let machine = Machine {
                    shared: &shared,
                    hook: &hook,
                };
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
                        Command::Stop | Command::TogglePause => {}
                        Command::Ack => machine.go(EngineEvent::Ack),
                        Command::Start(plan) => {
                            let mut ctx = RunContext {
                                clock: &clock,
                                waiter: &mut waiter,
                                backend: &mut backend,
                                rx: &rx,
                                shared: &shared,
                                machine: &machine,
                                pending: &mut pending,
                            };
                            ctx.execute(plan);
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

    fn send(&self, command: Command) {
        let _ = self.tx.send(command);
        (self.waker)();
    }

    /// Starts a run (after its countdown), replacing the current one if any.
    pub fn start(&self, plan: RunPlan) {
        self.send(Command::Start(plan));
    }

    /// Stops the run, or cancels the countdown.
    pub fn stop(&self) {
        self.send(Command::Stop);
    }

    pub fn toggle_pause(&self) {
        self.send(Command::TogglePause);
    }

    /// Clears the error state after the user saw it.
    pub fn ack_error(&self) {
        self.send(Command::Ack);
    }

    /// Stops through an atomic flag that bypasses the command queue.
    pub fn emergency_stop(&self) {
        self.shared.emergency.store(true, Ordering::Release);
        (self.waker)();
    }

    /// Blocks until the engine is idle (or in error) or the timeout passes.
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

/// Everything one run needs, borrowed from the engine thread.
struct RunContext<'a, C, W, B> {
    clock: &'a C,
    waiter: &'a mut W,
    backend: &'a mut B,
    rx: &'a mpsc::Receiver<Command>,
    shared: &'a Arc<Shared>,
    machine: &'a Machine<'a>,
    pending: &'a mut Option<Command>,
}

impl<C: Clock, W: CoarseWaiter, B: InputBackend> RunContext<'_, C, W, B> {
    fn execute(&mut self, plan: RunPlan) {
        let shared = self.shared;
        shared.emergency.store(false, Ordering::Release);
        shared.reset_counters(&plan);

        if plan.countdown_ns > 0 {
            let end = self.clock.now() + plan.countdown_ns;
            shared.countdown_end_ns.store(end, Ordering::Relaxed);
            self.machine.apply(
                EngineEvent::Start { countdown: true },
                None,
                Some(plan.countdown_ns),
                None,
            );
            match countdown(self.clock, self.waiter, self.rx, shared, self.pending, end) {
                CountdownEnd::Elapsed => self.machine.go(EngineEvent::CountdownElapsed),
                CountdownEnd::Cancelled(reason) => {
                    self.machine
                        .apply(EngineEvent::Cancel, Some(reason), None, None);
                    shared.record_end(RunEnd::Stopped);
                    return;
                }
            }
        } else {
            self.machine.go(EngineEvent::Start { countdown: false });
        }

        let start = self.clock.now();
        shared.run_started_ns.store(start.max(1), Ordering::Relaxed);
        let mut scheduler = (plan.build)(start, Arc::clone(shared));
        let mut control = EngineControl {
            rx: self.rx,
            shared,
            machine: self.machine,
            clock: self.clock,
            pending: self.pending,
            stop_reason: None,
        };
        let end = run(
            &mut scheduler,
            self.clock,
            self.waiter,
            self.backend,
            &mut control,
        );
        let reason = match &end {
            RunEnd::Stopped => control.stop_reason.unwrap_or(StopReason::User),
            RunEnd::Completed => StopReason::Completed,
            RunEnd::TimerElapsed => StopReason::Timer,
            RunEnd::Failed(_) => StopReason::Error,
        };
        self.machine
            .apply(EngineEvent::Stop, Some(reason), None, None);
        match &end {
            RunEnd::Failed(e) => {
                self.machine
                    .apply(EngineEvent::Fail, Some(reason), None, Some(e.clone()))
            }
            _ => self
                .machine
                .apply(EngineEvent::Stopped, Some(reason), None, None),
        }
        shared.record_end(end);
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        self.emergency_stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(windows)]
impl Engine {
    /// The production engine: QPC clock, high-resolution timer, `SendInput`, real-time priority.
    pub fn spawn_system(hook: StateHook) -> windows::core::Result<Self> {
        use ch_win::input::SendInputBackend;
        use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};
        Ok(Self::spawn(
            QpcClock::new(),
            HighResWaiter::new()?,
            SendInputBackend::system(),
            RealtimeGuard::acquire,
            hook,
        ))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use ch_core::engine::timeline::{IntervalTimeline, StopTimeline};
    use ch_core::input::NoopBackend;
    use ch_core::input::fake::RecordingBackend;
    use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};

    use super::*;

    const MS: u64 = NANOS_PER_MS;

    type Log = Arc<Mutex<Vec<StateChange>>>;

    fn engine_with<B: InputBackend + Send + 'static>(backend: B) -> (Engine, Log) {
        let log: Log = Arc::default();
        let sink = Arc::clone(&log);
        let engine = Engine::spawn(
            QpcClock::new(),
            HighResWaiter::new().expect("waiter"),
            backend,
            RealtimeGuard::acquire,
            Box::new(move |c| sink.lock().expect("lock").push(c.clone())),
        );
        (engine, log)
    }

    fn engine() -> (Engine, Log) {
        engine_with(NoopBackend)
    }

    fn noop_plan(countdown: u64, interval: u64, limit: Option<u64>) -> RunPlan {
        RunPlan::new(countdown, interval, None, move |start, _| {
            let mut t = IntervalTimeline::noop(start, interval);
            if let Some(n) = limit {
                t = t.with_limit(n);
            }
            Scheduler::new(vec![Box::new(t)])
        })
    }

    fn wait_for(engine: &Engine, state: EngineState) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while engine.shared.state() != state {
            assert!(
                Instant::now() < deadline,
                "never reached {state:?} (now {:?})",
                engine.shared.state()
            );
            std::thread::yield_now();
        }
    }

    fn states(log: &Log) -> Vec<EngineState> {
        log.lock().expect("lock").iter().map(|c| c.state).collect()
    }

    fn last_reason(log: &Log) -> Option<StopReason> {
        log.lock().expect("lock").last().and_then(|c| c.reason)
    }

    #[test]
    fn runs_a_plan_to_completion() {
        let (engine, log) = engine();
        engine.start(noop_plan(0, MS, Some(100)));
        let deadline = Instant::now() + Duration::from_secs(5);
        while engine.shared().runs_finished() == 0 {
            assert!(Instant::now() < deadline, "run never finished");
            std::thread::yield_now();
        }
        assert_eq!(engine.shared().fires(), 100);
        assert_eq!(engine.shared().last_end(), Some(RunEnd::Completed));
        assert_eq!(
            states(&log),
            vec![
                EngineState::Running,
                EngineState::Stopping,
                EngineState::Idle
            ]
        );
        assert_eq!(last_reason(&log), Some(StopReason::Completed));
    }

    #[test]
    fn countdown_precedes_the_run_and_reports_its_length() {
        let (engine, log) = engine();
        let started = Instant::now();
        engine.start(noop_plan(250 * MS, MS, Some(5)));
        wait_for(&engine, EngineState::Countdown);
        let snap = engine.shared().snapshot(QpcClock::new().now());
        assert!(snap.countdown_remaining_ns.is_some_and(|r| r <= 250 * MS));
        wait_for(&engine, EngineState::Idle);
        assert!(started.elapsed() >= Duration::from_millis(240));
        assert_eq!(
            states(&log),
            vec![
                EngineState::Countdown,
                EngineState::Running,
                EngineState::Stopping,
                EngineState::Idle
            ]
        );
        assert_eq!(
            log.lock().expect("lock")[0].countdown_total_ns,
            Some(250 * MS)
        );
    }

    #[test]
    fn stop_during_the_countdown_cancels_without_running() {
        let (engine, log) = engine();
        engine.start(noop_plan(5_000 * MS, MS, None));
        wait_for(&engine, EngineState::Countdown);
        engine.stop();
        wait_for(&engine, EngineState::Idle);
        assert_eq!(engine.shared().fires(), 0);
        assert_eq!(
            states(&log),
            vec![EngineState::Countdown, EngineState::Idle]
        );
        assert_eq!(last_reason(&log), Some(StopReason::User));
    }

    #[test]
    fn emergency_stop_halts_within_a_few_milliseconds() {
        let (engine, log) = engine();
        engine.start(noop_plan(0, MS, None));
        wait_for(&engine, EngineState::Running);
        std::thread::sleep(Duration::from_millis(50));
        for _ in 0..1_000 {
            let _ = engine.tx.send(Command::Ack);
        }
        let pressed = Instant::now();
        engine.emergency_stop();
        assert!(engine.wait_idle(Duration::from_secs(1)));
        let latency = pressed.elapsed();
        assert!(
            latency < Duration::from_millis(5),
            "stopped after {latency:?}"
        );
        assert_eq!(last_reason(&log), Some(StopReason::Emergency));
    }

    #[test]
    fn emergency_stop_also_cancels_a_countdown_and_a_pause() {
        let (engine, log) = engine();
        engine.start(noop_plan(5_000 * MS, MS, None));
        wait_for(&engine, EngineState::Countdown);
        engine.emergency_stop();
        wait_for(&engine, EngineState::Idle);
        assert_eq!(last_reason(&log), Some(StopReason::Emergency));

        engine.start(noop_plan(0, MS, None));
        wait_for(&engine, EngineState::Running);
        engine.toggle_pause();
        wait_for(&engine, EngineState::Paused);
        engine.emergency_stop();
        wait_for(&engine, EngineState::Idle);
        assert_eq!(last_reason(&log), Some(StopReason::Emergency));
    }

    #[test]
    fn pause_freezes_counters_and_elapsed_time() {
        let (engine, log) = engine();
        let clock = QpcClock::new();
        engine.start(noop_plan(0, MS, None));
        wait_for(&engine, EngineState::Running);
        std::thread::sleep(Duration::from_millis(60));
        engine.toggle_pause();
        wait_for(&engine, EngineState::Paused);
        let a = engine.shared().snapshot(clock.now());
        std::thread::sleep(Duration::from_millis(150));
        let b = engine.shared().snapshot(clock.now());
        assert_eq!(a.fires, b.fires, "no fires while paused");
        assert!(
            b.elapsed_ns - a.elapsed_ns < 5 * MS,
            "elapsed frozen: {a:?} → {b:?}"
        );

        engine.toggle_pause();
        wait_for(&engine, EngineState::Running);
        std::thread::sleep(Duration::from_millis(40));
        let c = engine.shared().snapshot(clock.now());
        assert!(c.fires > b.fires);
        assert_eq!(c.missed, 0, "resume does not replay the paused time");
        engine.stop();
        wait_for(&engine, EngineState::Idle);
        assert_eq!(
            states(&log),
            vec![
                EngineState::Running,
                EngineState::Paused,
                EngineState::Running,
                EngineState::Stopping,
                EngineState::Idle
            ]
        );
    }

    #[test]
    fn the_stop_timer_ends_the_run_and_reports_remaining_time() {
        let (engine, log) = engine();
        let clock = QpcClock::new();
        engine.start(RunPlan::new(0, MS, Some(200 * MS), |start, _| {
            Scheduler::new(vec![
                Box::new(IntervalTimeline::noop(start, MS)),
                Box::new(StopTimeline::new(start + 200 * MS)),
            ])
        }));
        wait_for(&engine, EngineState::Running);
        std::thread::sleep(Duration::from_millis(50));
        let snap = engine.shared().snapshot(clock.now());
        assert!(
            snap.remaining_ns
                .is_some_and(|r| r < 200 * MS && r > 100 * MS),
            "{snap:?}"
        );
        wait_for(&engine, EngineState::Idle);
        assert_eq!(last_reason(&log), Some(StopReason::Timer));
        assert_eq!(engine.shared().last_end(), Some(RunEnd::TimerElapsed));
    }

    #[test]
    fn input_failures_end_in_the_error_state_until_acknowledged() {
        let blocked = InputError::Blocked {
            sent: 0,
            expected: 2,
        };
        let (engine, log) = engine_with(RecordingBackend {
            fail_with: Some(blocked.clone()),
            ..Default::default()
        });
        engine.start(RunPlan::new(0, MS, None, |start, _| {
            Scheduler::new(vec![Box::new(IntervalTimeline::fixed(
                start,
                MS,
                Box::new(|b| {
                    b.click(
                        ch_core::input::MouseButton::Left,
                        ch_core::input::ClickKind::Single,
                    )
                }),
            ))])
        }));
        wait_for(&engine, EngineState::Error);
        let last = log.lock().expect("lock").last().cloned().expect("change");
        assert_eq!(last.error, Some(blocked));
        assert_eq!(last.reason, Some(StopReason::Error));
        engine.ack_error();
        wait_for(&engine, EngineState::Idle);
    }

    #[test]
    fn start_replaces_a_running_plan_and_resets_counters() {
        let (engine, _log) = engine();
        engine.start(noop_plan(0, MS, None));
        wait_for(&engine, EngineState::Running);
        let finished = engine.shared().runs_finished();
        engine.start(noop_plan(0, 2 * MS, Some(10)));
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine.shared().runs_finished() < finished + 2 {
            assert!(Instant::now() < deadline, "replacement never finished");
            std::thread::yield_now();
        }
        assert_eq!(engine.shared().fires(), 10);
        assert!(format!("{engine:?}").contains("Engine"));
        assert!(format!("{:?}", noop_plan(0, 1, None)).contains("RunPlan"));
    }

    #[test]
    fn clicks_and_skips_are_counted_by_the_plan() {
        let (engine, _log) = engine();
        engine.start(RunPlan::new(0, MS, None, |start, shared| {
            Scheduler::new(vec![Box::new(
                IntervalTimeline::fixed(
                    start,
                    MS,
                    Box::new(move |_| {
                        shared.add_clicks(2);
                        shared.add_skipped_self();
                        Ok(())
                    }),
                )
                .with_limit(5),
            )])
        }));
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine.shared().runs_finished() == 0 {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        let snap = engine.shared().snapshot(QpcClock::new().now());
        assert_eq!((snap.clicks, snap.skipped_self), (10, 5));
    }
}
