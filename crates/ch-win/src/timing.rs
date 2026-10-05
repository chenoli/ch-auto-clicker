//! High-resolution timing (ARCHITECTURE §5.2): QPC clock, a waitable timer that also wakes on a
//! signal, and real-time priority for the engine thread.

use std::sync::Arc;

use ch_core::engine::clock::Clock;
use ch_core::engine::wait::{CoarseWaiter, Wake, Waker};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};
use windows::Win32::System::Threading::{
    AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW,
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, GetCurrentThread,
    INFINITE, SetEvent, SetThreadPriority, SetWaitableTimer, THREAD_PRIORITY_TIME_CRITICAL,
    TIMER_ALL_ACCESS, WaitForMultipleObjects,
};
use windows::core::{PCWSTR, w};

const NANOS_PER_SEC: u128 = 1_000_000_000;
/// Waitable timers count in 100 ns units; negative due times are relative.
const NANOS_PER_TIMER_TICK: u64 = 100;

#[derive(Debug, Clone, Copy)]
pub struct QpcClock {
    frequency: u64,
}

impl QpcClock {
    pub fn new() -> Self {
        let mut frequency = 0i64;
        // SAFETY: writes the counter frequency into a valid local; cannot fail on Windows XP+.
        let _ = unsafe { QueryPerformanceFrequency(&mut frequency) };
        Self {
            frequency: frequency.max(1) as u64,
        }
    }
}

impl Default for QpcClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for QpcClock {
    fn now(&self) -> u64 {
        let mut counter = 0i64;
        // SAFETY: writes the current counter into a valid local; cannot fail on Windows XP+.
        let _ = unsafe { QueryPerformanceCounter(&mut counter) };
        ((counter as u128 * NANOS_PER_SEC) / u128::from(self.frequency)) as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerKind {
    /// `CREATE_WAITABLE_TIMER_HIGH_RESOLUTION` (Windows 10 1803+).
    HighResolution,
    /// Plain waitable timer with the system tick raised to 1 ms via `timeBeginPeriod`.
    Legacy,
}

impl TimerKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HighResolution => "highResolution",
            Self::Legacy => "legacy",
        }
    }
}

#[derive(Debug)]
struct OwnedHandle(HANDLE);

// SAFETY: kernel object handles may be used from any thread; the owner closes it exactly once.
unsafe impl Send for OwnedHandle {}
// SAFETY: SetEvent/WaitFor* on the same handle from several threads is supported by the kernel.
unsafe impl Sync for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        // SAFETY: the handle was created by this module and is closed only here.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Sleeps on a waitable timer and wakes early when its event is signalled from another thread.
#[derive(Debug)]
pub struct HighResWaiter {
    clock: QpcClock,
    timer: OwnedHandle,
    wake_event: Arc<OwnedHandle>,
    kind: TimerKind,
    raised_tick: bool,
}

impl HighResWaiter {
    pub fn new() -> windows::core::Result<Self> {
        // SAFETY: unnamed timer and event with default security; both handles are owned below.
        unsafe {
            let (timer, kind) = match CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            ) {
                Ok(t) => (t, TimerKind::HighResolution),
                Err(_) => (
                    CreateWaitableTimerExW(None, PCWSTR::null(), 0, TIMER_ALL_ACCESS.0)?,
                    TimerKind::Legacy,
                ),
            };
            let timer = OwnedHandle(timer);
            let wake_event = Arc::new(OwnedHandle(CreateEventW(
                None,
                false,
                false,
                PCWSTR::null(),
            )?));
            let raised_tick = kind == TimerKind::Legacy && timeBeginPeriod(1) == 0;
            Ok(Self {
                clock: QpcClock::new(),
                timer,
                wake_event,
                kind,
                raised_tick,
            })
        }
    }

    pub fn kind(&self) -> TimerKind {
        self.kind
    }
}

impl Drop for HighResWaiter {
    fn drop(&mut self) {
        if self.raised_tick {
            // SAFETY: balances the timeBeginPeriod(1) taken in `new`.
            unsafe { timeEndPeriod(1) };
        }
    }
}

impl CoarseWaiter for HighResWaiter {
    fn sleep_until(&mut self, deadline: u64) -> Wake {
        let now = self.clock.now();
        if deadline <= now {
            return Wake::Deadline;
        }
        let due = -(((deadline - now) / NANOS_PER_TIMER_TICK).max(1) as i64);
        // SAFETY: both handles are alive for the whole call; `due` outlives SetWaitableTimer.
        unsafe {
            if SetWaitableTimer(self.timer.0, &due, 0, None, None, false).is_err() {
                return Wake::Deadline;
            }
            let result =
                WaitForMultipleObjects(&[self.timer.0, self.wake_event.0], false, INFINITE);
            if result.0 == WAIT_OBJECT_0.0 + 1 {
                Wake::Signal
            } else {
                Wake::Deadline
            }
        }
    }

