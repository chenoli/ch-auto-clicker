use super::clock::Clock;
use super::scheduler::{FireReport, Scheduler};
use super::wait::{CoarseWaiter, Wake, wait_precisely};
use crate::input::{InputBackend, InputError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// Stop requested (command or emergency stop).
    Stopped,
    /// Every timeline finished on its own (e.g. a scan without loop).
    Completed,
    /// The stop-after timer fired.
    TimerElapsed,
    Failed(InputError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Poll {
    Continue,
    Stop,
    Pause,
}

/// Hooks the engine thread plugs into the loop. `poll` and `on_fire` must not block or allocate.
pub trait RunControl {
    /// Polled before every wait and after every wake.
    fn poll(&mut self) -> Poll;

    /// Blocks while paused. Returns `true` to resume, `false` to stop.
    fn wait_while_paused(&mut self) -> bool {
        false
    }

    fn on_fire(&mut self, report: &FireReport);
}

/// The scheduler loop: wait precisely for the next deadline, fire, repeat. Timelines are halted
/// when the run pauses and when it ends, however it ends.
pub fn run<C: Clock + ?Sized, W: CoarseWaiter + ?Sized>(
    scheduler: &mut Scheduler,
    clock: &C,
    waiter: &mut W,
    backend: &mut dyn InputBackend,
    control: &mut dyn RunControl,
) -> RunEnd {
    let end = run_until_end(scheduler, clock, waiter, backend, control);
    scheduler.halt_all(backend);
    end
}

fn run_until_end<C: Clock + ?Sized, W: CoarseWaiter + ?Sized>(
    scheduler: &mut Scheduler,
    clock: &C,
    waiter: &mut W,
    backend: &mut dyn InputBackend,
    control: &mut dyn RunControl,
) -> RunEnd {
    loop {
        match control.poll() {
            Poll::Continue => {}
            Poll::Stop => return RunEnd::Stopped,
            Poll::Pause => {
                scheduler.halt_all(backend);
                let paused_at = clock.now();
                if !control.wait_while_paused() {
                    return RunEnd::Stopped;
                }
                // Resume where we left off: every deadline moves by the time spent paused.
                scheduler.shift_all(clock.now().saturating_sub(paused_at));
                continue;
            }
        }
        let Some(due) = scheduler.next_due() else {
            return RunEnd::Completed;
        };
        if wait_precisely(
            clock,
            waiter,
            due.deadline,
            Scheduler::spin_window(due.interval),
        ) == Wake::Signal
        {
            continue;
        }
        match scheduler.fire(due, clock.now(), backend) {
            Ok(report) => {
                control.on_fire(&report);
                if report.end_run {
                    return RunEnd::TimerElapsed;
                }
            }
            Err(e) => return RunEnd::Failed(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::clock::{FakeClock, NANOS_PER_MS};
    use crate::engine::timeline::{IntervalTimeline, Timeline};
    use crate::engine::wait::fake::FakeWaiter;
    use crate::input::fake::RecordingBackend;
    use crate::input::{ClickKind, MouseButton};

    const MS: u64 = NANOS_PER_MS;

    #[derive(Default)]
    struct Control {
        fires: Vec<FireReport>,
        stop_after: Option<usize>,
        pause_at: Option<usize>,
        pause_for: u64,
        resume: bool,
        clock: Option<FakeClock>,
    }

    impl RunControl for Control {
        fn poll(&mut self) -> Poll {
            if self.pause_at.is_some_and(|n| self.fires.len() == n) {
                self.pause_at = None;
                return Poll::Pause;
            }
            if self.stop_after.is_some_and(|n| self.fires.len() >= n) {
                Poll::Stop
            } else {
                Poll::Continue
            }
        }

        fn wait_while_paused(&mut self) -> bool {
            if let Some(c) = &self.clock {
                c.advance(self.pause_for);
            }
            self.resume
        }

        fn on_fire(&mut self, report: &FireReport) {
            self.fires.push(*report);
        }
    }

    fn clicks(start: u64, interval: u64, limit: u64) -> Box<IntervalTimeline> {
        Box::new(
            IntervalTimeline::fixed(
                start,
                interval,
                Box::new(|b| b.click(MouseButton::Left, ClickKind::Single)),
            )
            .with_limit(limit),
        )
    }

    /// Counts halts and records the order of events.
    struct Halting {
        inner: IntervalTimeline,
        log: std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>,
    }

    impl Timeline for Halting {
        fn deadline(&self) -> Option<u64> {
            self.inner.deadline()
        }
        fn interval(&self) -> u64 {
            self.inner.interval()
        }
        fn fire(&mut self, backend: &mut dyn InputBackend) -> Result<(), InputError> {
            self.log.lock().expect("log").push("fire");
            self.inner.fire(backend)
        }
        fn rebase(&mut self, now: u64) {
            self.inner.rebase(now);
        }
        fn shift(&mut self, delta: u64) {
            self.inner.shift(delta);
        }
        fn halt(&mut self, _backend: &mut dyn InputBackend) {
            self.log.lock().expect("log").push("halt");
        }
    }

    #[test]
    fn timelines_are_halted_on_pause_and_at_the_end() {
        let clock = FakeClock::with_auto_step(1_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut scheduler = Scheduler::new(vec![Box::new(Halting {
            inner: IntervalTimeline::noop(0, MS).with_limit(3),
            log: std::sync::Arc::clone(&log),
        })]);
        let mut control = Control {
            pause_at: Some(1),
            pause_for: 5 * MS,
            resume: true,
            clock: Some(clock.clone()),
            ..Control::default()
        };
        let end = run(
            &mut scheduler,
            &clock,
            &mut waiter,
            &mut crate::input::NoopBackend,
            &mut control,
        );
        assert_eq!(end, RunEnd::Completed);
        assert_eq!(
            *log.lock().expect("log"),
            vec!["fire", "halt", "fire", "fire", "halt"]
        );
    }

    #[test]
    fn runs_to_completion_with_microsecond_lateness() {
        let clock = FakeClock::with_auto_step(200);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, 1_000)]);
        let mut backend = RecordingBackend::default();
        let mut control = Control::default();

        let end = run(
            &mut scheduler,
            &clock,
            &mut waiter,
            &mut backend,
            &mut control,
        );

        assert_eq!(end, RunEnd::Completed);
        assert_eq!(backend.calls.len(), 1_000);
        assert!(
            control
                .fires
                .iter()
                .all(|r| r.lateness_ns < 10_000 && r.missed == 0)
        );
        // 999 intervals of 1 ms: no drift accumulated.
        let elapsed = clock.peek();
        assert!(
            (999 * MS..999 * MS + 10_000).contains(&elapsed),
            "elapsed {elapsed}"
        );
    }

    #[test]
    fn a_million_events_do_not_drift() {
        // Coarse auto-step keeps the busy-wait short; lateness per event stays under one step.
        let step = 100_000;
        let clock = FakeClock::with_auto_step(step);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![Box::new(
            IntervalTimeline::noop(0, MS).with_limit(1_000_000),
        )]);
        let mut control = Control::default();

        let end = run(
            &mut scheduler,
            &clock,
            &mut waiter,
            &mut crate::input::NoopBackend,
            &mut control,
        );

        assert_eq!(end, RunEnd::Completed);
        assert_eq!(control.fires.len(), 1_000_000);
        assert!(
            control
                .fires
                .iter()
                .all(|r| r.lateness_ns <= 2 * step && r.missed == 0)
        );
        let last_fire = clock.peek();
        assert!(
            (999_999 * MS..999_999 * MS + 3 * step).contains(&last_fire),
            "drifted: last fire at {last_fire} ns"
        );
    }

    #[test]
    fn interleaves_timelines_by_deadline() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![
            clicks(0, 3 * MS, 3),
            Box::new(
                IntervalTimeline::fixed(
                    MS,
                    2 * MS,
                    Box::new(|b| b.click(MouseButton::Right, ClickKind::Single)),
                )
                .with_limit(3),
            ),
        ]);
        let mut backend = RecordingBackend::default();
        run(
            &mut scheduler,
            &clock,
            &mut waiter,
            &mut backend,
            &mut Control::default(),
        );

        use crate::input::fake::Call::Click;
        let order: Vec<_> = backend
            .calls
            .iter()
            .map(|c| match c {
                Click(MouseButton::Left, _) => 'L',
                _ => 'R',
            })
            .collect();
        // L@0 R@1 L@3 R@3 R@5 L@6 (ties go to the first timeline)
        assert_eq!(order.iter().collect::<String>(), "LRLRRL");
    }

    #[test]
    fn stops_when_asked() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, u64::MAX)]);
        let mut backend = RecordingBackend::default();
        let mut control = Control {
            stop_after: Some(5),
            ..Default::default()
        };
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut control
            ),
            RunEnd::Stopped
        );
        assert_eq!(backend.calls.len(), 5);
    }

    #[test]
    fn a_signal_rechecks_the_stop_flag_before_firing() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        waiter.signal_at(500 * MS);
        let mut scheduler = Scheduler::new(vec![clicks(0, 1_000 * MS, 2)]);
        let mut backend = RecordingBackend::default();
        let mut control = Control {
            stop_after: Some(1),
            ..Default::default()
        };
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut control
            ),
            RunEnd::Stopped
        );
        assert_eq!(backend.calls.len(), 1, "the second click never fired");
    }

    #[test]
    fn pausing_shifts_the_schedule_instead_of_bursting() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, 6)]);
        let mut backend = RecordingBackend::default();
        let mut control = Control {
            pause_at: Some(3),
            pause_for: 500 * MS,
            resume: true,
            clock: Some(clock.clone()),
            ..Default::default()
        };
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut control
            ),
            RunEnd::Completed
        );
        assert_eq!(backend.calls.len(), 6);
        assert!(
            control
                .fires
                .iter()
                .all(|r| r.missed == 0 && r.lateness_ns < 10_000),
            "{:?}",
            control.fires
        );
        assert!(clock.peek() >= 505 * MS, "last fire after the pause");
    }

    #[test]
    fn stopping_while_paused_ends_the_run() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, 6)]);
        let mut backend = RecordingBackend::default();
        let mut control = Control {
            pause_at: Some(2),
            resume: false,
            ..Default::default()
        };
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut control
            ),
            RunEnd::Stopped
        );
        assert_eq!(backend.calls.len(), 2);
    }

    #[test]
    fn a_terminal_timeline_ends_the_run_as_timer_elapsed() {
        use crate::engine::timeline::StopTimeline;
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![
            clicks(0, MS, u64::MAX),
            Box::new(StopTimeline::new(10 * MS)),
        ]);
        let mut backend = RecordingBackend::default();
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut Control::default()
            ),
            RunEnd::TimerElapsed
        );
        assert_eq!(
            backend.calls.len(),
            11,
            "clicks at 0..=10 ms, the click at 10 ms goes first"
        );
    }

    #[test]
    fn default_pause_handler_stops() {
        struct Minimal;
        impl RunControl for Minimal {
            fn poll(&mut self) -> Poll {
                Poll::Pause
            }
            fn on_fire(&mut self, _report: &FireReport) {}
        }
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, 3)]);
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut RecordingBackend::default(),
                &mut Minimal
            ),
            RunEnd::Stopped
        );
    }

    #[test]
    fn input_failures_end_the_run() {
        let clock = FakeClock::with_auto_step(100);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = Scheduler::new(vec![clicks(0, MS, 10)]);
        let blocked = InputError::Blocked {
            sent: 0,
            expected: 2,
        };
        let mut backend = RecordingBackend {
            fail_with: Some(blocked.clone()),
            ..Default::default()
        };
        assert_eq!(
            run(
                &mut scheduler,
                &clock,
                &mut waiter,
                &mut backend,
                &mut Control::default()
            ),
            RunEnd::Failed(blocked)
        );
    }
}
