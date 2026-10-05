//! Builds a [`RunPlan`] from a profile (ARCHITECTURE §5.1, §6.1, §6.6). The profile is copied, so
//! edits during a run apply on the next start (decision D8).

use ch_core::engine::clock::NANOS_PER_MS;
use ch_core::engine::scheduler::Scheduler;
use ch_core::engine::timeline::{IntervalTimeline, StopTimeline, Timeline};
use ch_core::input::TargetGuard;
use ch_core::model::{Mode, Position, Profile};
use ch_core::validate::{ValidationError, validate_profile};

use crate::engine::RunPlan;

const NANOS_PER_SEC: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error("profile is invalid: {0:?}")]
    Invalid(Vec<ValidationError>),
    /// Modes that arrive in later phases (points: 5, wiggle: 6, scan: 7).
    #[error("{0} is not available yet")]
    NotAvailableYet(&'static str),
}

impl PlanError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "PROFILE_INVALID",
            Self::NotAvailableYet(_) => "MODE_NOT_AVAILABLE",
        }
    }
}

pub fn build_plan(
    profile: &Profile,
    countdown_sec: u8,
    guard: Box<dyn TargetGuard>,
) -> Result<RunPlan, PlanError> {
    let errors = validate_profile(profile);
    if !errors.is_empty() {
        return Err(PlanError::Invalid(errors));
    }
    match (profile.mode, profile.modes.click.position) {
        (Mode::Click, Position::Cursor) => {}
        (Mode::Click, Position::Points) => return Err(PlanError::NotAvailableYet("points")),
        (Mode::Wiggle, _) => return Err(PlanError::NotAvailableYet("wiggle")),
        (Mode::Scan, _) => return Err(PlanError::NotAvailableYet("scan")),
    }

    let click = profile.click.clone();
    let intervals_ns: Vec<u64> = click
        .intervals_ms()
        .into_iter()
        .map(|ms| u64::from(ms) * NANOS_PER_MS)
        .collect();
    let nominal = u64::from(click.min_interval_ms()) * NANOS_PER_MS;
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
            let action = Box::new(move |backend: &mut dyn ch_core::input::InputBackend| {
                if guard.blocks_click() {
                    shared.add_skipped_self();
                    return Ok(());
                }
                backend.click(click.button, click.kind)?;
                shared.add_clicks(clicks_per_tick);
                Ok(())
            });
            let mut timelines: Vec<Box<dyn Timeline>> = vec![Box::new(IntervalTimeline::new(
                start,
                intervals_ns.into_iter().cycle(),
                action,
            ))];
            if let Some(after) = stop_after_ns {
                timelines.push(Box::new(StopTimeline::new(start + after)));
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

    use super::*;
    use crate::engine::Shared;

    const MS: u64 = NANOS_PER_MS;

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

    /// Builds the plan's scheduler and runs `fires` actions on virtual time.
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
        let plan = build_plan(&p, 3, Box::new(AllowAll)).expect("plan");
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
        let plan = build_plan(&p, 0, Box::new(AllowAll)).expect("plan");
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
        let plan = build_plan(&p, 0, Box::new(AllowAll)).expect("plan");
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
        let plan = build_plan(&profile(), 0, Box::new(Toggle(Arc::clone(&blocked)))).expect("plan");
        let (calls, _, shared, _) = simulate(plan, 3);
        assert!(calls.is_empty(), "nothing injected while over our window");
        assert_eq!(shared.clicks(), 0);
        assert_eq!(shared.snapshot(0).skipped_self, 3);
    }

    #[test]
    fn rejects_invalid_profiles_and_later_modes() {
        let mut p = profile();
        p.click.interval_ms = 0;
        let err = build_plan(&p, 3, Box::new(AllowAll)).expect_err("invalid");
        assert_eq!(err.code(), "PROFILE_INVALID");

        let mut p = profile();
        p.modes.click.position = Position::Points;
        assert_eq!(
            build_plan(&p, 3, Box::new(AllowAll)).expect_err("points"),
            PlanError::NotAvailableYet("points")
        );
        p.mode = Mode::Wiggle;
        assert_eq!(
            build_plan(&p, 3, Box::new(AllowAll))
                .expect_err("wiggle")
                .code(),
            "MODE_NOT_AVAILABLE"
        );
        p.mode = Mode::Scan;
        assert!(
            build_plan(&p, 3, Box::new(AllowAll))
                .expect_err("scan")
                .to_string()
                .contains("scan")
        );
    }
}
