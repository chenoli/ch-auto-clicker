use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use ch_core::engine::clock::{FakeClock, NANOS_PER_MS};
use ch_core::engine::run::{Poll, RunControl, RunEnd, run};
use ch_core::engine::scheduler::{FireReport, Scheduler};
use ch_core::engine::timeline::IntervalTimeline;
use ch_core::engine::wait::fake::FakeWaiter;
use ch_core::input::NoopBackend;
use ch_core::stats::AtomicHistogram;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: delegates every operation to the system allocator unchanged.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.with(Cell::get) {
            ALLOCATIONS.with(|a| a.set(a.get() + 1));
        }
        // SAFETY: same contract as the caller's.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: same contract as the caller's.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

struct EngineLikeControl<'a> {
    histogram: &'a AtomicHistogram,
    fires: u64,
    stop_counting_at: u64,
}

impl RunControl for EngineLikeControl<'_> {
    fn poll(&mut self) -> Poll {
        Poll::Continue
    }

    fn on_fire(&mut self, report: &FireReport) {
        self.histogram.record(report.lateness_ns);
        self.histogram.record_missed(report.missed);
        self.fires += 1;
        if self.fires == 10 {
            COUNTING.with(|c| c.set(true));
        }
        if self.fires == self.stop_counting_at {
            COUNTING.with(|c| c.set(false));
        }
    }
}

#[test]
fn the_run_loop_does_not_allocate() {
    let clock = FakeClock::with_auto_step(50_000);
    let mut waiter = FakeWaiter::new(clock.clone());
    let mut scheduler = Scheduler::new(vec![
        Box::new(IntervalTimeline::noop(0, NANOS_PER_MS).with_limit(20_000)),
        Box::new(
            IntervalTimeline::new(
                0,
                [1, 35, 70].map(|ms| ms * NANOS_PER_MS).into_iter().cycle(),
                Box::new(|_| Ok(())),
            )
            .with_limit(500),
        ),
    ]);
    let histogram = AtomicHistogram::default();
    let mut control = EngineLikeControl {
        histogram: &histogram,
        fires: 0,
        stop_counting_at: 15_000,
    };

    let end = run(
        &mut scheduler,
        &clock,
        &mut waiter,
        &mut NoopBackend,
        &mut control,
    );

    assert_eq!(end, RunEnd::Completed);
    assert!(control.fires >= 15_000);
    assert_eq!(
        ALLOCATIONS.with(Cell::get),
        0,
        "allocations in the steady-state loop"
    );
}
