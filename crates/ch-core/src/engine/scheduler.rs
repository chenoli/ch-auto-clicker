use super::clock::NANOS_PER_MS;
use super::timeline::Timeline;
use crate::input::{InputBackend, InputError};

pub const SPIN_WINDOW_NS: u64 = NANOS_PER_MS;
pub const SPIN_WINDOW_LONG_NS: u64 = 300_000;
pub const LONG_INTERVAL_NS: u64 = 20 * NANOS_PER_MS;
pub const MAX_TOLERANCE_NS: u64 = 5 * NANOS_PER_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Due {
    pub index: usize,
    pub deadline: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FireReport {
    pub lateness_ns: u64,
    pub missed: u64,
    pub end_run: bool,
    pub complete_run: bool,
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

    pub fn tolerance(interval: u64) -> u64 {
        (interval / 2).min(MAX_TOLERANCE_NS)
    }

    pub fn spin_window(interval: u64) -> u64 {
        if interval >= LONG_INTERVAL_NS {
            SPIN_WINDOW_LONG_NS
        } else {
            SPIN_WINDOW_NS
        }
    }

    pub fn fire(
        &mut self,
        due: Due,
        now: u64,
        backend: &mut dyn InputBackend,
    ) -> Result<FireReport, InputError> {
        let lateness_ns = now.saturating_sub(due.deadline);
        let timeline = &mut self.timelines[due.index];
        let result = timeline.fire(backend);
        let end_run = timeline.ends_run();
        let complete_run = timeline.completes_run();

        let mut missed = 0;
        if let Some(next) = timeline.deadline() {
            let gap = timeline.interval();
            if next < now + gap - Self::tolerance(gap) {
                if next <= now {
                    missed = (now - next) / gap.max(1) + 1;
                }
                timeline.rebase(now);
            }
        }
        result.map(|()| FireReport {
            lateness_ns,
            missed,
            end_run,
            complete_run,
        })
    }

    pub fn shift_all(&mut self, delta: u64) {
        for t in &mut self.timelines {
            t.shift(delta);
        }
    }

    pub fn halt_all(&mut self, backend: &mut dyn InputBackend) {
        for t in &mut self.timelines {
            t.halt(backend);
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

    fn fire_at(s: &mut Scheduler, now: u64) -> FireReport {
        let due = s.next_due().expect("due");
        s.fire(due, now, &mut NoopBackend).expect("ok")
    }

    fn next(s: &Scheduler) -> Option<u64> {
        s.next_due().map(|d| d.deadline)
    }

    #[test]
    fn tolerance_is_half_the_interval_up_to_five_ms() {
        assert_eq!(Scheduler::tolerance(2 * MS), MS);
        assert_eq!(Scheduler::tolerance(10 * MS), 5 * MS);
        assert_eq!(Scheduler::tolerance(100 * MS), 5 * MS);
        assert_eq!(Scheduler::tolerance(0), 0);
    }

    #[test]
    fn small_lateness_keeps_the_absolute_schedule() {
        let mut s = Scheduler::new(vec![noop(0, MS)]);
        let report = fire_at(&mut s, 400_000);
        assert_eq!(
            report,
            FireReport {
                lateness_ns: 400_000,
                missed: 0,
                end_run: false,
                complete_run: false
            }
        );
        assert_eq!(next(&s), Some(MS), "next stays on the grid");
    }

    #[test]
    fn lateness_past_the_tolerance_moves_the_next_click_instead_of_squeezing_it() {
        let mut s = Scheduler::new(vec![noop(0, 100 * MS)]);
        assert_eq!(fire_at(&mut s, 4 * MS).missed, 0);
        assert_eq!(
            next(&s),
            Some(100 * MS),
            "4 ms is inside the 5 ms tolerance"
        );

        let mut s = Scheduler::new(vec![noop(0, 100 * MS)]);
        assert_eq!(
            fire_at(&mut s, 8 * MS).missed,
            0,
            "nothing was skipped, only delayed"
        );
        assert_eq!(next(&s), Some(108 * MS));
    }

    #[test]
    fn a_stall_never_turns_into_a_burst() {
        let mut s = Scheduler::new(vec![noop(0, 2 * MS)]);
        let report = fire_at(&mut s, 40 * MS);
        assert_eq!(report.missed, 20, "deadlines 2..=40 ms were skipped");
        assert_eq!(next(&s), Some(42 * MS));

        let mut s = Scheduler::new(vec![noop(0, MS)]);
        let report = fire_at(&mut s, 200 * MS);
        assert_eq!(report.missed, 200);
        assert_eq!(next(&s), Some(201 * MS));
    }

    #[test]
    fn consecutive_clicks_keep_the_minimum_gap_under_random_stalls() {
        for interval in [MS, 2 * MS, 10 * MS, 37 * MS, 100 * MS] {
            let min_gap = interval - Scheduler::tolerance(interval);
            let mut s = Scheduler::new(vec![noop(0, interval)]);
            let mut seed = 0x9e37_79b9_u64;
            let mut last: Option<u64> = None;
            for _ in 0..2_000 {
                seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
                let stall = (seed >> 33) % (3 * interval);
                let deadline = next(&s).expect("due");
                let now = deadline + stall;
                fire_at(&mut s, now);
                if let Some(prev) = last {
                    assert!(
                        now - prev >= min_gap,
                        "{interval}: gap {} < {min_gap}",
                        now - prev
                    );
                }
                last = Some(now);
            }
        }
    }

    #[test]
    fn variation_uses_the_gap_of_the_next_click() {
        let gaps = [20 * MS, 70 * MS].into_iter().cycle();
        let mut s = Scheduler::new(vec![Box::new(IntervalTimeline::new(
            0,
            gaps,
            Box::new(|_| Ok(())),
        ))]);
        fire_at(&mut s, 0);
        assert_eq!(next(&s), Some(20 * MS));
        fire_at(&mut s, 26 * MS);
        assert_eq!(
            next(&s),
            Some(96 * MS),
            "70 ms gap, 6 ms late > 5 ms tolerance"
        );
        fire_at(&mut s, 98 * MS);
        assert_eq!(
            next(&s),
            Some(116 * MS),
            "20 ms gap, 2 ms late keeps the grid"
        );
    }

    #[test]
    fn skipping_time_never_skips_a_point() {
        let order = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seen = std::sync::Arc::clone(&order);
        let mut point = 0_u32;
        let mut s = Scheduler::new(vec![Box::new(IntervalTimeline::fixed(
            0,
            10 * MS,
            Box::new(move |_| {
                seen.lock().expect("lock").push(point % 3);
                point += 1;
                Ok(())
            }),
        ))]);
        for now in [0, 50 * MS, 61 * MS, 400 * MS, 410 * MS] {
            fire_at(&mut s, now);
        }
        assert_eq!(*order.lock().expect("lock"), [0, 1, 2, 0, 1]);
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