    fn waker(&self) -> Waker {
        let event = Arc::clone(&self.wake_event);
        Arc::new(move || {
            // SAFETY: the event handle is kept alive by the Arc captured here.
            let _ = unsafe { SetEvent(event.0) };
        })
    }
}

/// Raises the calling thread to time-critical priority and registers it with MMCSS ("Games"),
/// which keeps the scheduler from being parked behind normal threads. Reverted on drop.
#[derive(Debug)]
pub struct RealtimeGuard {
    mmcss: Option<OwnedMmcss>,
}

#[derive(Debug)]
struct OwnedMmcss(HANDLE);

impl Drop for OwnedMmcss {
    fn drop(&mut self) {
        // SAFETY: the handle came from AvSetMmThreadCharacteristicsW on this same thread.
        let _ = unsafe { AvRevertMmThreadCharacteristics(self.0) };
    }
}

impl RealtimeGuard {
    pub fn acquire() -> Self {
        // SAFETY: changes only the calling thread's priority; MMCSS task index is a valid local.
        unsafe {
            let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
            let mut task_index = 0u32;
            let mmcss = AvSetMmThreadCharacteristicsW(w!("Games"), &mut task_index)
                .ok()
                .map(OwnedMmcss);
            Self { mmcss }
        }
    }

    pub fn mmcss_active(&self) -> bool {
        self.mmcss.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ch_core::engine::clock::NANOS_PER_MS;
    use ch_core::engine::wait::wait_precisely;

    use super::*;

    #[test]
    fn qpc_clock_tracks_wall_time() {
        let clock = QpcClock::new();
        let a = clock.now();
        std::thread::sleep(Duration::from_millis(20));
        let elapsed = clock.now() - a;
        assert!(
            (15 * NANOS_PER_MS..200 * NANOS_PER_MS).contains(&elapsed),
            "{elapsed} ns"
        );
    }

    #[test]
    fn high_resolution_timer_is_available() {
        let waiter = HighResWaiter::new().expect("waiter");
        assert_eq!(
            waiter.kind(),
            TimerKind::HighResolution,
            "Windows 10 1803+ expected"
        );
        assert_eq!(waiter.kind().as_str(), "highResolution");
        assert_eq!(TimerKind::Legacy.as_str(), "legacy");
    }

    #[test]
    fn coarse_sleep_lands_close_to_the_deadline() {
        let clock = QpcClock::new();
        let mut waiter = HighResWaiter::new().expect("waiter");
        let mut worst = 0;
        for _ in 0..20 {
            let deadline = clock.now() + 2 * NANOS_PER_MS;
            assert_eq!(waiter.sleep_until(deadline), Wake::Deadline);
            worst = worst.max(clock.now().saturating_sub(deadline));
        }
        // Coarse only (no spin), sanity bound: the timer wakes near the deadline even while other
        // tests (or the user's apps) load the CPU. Real precision is measured by tools/jitter-bench.
        assert!(worst < 15 * NANOS_PER_MS, "worst overshoot {worst} ns");
    }

    #[test]
    fn precise_wait_hits_the_deadline_within_microseconds() {
        let clock = QpcClock::new();
        let mut waiter = HighResWaiter::new().expect("waiter");
        let _rt = RealtimeGuard::acquire();
        let mut lateness = Vec::new();
        for _ in 0..50 {
            let deadline = clock.now() + 3 * NANOS_PER_MS;
            wait_precisely(&clock, &mut waiter, deadline, NANOS_PER_MS);
            lateness.push(clock.now() - deadline);
        }
        lateness.sort_unstable();
        let p90 = lateness[44];
        assert!(p90 < 200_000, "p90 lateness {p90} ns");
    }

    #[test]
    fn waker_interrupts_a_long_sleep() {
        let clock = QpcClock::new();
        let mut waiter = HighResWaiter::new().expect("waiter");
        let wake = waiter.waker();
        let started = clock.now();
        let t = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            wake();
        });
        assert_eq!(
            waiter.sleep_until(started + 5_000 * NANOS_PER_MS),
            Wake::Signal
        );
        let woke_after = clock.now() - started;
        assert!(
            woke_after < 500 * NANOS_PER_MS,
            "woke after {woke_after} ns"
        );
        t.join().expect("thread");
    }

    #[test]
    fn past_deadlines_do_not_sleep() {
        let clock = QpcClock::new();
        let mut waiter = HighResWaiter::new().expect("waiter");
        assert_eq!(
            waiter.sleep_until(clock.now().saturating_sub(1)),
            Wake::Deadline
        );
    }

    #[test]
    fn realtime_guard_registers_with_mmcss() {
        let guard = RealtimeGuard::acquire();
        assert!(
            guard.mmcss_active(),
            "MMCSS service should be available on desktop Windows"
        );
    }
}
