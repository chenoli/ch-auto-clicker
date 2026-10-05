use super::clock::Clock;
use super::scheduler::{FireReport, Scheduler};
use super::wait::{CoarseWaiter, Wake, wait_precisely};
use crate::input::{InputBackend, InputError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    /// Stop requested (command, emergency stop, timer).
    Stopped,
    /// Every timeline finished on its own.
    Completed,
    Failed(InputError),
}

/// Hooks the engine thread plugs into the loop. Must not block or allocate.
pub trait RunControl {
    /// Polled before every wait and after every wake; `true` ends the run.
    fn should_stop(&mut self) -> bool;

    fn on_fire(&mut self, report: &FireReport);
}

/// The scheduler loop: wait precisely for the next deadline, fire, repeat.
pub fn run<C: Clock + ?Sized, W: CoarseWaiter + ?Sized>(
    scheduler: &mut Scheduler,
    clock: &C,
    waiter: &mut W,
    backend: &mut dyn InputBackend,
    control: &mut dyn RunControl,
) -> RunEnd {
    loop {
        if control.should_stop() {
            return RunEnd::Stopped;
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
            Ok(report) => control.on_fire(&report),
            Err(e) => return RunEnd::Failed(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::clock::{FakeClock, NANOS_PER_MS};
    use crate::engine::timeline::IntervalTimeline;
    use crate::engine::wait::fake::FakeWaiter;
    use crate::input::fake::RecordingBackend;
    use crate::input::{ClickKind, MouseButton};

    const MS: u64 = NANOS_PER_MS;

    #[derive(Default)]
    struct Control {
        fires: Vec<FireReport>,
        stop_after: Option<usize>,
    }

    impl RunControl for Control {
        fn should_stop(&mut self) -> bool {
            self.stop_after.is_some_and(|n| self.fires.len() >= n)
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
