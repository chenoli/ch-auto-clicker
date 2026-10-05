use ch_core::engine::clock::{Clock, NANOS_PER_MS, NANOS_PER_US};
use ch_core::engine::run::{Poll, RunControl, run};
use ch_core::engine::scheduler::{FireReport, Scheduler};
use ch_core::engine::timeline::IntervalTimeline;
use ch_core::engine::wait::CoarseWaiter;
use ch_core::input::NoopBackend;
use ch_core::stats::{Histogram, JitterStats};

pub const BENCH_INTERVALS_MS: [u64; 5] = [1, 2, 4, 8, 16];
pub const DEFAULT_MIN_RELIABLE_MS: u64 = 4;
const MAX_P99_FRACTION: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalResult {
    pub interval_ms: u64,
    pub stats: JitterStats,
}

impl IntervalResult {
    pub fn is_reliable(&self) -> bool {
        self.stats.missed == 0
            && self.stats.p99_us < self.interval_ms as f64 * 1_000.0 * MAX_P99_FRACTION
    }
}

struct Collect(Histogram);

impl RunControl for Collect {
    fn poll(&mut self) -> Poll {
        Poll::Continue
    }

    fn on_fire(&mut self, report: &FireReport) {
        self.0.record(report.lateness_ns);
        self.0.record_missed(report.missed);
    }
}

pub fn measure<C: Clock + ?Sized, W: CoarseWaiter + ?Sized>(
    clock: &C,
    waiter: &mut W,
    interval_ms: u64,
    duration_ns: u64,
) -> IntervalResult {
    let interval = interval_ms * NANOS_PER_MS;
    let fires = (duration_ns / interval).max(1);
    let timeline = IntervalTimeline::noop(clock.now() + interval, interval).with_limit(fires);
    let mut scheduler = Scheduler::new(vec![Box::new(timeline)]);
    let mut collect = Collect(Histogram::default());
    run(
        &mut scheduler,
        clock,
        waiter,
        &mut NoopBackend,
        &mut collect,
    );
    IntervalResult {
        interval_ms,
        stats: collect.0.stats(),
    }
}

pub fn min_reliable_interval_ms(results: &[IntervalResult]) -> Option<u64> {
    let mut sorted: Vec<_> = results.to_vec();
    sorted.sort_by_key(|r| r.interval_ms);
    let mut best = None;
    for r in sorted.iter().rev() {
        if !r.is_reliable() {
            break;
        }
        best = Some(r.interval_ms);
    }
    best
}

pub fn lateness_us(ns: u64) -> f64 {
    ns as f64 / NANOS_PER_US as f64
}

#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkReport {
    pub timer_kind: &'static str,
    pub mmcss: bool,
    pub results: Vec<IntervalResult>,
    pub min_reliable_ms: Option<u64>,
}

#[cfg(windows)]
pub fn run_system_benchmark(
    intervals_ms: &[u64],
    per_interval: std::time::Duration,
) -> windows::core::Result<BenchmarkReport> {
    use ch_win::timing::{HighResWaiter, QpcClock, RealtimeGuard};

    let intervals = intervals_ms.to_vec();
    std::thread::Builder::new()
        .name("ch-benchmark".into())
        .spawn(move || {
            let guard = RealtimeGuard::acquire();
            let clock = QpcClock::new();
            let mut waiter = HighResWaiter::new()?;
            let duration = per_interval.as_nanos() as u64;
            let results: Vec<_> = intervals
                .iter()
                .map(|&ms| measure(&clock, &mut waiter, ms, duration))
                .collect();
            Ok(BenchmarkReport {
                timer_kind: waiter.kind().as_str(),
                mmcss: guard.mmcss_active(),
                min_reliable_ms: min_reliable_interval_ms(&results),
                results,
            })
        })
        .expect("failed to spawn the benchmark thread")
        .join()
        .unwrap_or_else(|_| Err(windows::core::Error::empty()))
}

#[cfg(test)]
mod tests {
    use ch_core::engine::clock::FakeClock;
    use ch_core::engine::wait::fake::FakeWaiter;

    use super::*;

    fn result(interval_ms: u64, p99_us: f64, missed: u64) -> IntervalResult {
        IntervalResult {
            interval_ms,
            stats: JitterStats {
                samples: 100,
                p99_us,
                missed,
                ..JitterStats::default()
            },
        }
    }

    #[test]
    fn reliability_requires_low_p99_and_nothing_missed() {
        assert!(result(4, 999.0, 0).is_reliable());
        assert!(
            !result(4, 1_000.0, 0).is_reliable(),
            "25% of 4 ms is the limit"
        );
        assert!(!result(16, 10.0, 1).is_reliable());
    }

    #[test]
    fn picks_the_shortest_interval_with_every_longer_one_passing() {
        let results = [
            result(1, 600.0, 0),
            result(2, 100.0, 0),
            result(4, 2_000.0, 0),
            result(8, 100.0, 0),
            result(16, 100.0, 0),
        ];
        assert_eq!(min_reliable_interval_ms(&results), Some(8));
        assert_eq!(
            min_reliable_interval_ms(&[result(1, 10.0, 0), result(2, 10.0, 0)]),
            Some(1)
        );
        assert_eq!(min_reliable_interval_ms(&[result(16, 9_999.0, 0)]), None);
        assert_eq!(min_reliable_interval_ms(&[]), None);
    }

    #[test]
    fn measure_runs_the_requested_number_of_fires() {
        let clock = FakeClock::with_auto_step(50_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        let r = measure(&clock, &mut waiter, 2, 100 * NANOS_PER_MS);
        assert_eq!(r.interval_ms, 2);
        assert_eq!(r.stats.samples, 50);
        assert_eq!(r.stats.missed, 0);
        assert!(lateness_us(2_500) == 2.5);
    }

    #[cfg(windows)]
    #[test]
    fn system_benchmark_measures_this_machine() {
        let report = run_system_benchmark(&[2, 8], std::time::Duration::from_millis(300))
            .expect("benchmark");
        assert_eq!(report.results.len(), 2);
        assert_eq!(report.timer_kind, "highResolution");
        assert!(report.mmcss);
        assert!(report.results.iter().all(|r| r.stats.samples > 0));
        assert!(
            report.min_reliable_ms.is_some(),
            "8 ms must be reliable on any desktop: {report:?}"
        );
    }
}
