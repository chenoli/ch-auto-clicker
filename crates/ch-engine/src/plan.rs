use ch_core::engine::clock::NANOS_PER_MS;
use ch_core::engine::scheduler::Scheduler;
use ch_core::engine::timeline::{IntervalTimeline, StopTimeline, Timeline};
use ch_core::geometry::{DisplayLayout, Point, Rect};
use ch_core::input::{InputBackend, TargetGuard};
use ch_core::model::{Mode, Position, Profile, ScanAction};
use ch_core::scan::{ScanPath, ScanTimeline};
use ch_core::validate::{ValidationError, validate_profile};
use ch_core::wiggle::{MOTION_INTERVAL_NS, WiggleTimeline};

use crate::engine::RunPlan;

const NANOS_PER_SEC: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("profile is invalid: {0:?}")]
    Invalid(Vec<ValidationError>),
    #[error("the points list is empty")]
    NoPoints,
    #[error("point ({x}, {y}) is outside the screens")]
    PointsOffscreen { x: i32, y: i32 },
    #[error("no scan area selected")]
    NoArea,
    #[error("the scan area is outside the screens")]
    AreaOffscreen,
}

impl PlanError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "PROFILE_INVALID",
            Self::NoPoints => "NO_POINTS",
            Self::PointsOffscreen { .. } => "POINTS_OFFSCREEN",
            Self::NoArea => "NO_AREA",
            Self::AreaOffscreen => "AREA_OFFSCREEN",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screens {
    pub desktop: Rect,
    pub monitors: Vec<Rect>,
}

impl From<&DisplayLayout> for Screens {
    fn from(layout: &DisplayLayout) -> Self {
        Self {
            desktop: layout.virtual_desktop,
            monitors: layout.monitors.iter().map(|m| m.rect).collect(),
        }
    }
}

enum Motion {
    None,
    Wiggle { amplitude_pct: f64, speed_hz: u8 },
    Scan { path: ScanPath, speed_px_s: u32 },
}

