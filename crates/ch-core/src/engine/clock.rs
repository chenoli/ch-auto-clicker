use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

pub const NANOS_PER_US: u64 = 1_000;
pub const NANOS_PER_MS: u64 = 1_000_000;

pub trait Clock {
    /// Monotonic nanoseconds from an arbitrary origin.
    fn now(&self) -> u64;
}

/// Virtual time for tests. Clones share the same timeline.
#[derive(Debug, Clone, Default)]
pub struct FakeClock {
    now: Arc<AtomicU64>,
    auto_step: u64,
}

impl FakeClock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Every `now()` call advances time by `step` ns, so busy-wait loops terminate in tests.
    pub fn with_auto_step(step: u64) -> Self {
        Self {
            now: Arc::default(),
            auto_step: step,
        }
    }

    pub fn set(&self, t: u64) {
        self.now.fetch_max(t, Ordering::SeqCst);
    }

    pub fn advance(&self, delta: u64) {
        self.now.fetch_add(delta, Ordering::SeqCst);
    }

    pub fn peek(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }
}

impl Clock for FakeClock {
    fn now(&self) -> u64 {
        self.now.fetch_add(self.auto_step, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_clock_is_shared_and_monotonic() {
        let a = FakeClock::new();
        let b = a.clone();
        a.advance(5);
        b.set(3);
        assert_eq!(a.now(), 5, "set never goes backwards");
        b.set(9);
        assert_eq!(a.peek(), 9);
    }

    #[test]
    fn auto_step_advances_on_read() {
        let c = FakeClock::with_auto_step(10);
        assert_eq!(c.now(), 0);
        assert_eq!(c.now(), 10);
    }
}
