use crate::engine::timeline::Timeline;
use crate::geometry::{Point, Rect};
use crate::input::{InputBackend, InputError};

pub const MOTION_HZ: u64 = 250;

pub const REANCHOR_PX: i32 = 4;

pub fn effective_amplitude(amplitude_pct: f64, monitor_width: i32) -> i32 {
    let raw = amplitude_pct / 100.0 * f64::from(monitor_width);
    (raw.round() as i32).max(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WigglePattern {
    origin: Point,
    left: i32,
    amplitude: i32,
    speed_hz: u32,
}

impl WigglePattern {
    pub fn new(origin: Point, amplitude_pct: f64, speed_hz: u8, monitor: Rect) -> Self {
        let amplitude = effective_amplitude(amplitude_pct, monitor.width).min(monitor.width - 1);
        let mut left = origin.x - amplitude / 2;
        left = left.clamp(monitor.left, monitor.right() - 1 - amplitude);
        Self {
            origin,
            left,
            amplitude,
            speed_hz: u32::from(speed_hz),
        }
    }

    pub const fn origin(&self) -> Point {
        self.origin
    }

    pub const fn amplitude(&self) -> i32 {
        self.amplitude
    }

    pub const fn band(&self) -> (i32, i32) {
        (self.left, self.left + self.amplitude)
    }

    pub fn position(&self, elapsed_ns: u64) -> Point {
        let cycle_ns = 1_000_000_000 / u64::from(self.speed_hz.max(1));
        let phase = (elapsed_ns % cycle_ns) as f64 / cycle_ns as f64;
        let offset =
            f64::from(self.amplitude) * (1.0 - (std::f64::consts::TAU * phase).cos()) / 2.0;
        Point::new(self.left + offset.round() as i32, self.origin.y)
    }
}

pub const MOTION_INTERVAL_NS: u64 = 1_000_000_000 / MOTION_HZ;

const fn far(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() > REANCHOR_PX || (a.y - b.y).abs() > REANCHOR_PX
}

#[derive(Debug, Clone)]
pub struct WiggleTimeline {
    deadline: u64,
    anchor: u64,
    monitors: Vec<Rect>,
    amplitude_pct: f64,
    speed_hz: u8,
    pattern: Option<WigglePattern>,
    last: Option<Point>,
    previous: Option<Point>,
}

impl WiggleTimeline {
    pub fn new(start: u64, monitors: Vec<Rect>, amplitude_pct: f64, speed_hz: u8) -> Self {
        Self {
            deadline: start,
            anchor: start,
            monitors,
            amplitude_pct,
            speed_hz,
            pattern: None,
            last: None,
            previous: None,
        }
    }

    pub const fn pattern(&self) -> Option<&WigglePattern> {
        self.pattern.as_ref()
    }

    fn anchor_at(&mut self, origin: Point) {
        let monitor = self
            .monitors
            .iter()
            .find(|m| m.contains(origin))
            .or_else(|| self.monitors.first())
            .copied()
            .unwrap_or(Rect::new(origin.x, origin.y, 2, 2));
        self.pattern = Some(WigglePattern::new(
            origin,
            self.amplitude_pct,
            self.speed_hz,
            monitor,
        ));
        self.anchor = self.deadline;
        self.last = None;
        self.previous = None;
    }

    fn user_moved(&self, cursor: Point) -> bool {
        self.last.is_some_and(|last| far(cursor, last))
            && self.previous.is_none_or(|previous| far(cursor, previous))
    }
}

impl Timeline for WiggleTimeline {
    fn deadline(&self) -> Option<u64> {
        Some(self.deadline)
    }

    fn interval(&self) -> u64 {
        MOTION_INTERVAL_NS
    }

    fn fire(&mut self, backend: &mut dyn InputBackend) -> Result<(), InputError> {
        let cursor = backend.cursor_pos();
        match (self.pattern, cursor) {
            (None, _) => {
                let origin = cursor.unwrap_or_else(|| {
                    let m = self
                        .monitors
                        .first()
                        .copied()
                        .unwrap_or(Rect::new(0, 0, 2, 2));
                    Point::new(m.left + m.width / 2, m.top + m.height / 2)
                });
                self.anchor_at(origin);
            }
            (Some(_), Some(c)) if self.user_moved(c) => self.anchor_at(c),
            _ => {}
        }
        let elapsed = self.deadline - self.anchor;
        self.deadline += MOTION_INTERVAL_NS;
        let Some(pattern) = self.pattern else {
            return Ok(());
        };
        let at = pattern.position(elapsed);
        if self.last != Some(at) {
            self.previous = self.last;
            self.last = Some(at);
            backend.move_to(at)?;
        }
        Ok(())
    }

    fn rebase(&mut self, now: u64) {
        self.deadline = now + MOTION_INTERVAL_NS;
    }

    fn shift(&mut self, delta: u64) {
        self.deadline += delta;
        self.anchor += delta;
    }

    fn halt(&mut self, backend: &mut dyn InputBackend) {
        let Some(pattern) = self.pattern else {
            return;
        };
        if let Some(c) = backend.cursor_pos()
            && self.user_moved(c)
        {
            return;
        }
        if backend.move_to(pattern.origin()).is_ok() {
            self.previous = self.last;
            self.last = Some(pattern.origin());
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::input::fake::{Call, RecordingBackend};

    const MON: Rect = Rect::new(0, 0, 1920, 1080);
    const MS: u64 = 1_000_000;

    #[test]
    fn tiny_amplitudes_still_move_one_pixel() {
        assert_eq!(effective_amplitude(0.05, 1920), 1);
        assert_eq!(effective_amplitude(0.0, 1920), 1);
        assert_eq!(effective_amplitude(1.0, 1920), 19);
        assert_eq!(effective_amplitude(100.0, 1920), 1920);
    }

    #[test]
    fn oscillates_from_left_to_right_and_back() {
        let p = WigglePattern::new(Point::new(960, 500), 1.0, 8, MON);
        assert_eq!(p.band(), (951, 970));
        assert_eq!(p.position(0), Point::new(951, 500));
        assert_eq!(p.position(62_500_000), Point::new(970, 500));
        assert_eq!(p.position(125 * MS), Point::new(951, 500));
    }

    #[test]
    fn one_pixel_becomes_a_square_wave() {
        let p = WigglePattern::new(Point::new(10, 10), 0.05, 4, MON);
        let xs: Vec<i32> = (0..250).map(|i| p.position(i * MS).x).collect();
        assert!(xs.iter().all(|x| *x == 10 || *x == 11));
        assert!(xs.contains(&10) && xs.contains(&11));
    }

    #[test]
    fn stays_inside_the_monitor_at_the_edges() {
        let right = WigglePattern::new(Point::new(1919, 0), 2.0, 8, MON);
        assert_eq!(right.band().1, 1919);
        let left = WigglePattern::new(
            Point::new(-1675, 0),
            2.0,
            8,
            Rect::new(-1680, 0, 1680, 1050),
        );
        assert_eq!(left.band().0, -1680);
        let full = WigglePattern::new(Point::new(0, 0), 100.0, 8, MON);
        assert_eq!(full.band(), (0, 1919));
    }

    fn timeline(pct: f64) -> WiggleTimeline {
        WiggleTimeline::new(0, vec![Rect::new(-1680, 0, 1680, 1050), MON], pct, 8)
    }

    fn backend_at(x: i32, y: i32) -> RecordingBackend {
        RecordingBackend {
            cursor: Some(Point::new(x, y)),
            ..Default::default()
        }
    }

    fn fire_n(t: &mut WiggleTimeline, b: &mut RecordingBackend, n: usize) {
        for _ in 0..n {
            t.fire(b).expect("fire");
        }
    }

    #[test]
    fn timeline_starts_at_the_cursor_and_returns_to_it() {
        let mut t = timeline(1.0);
        let mut b = backend_at(960, 500);
        assert_eq!(t.interval(), 4 * MS);
        fire_n(&mut t, &mut b, 40);
        assert_eq!(t.deadline(), Some(160 * MS));
        assert_eq!(t.pattern().map(WigglePattern::band), Some((951, 970)));
        assert!(
            b.calls
                .iter()
                .all(|c| matches!(c, Call::MoveTo(p) if p.y == 500))
        );
        assert!(b.calls.windows(2).all(|w| w[0] != w[1]));
        t.halt(&mut b);
        assert_eq!(b.cursor, Some(Point::new(960, 500)));
    }

    #[test]
    fn timeline_follows_the_user_and_leaves_the_cursor_with_them() {
        let mut t = timeline(1.0);
        let mut b = backend_at(960, 500);
        fire_n(&mut t, &mut b, 5);
        b.cursor = Some(Point::new(-800, 300));
        fire_n(&mut t, &mut b, 1);
        let p = t.pattern().copied().expect("pattern");
        assert_eq!(p.origin(), Point::new(-800, 300));
        assert!(
            p.band().0 >= -1680 && p.band().1 < 0,
            "stays on the left monitor"
        );
        fire_n(&mut t, &mut b, 3);
        b.cursor = Some(Point::new(100, 100));
        t.halt(&mut b);
        assert_eq!(
            b.cursor,
            Some(Point::new(100, 100)),
            "halt does not fight the user"
        );
    }

    #[test]
    fn a_lagging_cursor_is_not_mistaken_for_the_user() {
        let mut t = timeline(100.0);
        let mut b = backend_at(960, 500);
        fire_n(&mut t, &mut b, 3);
        let origin = t.pattern().map(WigglePattern::origin);
        let Some(Call::MoveTo(prev)) = b.calls.get(b.calls.len() - 2).cloned() else {
            panic!("expected two moves");
        };
        b.cursor = Some(prev);
        fire_n(&mut t, &mut b, 1);
        assert_eq!(t.pattern().map(WigglePattern::origin), origin);
    }

    #[test]
    fn timeline_without_a_readable_cursor_uses_the_first_monitor() {
        let mut t = WiggleTimeline::new(0, vec![MON], 1.0, 16);
        let mut b = RecordingBackend::default();
        fire_n(&mut t, &mut b, 2);
        assert_eq!(
            t.pattern().map(WigglePattern::origin),
            Some(Point::new(960, 540))
        );
        let mut empty = WiggleTimeline::new(0, Vec::new(), 1.0, 16);
        fire_n(&mut empty, &mut b, 1);
        assert_eq!(
            empty.pattern().map(WigglePattern::origin),
            Some(Point::new(1, 1))
        );
    }

    #[test]
    fn shift_keeps_the_phase_and_rebase_skips_ahead() {
        let mut t = timeline(1.0);
        let mut b = backend_at(960, 500);
        t.halt(&mut b);
        assert!(
            b.calls.is_empty(),
            "nothing to undo before the first update"
        );
        fire_n(&mut t, &mut b, 10);
        let pattern = t.pattern().copied().expect("pattern");
        t.shift(1_000 * MS);
        assert_eq!(t.deadline(), Some(1_040 * MS));
        b.cursor = None;
        b.calls.clear();
        fire_n(&mut t, &mut b, 1);
        let expected = pattern.position(40 * MS);
        assert!(b.calls.is_empty() || b.calls == vec![Call::MoveTo(expected)]);
        t.rebase(5_000 * MS);
        assert_eq!(t.deadline(), Some(5_004 * MS));
    }

    #[test]
    fn move_errors_propagate() {
        let mut t = timeline(10.0);
        let mut b = backend_at(960, 500);
        b.fail_with = Some(InputError::Blocked {
            sent: 0,
            expected: 1,
        });
        assert_eq!(
            t.fire(&mut b),
            Err(InputError::Blocked {
                sent: 0,
                expected: 1
            })
        );
    }

    proptest! {
        #[test]
        fn never_leaves_the_band_or_the_monitor(
            x in 0i32..1920,
            pct in 0.0f64..100.0,
            hz in prop::sample::select(vec![4u8, 8, 16]),
            t in 0u64..10_000_000_000,
        ) {
            let p = WigglePattern::new(Point::new(x, 7), pct, hz, MON);
            let (l, r) = p.band();
            let at = p.position(t);
            prop_assert!(l <= at.x && at.x <= r);
            prop_assert!(MON.contains(at));
            prop_assert_eq!(at.y, 7);
        }

        #[test]
        fn whole_cycles_return_to_the_start(
            x in 0i32..1920,
            pct in 0.0f64..5.0,
            hz in prop::sample::select(vec![4u8, 8, 16]),
            cycles in 0u64..10_000,
        ) {
            let p = WigglePattern::new(Point::new(x, 0), pct, hz, MON);
            let cycle = 1_000_000_000 / u64::from(hz);
            prop_assert_eq!(p.position(cycles * cycle), p.position(0));
        }
    }
}