pub fn build_plan(
    profile: &Profile,
    countdown_sec: u8,
    screens: Screens,
    guard: Box<dyn TargetGuard>,
) -> Result<RunPlan, PlanError> {
    let errors = validate_profile(profile);
    if !errors.is_empty() {
        return Err(PlanError::Invalid(errors));
    }
    let desktop = screens.desktop;
    let mut motion = Motion::None;
    let mut clicks = true;
    let points: Vec<(Point, u64)> = match (profile.mode, profile.modes.click.position) {
        (Mode::Click, Position::Cursor) => Vec::new(),
        (Mode::Click, Position::Points) => {
            let points = &profile.modes.click.points;
            if points.is_empty() {
                return Err(PlanError::NoPoints);
            }
            if let Some(p) = points
                .iter()
                .find(|p| !desktop.contains(Point::new(p.x, p.y)))
            {
                return Err(PlanError::PointsOffscreen { x: p.x, y: p.y });
            }
            points
                .iter()
                .map(|p| (Point::new(p.x, p.y), u64::from(p.delay_ms) * NANOS_PER_MS))
                .collect()
        }
        (Mode::Wiggle, _) => {
            let w = &profile.modes.wiggle;
            motion = Motion::Wiggle {
                amplitude_pct: w.amplitude_pct,
                speed_hz: w.speed_hz,
            };
            clicks = w.click_while_wiggling;
            Vec::new()
        }
        (Mode::Scan, _) => {
            let s = &profile.modes.scan;
            let area = s.area.ok_or(PlanError::NoArea)?;
            let rect = Rect::new(
                area.x,
                area.y,
                i32::try_from(area.w).unwrap_or(i32::MAX),
                i32::try_from(area.h).unwrap_or(i32::MAX),
            );
            if !desktop.contains_rect(&rect) {
                return Err(PlanError::AreaOffscreen);
            }
            motion = Motion::Scan {
                path: ScanPath::new(area, s.line_spacing_px, s.loop_),
                speed_px_s: s.speed_px_s,
            };
            clicks = s.action == ScanAction::MoveClick;
            Vec::new()
        }
    };

    let click = profile.click.clone();
    let intervals_ns: Vec<u64> = click
        .intervals_ms()
        .into_iter()
        .map(|ms| u64::from(ms) * NANOS_PER_MS)
        .collect();
    let nominal = if clicks {
        u64::from(click.min_interval_ms()) * NANOS_PER_MS
    } else {
        MOTION_INTERVAL_NS
    };
    let monitors = screens.monitors;
    let stop_after_ns = profile
        .stop_after
        .enabled
        .then(|| u64::from(profile.stop_after.seconds) * NANOS_PER_SEC);
    let countdown_ns = u64::from(countdown_sec) * NANOS_PER_SEC;

    Ok(RunPlan::new(
        countdown_ns,
        nominal,
        stop_after_ns,
        move |start, shared| {
            let clicks_per_tick = u64::from(click.kind.clicks());
            let mut timelines: Vec<Box<dyn Timeline>> = Vec::new();
            match motion {
                Motion::None => {}
                Motion::Wiggle {
                    amplitude_pct,
                    speed_hz,
                } => timelines.push(Box::new(WiggleTimeline::new(
                    start,
                    monitors,
                    amplitude_pct,
                    speed_hz,
                ))),
                Motion::Scan { path, speed_px_s } => {
                    timelines.push(Box::new(ScanTimeline::new(start, path, speed_px_s)));
                }
            }
            let click_timeline: Box<dyn Timeline> = if points.is_empty() {
                let action = Box::new(move |backend: &mut dyn InputBackend| {
                    if guard.blocks_click() {
                        shared.add_skipped_self();
                        return Ok(());
                    }
                    let elevated = guard.target_is_elevated();
                    shared.set_blocked_by_admin(elevated);
                    if elevated {
                        return Ok(());
                    }
                    backend.click(click.button, click.kind)?;
                    shared.add_clicks(clicks_per_tick);
                    Ok(())
                });
                Box::new(IntervalTimeline::new(
                    start,
                    intervals_ns.into_iter().cycle(),
                    action,
                ))
            } else {
                let delays: Vec<u64> = points.iter().map(|(_, d)| *d).collect();
                let mut cycle = intervals_ns.into_iter().cycle();
                let gaps = (1usize..)
                    .map(move |n| cycle.next().unwrap_or(NANOS_PER_MS) + delays[n % delays.len()]);
                let targets: Vec<Point> = points.iter().map(|(p, _)| *p).collect();
                let mut next = 0usize;
                let action = Box::new(move |backend: &mut dyn InputBackend| {
                    let target = targets[next];
                    next = (next + 1) % targets.len();
                    backend.move_to(target)?;
                    if guard.blocks_click() {
                        shared.add_skipped_self();
                        return Ok(());
                    }
                    let elevated = guard.target_is_elevated();
                    shared.set_blocked_by_admin(elevated);
                    if elevated {
                        return Ok(());
                    }
                    backend.click(click.button, click.kind)?;
                    shared.add_clicks(clicks_per_tick);
                    Ok(())
                });
                Box::new(IntervalTimeline::new(start, gaps, action))
            };
            if clicks {
                timelines.push(click_timeline);
            }
            if let Some(after) = stop_after_ns {
                timelines.push(Box::new(StopTimeline::new(start + after)) as Box<dyn Timeline>);
            }
            Scheduler::new(timelines)
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    use ch_core::engine::clock::FakeClock;
    use ch_core::engine::run::{Poll, RunControl, RunEnd, run};
    use ch_core::engine::scheduler::FireReport;
    use ch_core::engine::wait::fake::FakeWaiter;
    use ch_core::input::fake::{Call, RecordingBackend};
    use ch_core::input::{AllowAll, ClickKind, MouseButton};
    use ch_core::model::{Area, ScreenPoint};

    use super::*;
    use crate::engine::Shared;

    const MS: u64 = NANOS_PER_MS;
    const DESK: Rect = Rect::new(-1680, 0, 3600, 1080);

    fn desk() -> Screens {
        Screens {
            desktop: DESK,
            monitors: vec![Rect::new(-1680, 0, 1680, 1050), Rect::new(0, 0, 1920, 1080)],
        }
    }

    struct Fires(Vec<u64>, FakeClock, usize);

    impl RunControl for Fires {
        fn poll(&mut self) -> Poll {
            if self.0.len() >= self.2 {
                Poll::Stop
            } else {
                Poll::Continue
            }
        }
        fn on_fire(&mut self, _r: &FireReport) {
            self.0.push(self.1.peek());
        }
    }

    fn profile() -> Profile {
        Profile::new_default("p", "Teste", "2026-10-04T00:00:00Z")
    }

    fn simulate(plan: RunPlan, fires: usize) -> (Vec<Call>, Vec<u64>, Arc<Shared>, RunEnd) {
        let shared = Arc::new(Shared::default());
        let clock = FakeClock::with_auto_step(1_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        let mut scheduler = (plan.build_for_test())(0, Arc::clone(&shared));
        let mut backend = RecordingBackend::default();
        let mut control = Fires(Vec::new(), clock.clone(), fires);
        let end = run(
            &mut scheduler,
            &clock,
            &mut waiter,
            &mut backend,
            &mut control,
        );
        (backend.calls, control.0, shared, end)
    }

    #[test]
    fn clicks_at_the_fixed_interval_with_the_chosen_button() {
        let mut p = profile();
        p.click.button = MouseButton::Right;
        p.click.kind = ClickKind::Double;
        let plan = build_plan(&p, 3, desk(), Box::new(AllowAll)).expect("plan");
        assert_eq!(plan.countdown_ns, 3_000 * MS);
        assert_eq!(plan.nominal_interval_ns, 100 * MS);
        assert_eq!(plan.stop_after_ns, None);

        let (calls, times, shared, _) = simulate(plan, 4);
        assert_eq!(
            calls,
            vec![Call::Click(MouseButton::Right, ClickKind::Double); 4]
        );
        let gaps: Vec<_> = times
            .windows(2)
            .map(|w| (w[1] - w[0] + MS / 2) / MS)
            .collect();
        assert_eq!(gaps, vec![100, 100, 100]);
        assert_eq!(shared.clicks(), 8, "double click counts two per tick");
    }

    #[test]
    fn follows_the_variation_cycle() {
        let mut p = profile();
        p.click.variation.enabled = true;
        let plan = build_plan(&p, 0, desk(), Box::new(AllowAll)).expect("plan");
        assert_eq!(plan.nominal_interval_ns, MS, "shortest block");
        let (_, times, _, _) = simulate(plan, 5);
        let gaps: Vec<_> = times
            .windows(2)
            .map(|w| (w[1] - w[0] + MS / 2) / MS)
            .collect();
        assert_eq!(gaps, vec![1, 35, 70, 1]);
    }

    #[test]
    fn the_stop_timer_ends_the_run() {
        let mut p = profile();
        p.stop_after.enabled = true;
        p.stop_after.seconds = 30;
        p.click.interval_ms = 10_000;
        let plan = build_plan(&p, 0, desk(), Box::new(AllowAll)).expect("plan");
        assert_eq!(plan.stop_after_ns, Some(30_000 * MS));
        let (calls, _, _, end) = simulate(plan, usize::MAX);
        assert_eq!(end, RunEnd::TimerElapsed);
        assert_eq!(
            calls.len(),
            4,
            "clicks at 0, 10, 20 and 30 s; the click at 30 s goes first"
        );
    }

    #[test]
    fn skips_clicks_over_our_own_window() {
        struct Toggle(Arc<AtomicBool>);
        impl TargetGuard for Toggle {
            fn blocks_click(&self) -> bool {
                self.0.load(Ordering::SeqCst)
            }
        }
        let blocked = Arc::new(AtomicBool::new(true));
        let plan = build_plan(
            &profile(),
            0,
            desk(),
            Box::new(Toggle(Arc::clone(&blocked))),
        )
        .expect("plan");
        let (calls, _, shared, _) = simulate(plan, 3);
        assert!(calls.is_empty(), "nothing injected while over our window");
        assert_eq!(shared.clicks(), 0);
        assert_eq!(shared.snapshot(0).skipped_self, 3);
    }

    #[test]
    fn multi_point_moves_then_clicks_each_point_in_a_loop() {
        let mut p = profile();
        p.modes.click.position = Position::Points;
        p.modes.click.points = vec![
            ScreenPoint {
                id: "a".into(),
                x: 10,
                y: 20,
                delay_ms: 0,
            },
            ScreenPoint {
                id: "b".into(),
                x: -840,
                y: 610,
                delay_ms: 120,
            },
        ];
        let plan = build_plan(&p, 0, desk(), Box::new(AllowAll)).expect("plan");
        let (calls, times, shared, _) = simulate(plan, 4);
        let click = Call::Click(MouseButton::Left, ClickKind::Single);
        assert_eq!(
            calls,
            vec![
                Call::MoveTo(ch_core::geometry::Point::new(10, 20)),
                click.clone(),
                Call::MoveTo(ch_core::geometry::Point::new(-840, 610)),
                click.clone(),
                Call::MoveTo(ch_core::geometry::Point::new(10, 20)),
                click.clone(),
                Call::MoveTo(ch_core::geometry::Point::new(-840, 610)),
                click,
            ]
        );
        let gaps: Vec<_> = times
            .windows(2)
            .map(|w| (w[1] - w[0] + MS / 2) / MS)
            .collect();
        assert_eq!(
            gaps,
            vec![220, 100, 220],
            "the delay of the point being reached is added to the interval"
        );
        assert_eq!(shared.clicks(), 4);
    }

    #[test]
    fn multi_point_skips_points_over_our_window() {
        struct Always;
        impl TargetGuard for Always {
            fn blocks_click(&self) -> bool {
                true
            }
        }
        let mut p = profile();
        p.modes.click.position = Position::Points;
        p.modes.click.points = vec![ScreenPoint {
            id: "a".into(),
            x: 1,
            y: 1,
            delay_ms: 0,
        }];
        let plan = build_plan(&p, 0, desk(), Box::new(Always)).expect("plan");
        let (calls, _, shared, _) = simulate(plan, 2);
        assert!(
            calls.iter().all(|c| matches!(c, Call::MoveTo(_))),
            "moves but never clicks: {calls:?}"
        );
        assert_eq!(shared.snapshot(0).skipped_self, 2);
    }

    #[test]
    fn an_elevated_target_skips_clicks_and_flags_the_run() {
        struct Elevated(Arc<AtomicBool>);
        impl TargetGuard for Elevated {
            fn blocks_click(&self) -> bool {
                false
            }
            fn target_is_elevated(&self) -> bool {
                self.0.load(Ordering::SeqCst)
            }
        }
        let elevated = Arc::new(AtomicBool::new(true));
        let plan = build_plan(
            &profile(),
            0,
            desk(),
            Box::new(Elevated(Arc::clone(&elevated))),
        )
        .expect("plan");
        let (calls, _, shared, _) = simulate(plan, 3);
        assert!(calls.is_empty(), "nothing injected: {calls:?}");
        assert_eq!(shared.clicks(), 0);
        assert!(shared.snapshot(0).blocked_by_admin);

        elevated.store(false, Ordering::SeqCst);
        let plan = build_plan(
            &profile(),
            0,
            desk(),
            Box::new(Elevated(Arc::clone(&elevated))),
        )
        .expect("plan");
        let (calls, _, shared, _) = simulate(plan, 2);
        assert_eq!(calls.len(), 2);
        assert!(!shared.snapshot(0).blocked_by_admin);
    }

    #[test]
    fn wiggle_only_moves_and_returns_to_the_origin() {
        let mut p = profile();
        p.mode = Mode::Wiggle;
        p.modes.wiggle.amplitude_pct = 1.0;
        p.modes.wiggle.speed_hz = 8;
        p.stop_after.enabled = false;
        let plan = build_plan(&p, 0, desk(), Box::new(AllowAll)).expect("plan");
        assert_eq!(plan.nominal_interval_ns, 4 * MS);
        let (calls, times, shared, end) = simulate(plan, 63);
        assert_eq!(end, RunEnd::Stopped);
        assert_eq!(shared.clicks(), 0);
        assert!(calls.iter().all(|c| matches!(c, Call::MoveTo(_))));
        assert!(times.windows(2).all(|w| w[1] - w[0] < 4 * MS + 10_000));
        let xs: Vec<i32> = calls
            .iter()
            .filter_map(|c| match c {
                Call::MoveTo(pt) => Some(pt.x),
                _ => None,
            })
            .collect();
        let (lo, hi) = (xs.iter().min().copied(), xs.iter().max().copied());
        assert_eq!((lo, hi), (Some(-840 - 8), Some(-840 - 8 + 17)));
        assert_eq!(calls.last(), Some(&Call::MoveTo(Point::new(-840, 525))));
    }

    #[test]
    fn wiggle_can_click_at_the_profile_rate() {
        let mut p = profile();
        p.mode = Mode::Wiggle;
        p.modes.wiggle.click_while_wiggling = true;
        p.click.interval_ms = 20;
        p.click.variation.enabled = false;
        p.stop_after.enabled = false;
        let plan = build_plan(&p, 0, desk(), Box::new(AllowAll)).expect("plan");
        assert_eq!(plan.nominal_interval_ns, 20 * MS);
        let (calls, _, shared, _) = simulate(plan, 60);
        let clicks = calls
            .iter()
            .filter(|c| matches!(c, Call::Click(..)))
            .count();
        assert!(clicks >= 8, "clicks {clicks}");
        assert_eq!(shared.clicks(), clicks as u64);
        assert!(calls.iter().any(|c| matches!(c, Call::MoveTo(_))));
    }

    #[test]
    fn screens_come_from_the_display_layout() {
        let layout = DisplayLayout {
            virtual_desktop: DESK,
            monitors: vec![ch_core::geometry::Monitor {
                id: "a".into(),
                rect: Rect::new(0, 0, 1920, 1080),
                work_area: Rect::new(0, 0, 1920, 1040),
                scale: 1.0,
                primary: true,
            }],
        };
        let screens = Screens::from(&layout);
        assert_eq!(screens.desktop, DESK);
        assert_eq!(screens.monitors, vec![Rect::new(0, 0, 1920, 1080)]);
    }

    #[test]
    fn rejects_invalid_profiles_and_later_modes() {
        let mut p = profile();
        p.click.interval_ms = 0;
        let err = build_plan(&p, 3, desk(), Box::new(AllowAll)).expect_err("invalid");
        assert_eq!(err.code(), "PROFILE_INVALID");

        let mut p = profile();
        p.modes.click.position = Position::Points;
        assert_eq!(
            build_plan(&p, 3, desk(), Box::new(AllowAll)).expect_err("no points"),
            PlanError::NoPoints
        );
        p.modes.click.points = vec![ScreenPoint {
            id: "far".into(),
            x: 5000,
            y: 10,
            delay_ms: 0,
        }];
        let err = build_plan(&p, 3, desk(), Box::new(AllowAll)).expect_err("offscreen");
        assert_eq!(err, PlanError::PointsOffscreen { x: 5000, y: 10 });
        assert_eq!(err.code(), "POINTS_OFFSCREEN");
        assert_eq!(PlanError::NoPoints.code(), "NO_POINTS");
        p.mode = Mode::Scan;
        let err = build_plan(&p, 3, desk(), Box::new(AllowAll)).expect_err("no area");
        assert_eq!(
            (err.code(), err.to_string().contains("area")),
            ("NO_AREA", true)
        );
        p.modes.scan.area = Some(Area {
            x: 1800,
            y: 0,
            w: 400,
            h: 100,
        });
        let err = build_plan(&p, 3, desk(), Box::new(AllowAll)).expect_err("offscreen area");
        assert_eq!(err.code(), "AREA_OFFSCREEN");
    }

    fn scan_profile(action: ScanAction, looped: bool) -> Profile {
        let mut p = profile();
        p.mode = Mode::Scan;
        p.stop_after.enabled = false;
        p.click.interval_ms = 20;
        p.click.variation.enabled = false;
        p.modes.scan.area = Some(Area {
            x: 100,
            y: 100,
            w: 101,
            h: 17,
        });
        p.modes.scan.line_spacing_px = 8;
        p.modes.scan.speed_px_s = 2400;
        p.modes.scan.action = action;
        p.modes.scan.loop_ = looped;
        p
    }

    #[test]
    fn scan_without_loop_covers_the_area_and_completes() {
        let plan = build_plan(
            &scan_profile(ScanAction::Move, false),
            0,
            desk(),
            Box::new(AllowAll),
        )
        .expect("plan");
        assert_eq!(plan.nominal_interval_ns, 4 * MS);
        let (calls, _, shared, end) = simulate(plan, 1_000);
        assert_eq!(end, RunEnd::Completed);
        assert_eq!(shared.clicks(), 0);
        assert_eq!(calls.first(), Some(&Call::MoveTo(Point::new(100, 100))));
        assert_eq!(calls.last(), Some(&Call::MoveTo(Point::new(200, 116))));
        assert!(calls.iter().all(|c| matches!(c, Call::MoveTo(p)
            if (100..=200).contains(&p.x) && (100..=116).contains(&p.y))));
    }

    #[test]
    fn scan_can_move_and_click_in_a_loop() {
        let plan = build_plan(
            &scan_profile(ScanAction::MoveClick, true),
            0,
            desk(),
            Box::new(AllowAll),
        )
        .expect("plan");
        assert_eq!(plan.nominal_interval_ns, 20 * MS);
        let (calls, _, shared, end) = simulate(plan, 300);
        assert_eq!(
            end,
            RunEnd::Stopped,
            "a looping scan only ends when stopped"
        );
        let clicks = calls
            .iter()
            .filter(|c| matches!(c, Call::Click(..)))
            .count();
        assert!(clicks > 10, "clicks {clicks}");
        assert_eq!(shared.clicks(), clicks as u64);
    }
}
