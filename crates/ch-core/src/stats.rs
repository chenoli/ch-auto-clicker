//! Lateness statistics for the scheduler (ARCHITECTURE §5.3): a log-bucketed histogram, a
//! lock-free variant the engine thread writes to, and the "timing degraded" detector.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use crate::engine::clock::NANOS_PER_US;

const SUB_BUCKETS: usize = 8;
/// Powers of two in microseconds covered before the overflow bucket (2^21 µs ≈ 2.1 s).
const MAX_POW: usize = 21;
/// Bucket 0 holds sub-microsecond samples; the last one holds everything past the range.
pub const BUCKETS: usize = 1 + MAX_POW * SUB_BUCKETS + 1;

fn bucket_of(ns: u64) -> usize {
    let us = ns / NANOS_PER_US;
    if us == 0 {
        return 0;
    }
    let pow = (63 - us.leading_zeros()) as usize;
    if pow >= MAX_POW {
        return BUCKETS - 1;
    }
    let base = 1u64 << pow;
    let sub = ((us - base) * SUB_BUCKETS as u64 / base) as usize;
    1 + pow * SUB_BUCKETS + sub
}

/// Exclusive upper bound of a bucket in microseconds (percentiles report this conservative edge).
fn bucket_upper_us(index: usize) -> u64 {
    if index == 0 {
        return 1;
    }
    if index >= BUCKETS - 1 {
        return 1u64 << MAX_POW;
    }
    let pow = (index - 1) / SUB_BUCKETS;
    let sub = ((index - 1) % SUB_BUCKETS) as u64;
    let base = 1u64 << pow;
    base + (base * (sub + 1)).div_ceil(SUB_BUCKETS as u64)
}

/// Summary sent to the UI once per second while running.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JitterStats {
    pub samples: u64,
    pub p50_us: f64,
    pub p95_us: f64,
    pub p99_us: f64,
    pub max_us: f64,
    pub missed: u64,
}

#[derive(Debug, Clone)]
pub struct Histogram {
    counts: [u64; BUCKETS],
    total: u64,
    max_ns: u64,
    missed: u64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self {
            counts: [0; BUCKETS],
            total: 0,
            max_ns: 0,
            missed: 0,
        }
    }
}

impl Histogram {
    pub fn record(&mut self, lateness_ns: u64) {
        self.counts[bucket_of(lateness_ns)] += 1;
        self.total += 1;
        self.max_ns = self.max_ns.max(lateness_ns);
    }

    pub fn record_missed(&mut self, events: u64) {
        self.missed += events;
    }

    pub fn count(&self) -> u64 {
        self.total
    }

    pub fn missed(&self) -> u64 {
        self.missed
    }

    pub fn max_us(&self) -> f64 {
        self.max_ns as f64 / NANOS_PER_US as f64
    }

    /// Upper edge of the bucket holding the `p`-th percentile (0.0..=1.0), in microseconds.
    pub fn percentile_us(&self, p: f64) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        let rank = ((p.clamp(0.0, 1.0) * self.total as f64).ceil() as u64).max(1);
        let mut seen = 0;
        for (index, &count) in self.counts.iter().enumerate() {
            seen += count;
            if seen >= rank {
                return (bucket_upper_us(index) as f64).min(self.max_us().max(1.0));
            }
        }
        self.max_us()
    }

    pub fn merge(&mut self, other: &Histogram) {
        for (a, b) in self.counts.iter_mut().zip(other.counts.iter()) {
            *a += b;
        }
        self.total += other.total;
        self.max_ns = self.max_ns.max(other.max_ns);
        self.missed += other.missed;
    }

    pub fn stats(&self) -> JitterStats {
        JitterStats {
            samples: self.total,
            p50_us: self.percentile_us(0.50),
            p95_us: self.percentile_us(0.95),
            p99_us: self.percentile_us(0.99),
            max_us: self.max_us(),
            missed: self.missed,
        }
    }
}

/// Written by the engine thread without locks or allocation; drained by the telemetry thread.
#[derive(Debug)]
pub struct AtomicHistogram {
    counts: [AtomicU32; BUCKETS],
    max_ns: AtomicU64,
    missed: AtomicU64,
}

impl Default for AtomicHistogram {
    fn default() -> Self {
        Self {
            counts: std::array::from_fn(|_| AtomicU32::new(0)),
            max_ns: AtomicU64::new(0),
            missed: AtomicU64::new(0),
        }
    }
}

impl AtomicHistogram {
    pub fn record(&self, lateness_ns: u64) {
        self.counts[bucket_of(lateness_ns)].fetch_add(1, Ordering::Relaxed);
        self.max_ns.fetch_max(lateness_ns, Ordering::Relaxed);
    }

    pub fn record_missed(&self, events: u64) {
        self.missed.fetch_add(events, Ordering::Relaxed);
    }

    /// Returns everything recorded since the previous drain and resets the counters.
    pub fn drain(&self) -> Histogram {
        let mut out = Histogram::default();
        for (slot, counter) in out.counts.iter_mut().zip(self.counts.iter()) {
            *slot = u64::from(counter.swap(0, Ordering::Relaxed));
        }
        out.total = out.counts.iter().sum();
        out.max_ns = self.max_ns.swap(0, Ordering::Relaxed);
        out.missed = self.missed.swap(0, Ordering::Relaxed);
        out
    }
}

