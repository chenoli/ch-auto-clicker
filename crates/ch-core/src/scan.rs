use crate::engine::timeline::Timeline;
use crate::geometry::Point;
use crate::input::{InputBackend, InputError};
use crate::model::Area;
use crate::wiggle::MOTION_INTERVAL_NS;

const NANOS_PER_SEC: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ScanPath {
    vertices: Vec<Point>,
    cumulative: Vec<f64>,
    rows: u32,
    looped: bool,
}

impl ScanPath {
    pub fn new(area: Area, spacing: u32, looped: bool) -> Self {
        let spacing = spacing.max(1);
        let w = i32::try_from(area.w.max(1)).unwrap_or(i32::MAX);
        let h = area.h.max(1);
        let (left, right) = (area.x, area.x.saturating_add(w - 1));
        let rows = (h - 1) / spacing + 1;
        let mut vertices = Vec::with_capacity(rows as usize * 2 + 1);
        for row in 0..rows {
            let y = area
                .y
                .saturating_add(i32::try_from(row * spacing).unwrap_or(i32::MAX));
            let (a, b) = if row % 2 == 0 {
                (left, right)
            } else {
                (right, left)
            };
            vertices.push(Point::new(a, y));
            vertices.push(Point::new(b, y));
        }
        if looped {
            vertices.push(Point::new(left, area.y));
        }
        let mut cumulative = Vec::with_capacity(vertices.len());
        let mut total = 0.0;
        for (i, v) in vertices.iter().enumerate() {
            if i > 0 {
                total += distance(vertices[i - 1], *v);
            }
            cumulative.push(total);
        }
        Self {
            vertices,
            cumulative,
            rows,
            looped,
        }
    }

    pub const fn rows(&self) -> u32 {
        self.rows
    }

    pub fn length(&self) -> f64 {
        self.cumulative.last().copied().unwrap_or(0.0)
    }

    pub fn cycle_secs(&self, speed_px_s: u32) -> f64 {
        self.length() / f64::from(speed_px_s.max(1))
    }

    pub fn start(&self) -> Point {
        self.vertices.first().copied().unwrap_or(Point::new(0, 0))
    }

    pub fn point_at(&self, s: f64) -> Point {
        let s = s.clamp(0.0, self.length());
        let i = self.cumulative.partition_point(|&c| c < s).max(1);
        let Some(&b) = self.vertices.get(i) else {
            return self.vertices.last().copied().unwrap_or(Point::new(0, 0));
        };
        let a = self.vertices[i - 1];
        let seg = self.cumulative[i] - self.cumulative[i - 1];
        let t = if seg > 0.0 {
            (s - self.cumulative[i - 1]) / seg
        } else {
            0.0
        };
        Point::new(
            lerp(a.x, b.x, t).round() as i32,
            lerp(a.y, b.y, t).round() as i32,
        )
    }

    pub fn distance_at(&self, elapsed_ns: u64, speed_px_s: u32) -> f64 {
        let travelled = elapsed_ns as f64 / NANOS_PER_SEC * f64::from(speed_px_s);
        let length = self.length();
        if length <= 0.0 {
            0.0
        } else if self.looped {
            travelled % length
        } else {
            travelled.min(length)
        }
    }
}

fn distance(a: Point, b: Point) -> f64 {
    f64::from(a.x - b.x).hypot(f64::from(a.y - b.y))
}

fn lerp(a: i32, b: i32, t: f64) -> f64 {
    f64::from(a) + (f64::from(b) - f64::from(a)) * t
}

#[derive(Debug, Clone)]
pub struct ScanTimeline {
    path: ScanPath,
    speed_px_s: u32,
    start: u64,
    deadline: Option<u64>,
    last: Option<Point>,
}

impl ScanTimeline {
    pub fn new(start: u64, path: ScanPath, speed_px_s: u32) -> Self {
        Self {
            path,
            speed_px_s,
            start,
            deadline: Some(start),
            last: None,
        }
    }
}

impl Timeline for ScanTimeline {
    fn deadline(&self) -> Option<u64> {
        self.deadline
    }

    fn interval(&self) -> u64 {
        MOTION_INTERVAL_NS
    }

    fn fire(&mut self, backend: &mut dyn InputBackend) -> Result<(), InputError> {
        let Some(now) = self.deadline else {
            return Ok(());
        };
        let s = self.path.distance_at(now - self.start, self.speed_px_s);
        let finished = !self.path.looped && s >= self.path.length();
        self.deadline = (!finished).then_some(now + MOTION_INTERVAL_NS);
        let at = self.path.point_at(s);
        if self.last != Some(at) {
            self.last = Some(at);
            backend.move_to(at)?;
        }
        Ok(())
    }

    fn rebase(&mut self, now: u64) {
        if self.deadline.is_some() {
            self.deadline = Some(now + MOTION_INTERVAL_NS);
        }
    }

    fn shift(&mut self, delta: u64) {
        self.start += delta;
        if let Some(d) = self.deadline.as_mut() {
            *d += delta;
        }
    }

