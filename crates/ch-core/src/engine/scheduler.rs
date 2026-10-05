use super::clock::NANOS_PER_MS;
use super::timeline::Timeline;
use crate::input::{InputBackend, InputError};

/// Busy-wait stretch before each deadline.
pub const SPIN_WINDOW_NS: u64 = NANOS_PER_MS;
/// Shorter spin for long intervals, where sub-millisecond precision matters less than CPU use.
pub const SPIN_WINDOW_LONG_NS: u64 = 300_000;
pub const LONG_INTERVAL_NS: u64 = 20 * NANOS_PER_MS;
/// Lateness below this never triggers a skip, whatever the interval.
pub const SKIP_GRACE_NS: u64 = 50 * NANOS_PER_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due {
    pub index: usize,
    pub deadline: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FireReport {
    pub lateness_ns: u64,
    /// Events dropped by the skip policy after a stall.
    pub missed: u64,
}

pub struct Scheduler {
    timelines: Vec<Box<dyn Timeline>>,
}

impl std::fmt::Debug for Scheduler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scheduler")
            .field("timelines", &self.timelines.len())
            .field("next", &self.next_due())
            .finish()
    }
}

impl Scheduler {
    pub fn new(timelines: Vec<Box<dyn Timeline>>) -> Self {
        Self { timelines }
    }

    /// Earliest deadline across timelines; ties go to the first timeline.
    pub fn next_due(&self) -> Option<Due> {
        self.timelines
            .iter()
            .enumerate()
            .filter_map(|(index, t)| {
                t.deadline().map(|deadline| Due {
                    index,
                    deadline,
                    interval: t.interval(),
                })
            })
            .min_by_key(|due| (due.deadline, due.index))
    }

    pub fn spin_window(interval: u64) -> u64 {
        if interval >= LONG_INTERVAL_NS {
            SPIN_WINDOW_LONG_NS
        } else {
            SPIN_WINDOW_NS
        }
    }

    /// Fires the due timeline at `now`.
    ///
    /// After a stall longer than `max(2 × interval, 50 ms)` the backlog is dropped instead of
    /// replayed as a burst of clicks; the dropped events are reported as missed.
    pub fn fire(
        &mut self,
        due: Due,
        now: u64,
        backend: &mut dyn InputBackend,
    ) -> Result<FireReport, InputError> {
        let lateness_ns = now.saturating_sub(due.deadline);
        let timeline = &mut self.timelines[due.index];
        let result = timeline.fire(backend);

        let mut missed = 0;
        if lateness_ns > (2 * due.interval).max(SKIP_GRACE_NS)
            && let Some(next) = timeline.deadline()
            && next < now
        {
            missed = (now - next) / timeline.interval().max(1) + 1;
            timeline.rebase(now);
        }
        result.map(|()| FireReport {
            lateness_ns,
            missed,
        })
    }

    pub fn shift_all(&mut self, delta: u64) {
        for t in &mut self.timelines {
            t.shift(delta);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::timeline::IntervalTimeline;
    use crate::input::NoopBackend;
    use crate::input::fake::RecordingBackend;

    const MS: u64 = NANOS_PER_MS;

    fn noop(start: u64, interval: u64) -> Box<dyn Timeline> {
        Box::new(IntervalTimeline::noop(start, interval))
    }

    #[test]
    fn picks_the_earliest_timeline_and_breaks_ties_by_order() {
        let s = Scheduler::new(vec![noop(10, 5), noop(4, 3), noop(4, 7)]);
        assert_eq!(
            s.next_due(),
            Some(Due {
                index: 1,
                deadline: 4,
                interval: 3
            })
        );
        assert!(Scheduler::new(vec![]).next_due().is_none());
        assert!(format!("{s:?}").contains("timelines: 3"));
    }

    #[test]
    fn spin_window_shrinks_for_long_intervals() {
        assert_eq!(Scheduler::spin_window(MS), SPIN_WINDOW_NS);
        assert_eq!(Scheduler::spin_window(20 * MS), SPIN_WINDOW_LONG_NS);
    }

    #[test]
    fn small_lateness_keeps_the_absolute_schedule() {
        let mut s = Scheduler::new(vec![noop(0, MS)]);
        let due = s.next_due().expect("due");
        let report = s.fire(due, 400_000, &mut NoopBackend).expect("ok");
        assert_eq!(
            report,
            FireReport {
                lateness_ns: 400_000,
                missed: 0
            }
        );
        assert_eq!(
            s.next_due().map(|d| d.deadline),
            Some(MS),
            "next stays on the grid"
        );
    }

    #[test]
    fn a_long_stall_skips_the_backlog_instead_of_bursting() {
        let mut s = Scheduler::new(vec![noop(0, MS)]);
        let due = s.next_due().expect("due");
        let report = s.fire(due, 200 * MS, &mut NoopBackend).expect("ok");
        assert_eq!(report.lateness_ns, 200 * MS);
        assert_eq!(report.missed, 200, "deadlines 1..=200 ms were skipped");
        assert_eq!(s.next_due().map(|d| d.deadline), Some(201 * MS));
    }

    #[test]
    fn lateness_under_the_grace_period_is_replayed() {
        let mut s = Scheduler::new(vec![noop(0, MS)]);
        let due = s.next_due().expect("due");
        let report = s.fire(due, 30 * MS, &mut NoopBackend).expect("ok");
        assert_eq!(report.missed, 0);
        assert_eq!(s.next_due().map(|d| d.deadline), Some(MS));
    }

    #[test]
    fn errors_propagate() {
        let mut s = Scheduler::new(vec![Box::new(IntervalTimeline::fixed(
            0,
            MS,
            Box::new(|b| b.button_down(crate::input::MouseButton::Left)),
        ))]);
        let mut backend = RecordingBackend {
            fail_with: Some(InputError::Blocked {
                sent: 0,
                expected: 1,
            }),
            ..Default::default()
        };
        let due = s.next_due().expect("due");
        assert!(s.fire(due, 0, &mut backend).is_err());
    }

    #[test]
    fn shift_all_moves_every_timeline() {
        let mut s = Scheduler::new(vec![noop(10, 5), noop(20, 5)]);
        s.shift_all(100);
        assert_eq!(s.next_due().map(|d| d.deadline), Some(110));
    }
}