/// Flags sustained timing trouble: p95 above half the interval for 2 consecutive samples turns it
/// on; 5 consecutive good samples turn it off again (ARCHITECTURE §5.3).
#[derive(Debug, Default)]
pub struct DegradedDetector {
    degraded: bool,
    bad_streak: u32,
    good_streak: u32,
}

impl DegradedDetector {
    const BAD_TO_RAISE: u32 = 2;
    const GOOD_TO_CLEAR: u32 = 5;

    pub fn is_degraded(&self) -> bool {
        self.degraded
    }

    /// Feeds one per-second sample. Returns the new state only when it changes.
    pub fn update(&mut self, p95_us: f64, interval_us: f64) -> Option<bool> {
        let bad = interval_us > 0.0 && p95_us > interval_us / 2.0;
        if bad {
            self.bad_streak += 1;
            self.good_streak = 0;
        } else {
            self.good_streak += 1;
            self.bad_streak = 0;
        }
        if !self.degraded && self.bad_streak >= Self::BAD_TO_RAISE {
            self.degraded = true;
            return Some(true);
        }
        if self.degraded && self.good_streak >= Self::GOOD_TO_CLEAR {
            self.degraded = false;
            return Some(false);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    const US: u64 = NANOS_PER_US;

    #[test]
    fn buckets_are_monotonic_and_cover_the_range() {
        let mut previous = 0;
        for us in [
            0u64, 1, 2, 3, 7, 8, 100, 1_000, 65_536, 1_000_000, 2_000_000,
        ] {
            let b = bucket_of(us * US);
            assert!(b >= previous, "{us} µs");
            assert!(
                us < bucket_upper_us(b) || b == BUCKETS - 1,
                "{us} µs below its bucket edge"
            );
            previous = b;
        }
        assert_eq!(bucket_of(10_000_000 * US), BUCKETS - 1);
        assert_eq!(bucket_upper_us(BUCKETS - 1), 1 << MAX_POW);
    }

    #[test]
    fn empty_histogram_reports_zeros() {
        assert_eq!(Histogram::default().stats(), JitterStats::default());
    }

    #[test]
    fn summarizes_samples_and_missed_events() {
        let mut h = Histogram::default();
        for _ in 0..98 {
            h.record(50 * US);
        }
        h.record(900 * US);
        h.record(4_000 * US);
        h.record_missed(3);
        let s = h.stats();
        assert_eq!(s.samples, 100);
        assert_eq!(s.missed, 3);
        assert!((50.0..=56.0).contains(&s.p50_us), "p50 {}", s.p50_us);
        assert!((50.0..=56.0).contains(&s.p95_us), "p95 {}", s.p95_us);
        assert!((900.0..=1_024.0).contains(&s.p99_us), "p99 {}", s.p99_us);
        assert_eq!(s.max_us, 4_000.0);
    }

    #[test]
    fn merge_adds_everything() {
        let mut a = Histogram::default();
        a.record(10 * US);
        let mut b = Histogram::default();
        b.record(2_000 * US);
        b.record_missed(1);
        a.merge(&b);
        assert_eq!(a.count(), 2);
        assert_eq!(a.missed(), 1);
        assert_eq!(a.max_us(), 2_000.0);
    }

    #[test]
    fn atomic_histogram_drains_and_resets() {
        let h = AtomicHistogram::default();
        h.record(5 * US);
        h.record(700 * US);
        h.record_missed(2);
        let first = h.drain();
        assert_eq!(first.count(), 2);
        assert_eq!(first.missed(), 2);
        assert_eq!(first.max_us(), 700.0);
        let second = h.drain();
        assert_eq!(second.count(), 0);
        assert_eq!(second.max_us(), 0.0);
    }

    #[test]
    fn degraded_needs_two_bad_seconds_and_five_good_ones() {
        let mut d = DegradedDetector::default();
        assert_eq!(d.update(600.0, 1_000.0), None);
        assert_eq!(d.update(100.0, 1_000.0), None, "streak broken");
        assert_eq!(d.update(600.0, 1_000.0), None);
        assert_eq!(d.update(700.0, 1_000.0), Some(true));
        assert!(d.is_degraded());
        for _ in 0..4 {
            assert_eq!(d.update(100.0, 1_000.0), None);
        }
        assert_eq!(d.update(100.0, 1_000.0), Some(false));
        assert!(!d.is_degraded());
        assert_eq!(
            d.update(900.0, 0.0),
            None,
            "no interval means nothing to compare"
        );
    }

    proptest! {
        // Percentiles stay within one bucket (12.5% + 1 µs) of the exact value.
        #[test]
        fn percentile_error_is_below_one_bucket(
            samples in prop::collection::vec(0u64..500_000, 1..400),
            p in 0.01f64..1.0,
        ) {
            let mut h = Histogram::default();
            for &us in &samples {
                h.record(us * US);
            }
            let mut sorted = samples.clone();
            sorted.sort_unstable();
            let rank = ((p * sorted.len() as f64).ceil() as usize).max(1);
            let exact = sorted[rank - 1] as f64;
            let reported = h.percentile_us(p);
            prop_assert!(reported >= exact.min(h.max_us()), "reported {reported} < exact {exact}");
            prop_assert!(reported <= exact * 1.125 + 1.0, "reported {reported} > exact {exact} + 1 bucket");
        }
    }
}