    fn completes_run(&self) -> bool {
        self.deadline.is_none()
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;
    use crate::input::fake::{Call, RecordingBackend};

    const AREA: Area = Area {
        x: 480,
        y: 300,
        w: 776,
        h: 260,
    };
    const MS: u64 = 1_000_000;

    #[test]
    fn rows_follow_the_spacing_and_stay_inside() {
        let p = ScanPath::new(AREA, 24, false);
        assert_eq!(p.rows(), 11);
        assert_eq!(p.start(), Point::new(480, 300));
        assert!((p.length() - (11.0 * 775.0 + 240.0)).abs() < 1e-9);
        assert_eq!(p.point_at(f64::MAX), Point::new(1255, 540));
        assert_eq!(ScanPath::new(Area { h: 240, ..AREA }, 24, false).rows(), 10);
    }

    #[test]
    fn walks_the_zigzag_by_distance() {
        let p = ScanPath::new(AREA, 24, false);
        assert_eq!(p.point_at(0.0), Point::new(480, 300));
        assert_eq!(p.point_at(387.5), Point::new(868, 300));
        assert_eq!(p.point_at(775.0), Point::new(1255, 300));
        assert_eq!(p.point_at(775.0 + 12.0), Point::new(1255, 312));
        assert_eq!(p.point_at(775.0 + 24.0 + 775.0), Point::new(480, 324));
        assert_eq!(p.point_at(-5.0), Point::new(480, 300));
    }

    #[test]
    fn the_loop_returns_in_a_straight_line_and_counts_in_the_cycle() {
        let open = ScanPath::new(AREA, 24, false);
        let looped = ScanPath::new(AREA, 24, true);
        let back = 775f64.hypot(240.0);
        assert!((looped.length() - open.length() - back).abs() < 1e-9);
        assert!((looped.cycle_secs(1200) - looped.length() / 1200.0).abs() < 1e-12);
        assert_eq!(looped.point_at(looped.length()), Point::new(480, 300));
        assert_eq!(
            looped.point_at(open.length() + back / 2.0),
            Point::new(868, 420)
        );
    }

    #[test]
    fn distance_wraps_with_loop_and_stops_without() {
        let looped = ScanPath::new(AREA, 24, true);
        let open = ScanPath::new(AREA, 24, false);
        let one_lap = (looped.length() / 1200.0 * 1e9) as u64;
        assert!(looped.distance_at(one_lap + 500 * MS, 1200) < 601.0);
        assert!((open.distance_at(u64::MAX / 2, 1200) - open.length()).abs() < 1e-9);
        assert!((open.distance_at(1_000 * MS, 600) - 600.0).abs() < 1e-9);
    }

    #[test]
    fn tiny_or_degenerate_areas_still_work() {
        let dot = ScanPath::new(
            Area {
                x: 5,
                y: 5,
                w: 1,
                h: 1,
            },
            8,
            false,
        );
        assert_eq!(dot.rows(), 1);
        assert_eq!(dot.length(), 0.0);
        assert_eq!(dot.point_at(10.0), Point::new(5, 5));
        assert_eq!(dot.distance_at(5 * MS, 600), 0.0);
        assert_eq!(dot.cycle_secs(0), 0.0);
    }

    #[test]
    fn timeline_without_loop_moves_then_completes() {
        let path = ScanPath::new(
            Area {
                x: 0,
                y: 0,
                w: 101,
                h: 17,
            },
            8,
            false,
        );
        let mut t = ScanTimeline::new(0, path, 2400);
        let mut b = RecordingBackend::default();
        assert_eq!(t.interval(), 4 * MS);
        let mut fires = 0;
        while t.deadline().is_some() {
            t.fire(&mut b).expect("fire");
            fires += 1;
            assert!(fires < 100, "never finished");
        }
        assert!(t.completes_run());
        assert_eq!(fires, 34);
        assert_eq!(b.calls.first(), Some(&Call::MoveTo(Point::new(0, 0))));
        assert_eq!(b.calls.last(), Some(&Call::MoveTo(Point::new(100, 16))));
        t.fire(&mut b)
            .expect("finished timelines ignore extra fires");
        t.rebase(10 * MS);
        assert_eq!(t.deadline(), None);
    }

    #[test]
    fn timeline_with_loop_never_completes_and_keeps_phase_over_pauses() {
        let path = ScanPath::new(AREA, 24, true);
        let mut t = ScanTimeline::new(0, path.clone(), 1200);
        let mut b = RecordingBackend::default();
        for _ in 0..10 {
            t.fire(&mut b).expect("fire");
        }
        assert!(!t.completes_run());
        t.shift(500 * MS);
        assert_eq!(t.deadline(), Some(540 * MS));
        t.fire(&mut b).expect("fire");
        assert_eq!(b.calls.last(), Some(&Call::MoveTo(path.point_at(48.0))));
        t.rebase(2_000 * MS);
        assert_eq!(t.deadline(), Some(2_004 * MS));
        b.fail_with = Some(InputError::OutsideDesktop { x: 0, y: 0 });
        assert!(t.fire(&mut b).is_err());
    }

    proptest! {
        #[test]
        fn every_point_is_inside_the_area(
            x in -2000i32..2000, y in -2000i32..2000,
            w in 16u32..2000, h in 16u32..1200,
            spacing in 8u32..=130,
            looped: bool,
            s in 0.0f64..1e6,
        ) {
            let area = Area { x, y, w, h };
            let p = ScanPath::new(area, spacing, looped);
            let at = p.point_at(s);
            prop_assert!(at.x >= x && at.x < x + w as i32);
            prop_assert!(at.y >= y && at.y < y + h as i32);
        }

        #[test]
        fn length_matches_the_documented_formula(
            w in 16u32..2000, h in 16u32..1200, spacing in 8u32..=130,
        ) {
            let p = ScanPath::new(Area { x: 0, y: 0, w, h }, spacing, false);
            let rows = f64::from(p.rows());
            let expected = rows * f64::from(w - 1) + (rows - 1.0) * f64::from(spacing);
            prop_assert!((p.length() - expected).abs() < 1e-6);
        }
    }
}
