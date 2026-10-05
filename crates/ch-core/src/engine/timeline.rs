use crate::input::{InputBackend, InputError};

/// One stream of timed actions (clicks, cursor motion, the stop timer). The scheduler always
/// fires the timeline with the earliest deadline.
pub trait Timeline: Send {
    /// Next due time in clock ns, or `None` once finished.
    fn deadline(&self) -> Option<u64>;

    /// Gap between the previous deadline and the current one; drives the spin window and the
    /// skip policy.
    fn interval(&self) -> u64;

    /// Runs the due action and moves the deadline forward from the previous deadline (not from
    /// "now"), so jitter never accumulates into drift.
    fn fire(&mut self, backend: &mut dyn InputBackend) -> Result<(), InputError>;

    /// Drops the backlog after a long stall: the next deadline becomes `now + interval`.
    fn rebase(&mut self, now: u64);

    /// Pushes every future deadline back by `delta` (resume after pause).
    fn shift(&mut self, delta: u64);

    /// Firing this timeline ends the run (the stop-after timer).
    fn ends_run(&self) -> bool {
        false
    }

    /// The run pauses or ends: undo anything visible (the wiggle returns the cursor to its origin).
    fn halt(&mut self, _backend: &mut dyn InputBackend) {}
}

/// Fires once at `deadline` and ends the run ("Parar após").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopTimeline {
    deadline: Option<u64>,
}

impl StopTimeline {
    pub fn new(deadline: u64) -> Self {
        Self {
            deadline: Some(deadline),
        }
    }
}

impl Timeline for StopTimeline {
    fn deadline(&self) -> Option<u64> {
        self.deadline
    }

    /// Long enough that the scheduler only spins briefly before it.
    fn interval(&self) -> u64 {
        u64::MAX / 4
    }

    fn fire(&mut self, _backend: &mut dyn InputBackend) -> Result<(), InputError> {
        self.deadline = None;
        Ok(())
    }

    /// The timer is absolute: a stall never moves it.
    fn rebase(&mut self, _now: u64) {}

    fn shift(&mut self, delta: u64) {
        if let Some(d) = self.deadline.as_mut() {
            *d += delta;
        }
    }

    fn ends_run(&self) -> bool {
        true
    }
}

pub type Action = Box<dyn FnMut(&mut dyn InputBackend) -> Result<(), InputError> + Send>;

/// Fires `action` at `start`, then after each interval yielded by the iterator.
pub struct IntervalTimeline {
    deadline: Option<u64>,
    last_gap: u64,
    next_gap: Option<u64>,
    intervals: Box<dyn Iterator<Item = u64> + Send>,
    action: Action,
    remaining: Option<u64>,
}

impl IntervalTimeline {
    pub fn new(
        start: u64,
        intervals: impl Iterator<Item = u64> + Send + 'static,
        action: Action,
    ) -> Self {
        let mut intervals: Box<dyn Iterator<Item = u64> + Send> = Box::new(intervals);
        let next_gap = intervals.next();
        Self {
            deadline: Some(start),
            last_gap: next_gap.unwrap_or(0),
            next_gap,
            intervals,
            action,
            remaining: None,
        }
    }

    pub fn fixed(start: u64, interval: u64, action: Action) -> Self {
        Self::new(start, std::iter::repeat(interval), action)
    }

    /// A timeline that does nothing when it fires; used by the timing benchmark.
    pub fn noop(start: u64, interval: u64) -> Self {
        Self::fixed(start, interval, Box::new(|_| Ok(())))
    }

    /// Finishes after `fires` actions.
    pub fn with_limit(mut self, fires: u64) -> Self {
        self.remaining = Some(fires);
        if fires == 0 {
            self.deadline = None;
        }
        self
    }
}

impl std::fmt::Debug for IntervalTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IntervalTimeline")
            .field("deadline", &self.deadline)
            .field("last_gap", &self.last_gap)
            .field("remaining", &self.remaining)
            .finish_non_exhaustive()
    }
}

impl Timeline for IntervalTimeline {
    fn deadline(&self) -> Option<u64> {
        self.deadline
    }

    fn interval(&self) -> u64 {
        self.last_gap
    }

    fn fire(&mut self, backend: &mut dyn InputBackend) -> Result<(), InputError> {
        let Some(current) = self.deadline else {
            return Ok(());
        };
        let result = (self.action)(backend);

        if let Some(left) = self.remaining.as_mut() {
            *left -= 1;
        }
        self.deadline = match (self.next_gap, self.remaining) {
            (_, Some(0)) | (None, _) => None,
            (Some(gap), _) => {
                self.last_gap = gap;
                self.next_gap = self.intervals.next();
                Some(current + gap)
            }
        };
        result
    }

    fn rebase(&mut self, now: u64) {
        if self.deadline.is_some() {
            self.deadline = Some(now + self.last_gap);
        }
    }

    fn shift(&mut self, delta: u64) {
        if let Some(d) = self.deadline.as_mut() {
            *d += delta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::fake::RecordingBackend;
    use crate::input::{ClickKind, MouseButton};

    fn click() -> Action {
        Box::new(|b| b.click(MouseButton::Left, ClickKind::Single))
    }

    #[test]
    fn first_fire_is_at_start_then_follows_the_intervals() {
        let mut t = IntervalTimeline::new(100, [1, 35, 70].into_iter().cycle(), click());
        let mut backend = RecordingBackend::default();
        let mut seen = Vec::new();
        for _ in 0..5 {
            seen.push((t.deadline().expect("running"), t.interval()));
            t.fire(&mut backend).expect("ok");
        }
        assert_eq!(
            seen,
            vec![(100, 1), (101, 1), (136, 35), (206, 70), (207, 1)]
        );
        assert_eq!(backend.calls.len(), 5);
    }

    #[test]
    fn finishes_when_the_intervals_run_out() {
        let mut t = IntervalTimeline::new(0, [10].into_iter(), click());
        let mut backend = RecordingBackend::default();
        t.fire(&mut backend).expect("ok");
        assert_eq!(t.deadline(), Some(10));
        t.fire(&mut backend).expect("ok");
        assert_eq!(t.deadline(), None);
        t.fire(&mut backend)
            .expect("finished timelines ignore fire");
        assert_eq!(backend.calls.len(), 2);
    }

    #[test]
    fn limit_stops_after_n_fires() {
        let mut t = IntervalTimeline::noop(0, 5).with_limit(3);
        let mut backend = RecordingBackend::default();
        let mut fired = 0;
        while t.deadline().is_some() {
            t.fire(&mut backend).expect("ok");
            fired += 1;
        }
        assert_eq!(fired, 3);
        assert_eq!(IntervalTimeline::noop(0, 5).with_limit(0).deadline(), None);
    }

    #[test]
    fn rebase_and_shift_move_the_next_deadline() {
        let mut t = IntervalTimeline::noop(0, 10);
        t.rebase(1_000);
        assert_eq!(t.deadline(), Some(1_010));
        t.shift(500);
        assert_eq!(t.deadline(), Some(1_510));
    }

    #[test]
    fn stop_timeline_fires_once_and_ends_the_run() {
        let mut t = StopTimeline::new(1_000);
        assert!(t.ends_run());
        assert!(t.interval() > 1_000_000_000);
        t.rebase(5_000);
        assert_eq!(t.deadline(), Some(1_000), "rebase never moves the timer");
        t.shift(500);
        assert_eq!(t.deadline(), Some(1_500), "pauses do");
        t.fire(&mut RecordingBackend::default()).expect("ok");
        assert_eq!(t.deadline(), None);
        t.shift(10);
        assert!(!IntervalTimeline::noop(0, 1).ends_run());
    }

    #[test]
    fn errors_still_advance_the_timeline() {
        let mut t = IntervalTimeline::fixed(0, 10, click());
        let mut backend = RecordingBackend {
            fail_with: Some(InputError::Blocked {
                sent: 0,
                expected: 2,
            }),
            ..Default::default()
        };
        assert!(t.fire(&mut backend).is_err());
        assert_eq!(t.deadline(), Some(10));
        assert!(format!("{t:?}").contains("IntervalTimeline"));
    }
}
