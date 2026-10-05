use std::sync::Arc;

use super::clock::Clock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wake {
    Deadline,
    Signal,
}

pub type Waker = Arc<dyn Fn() + Send + Sync>;

pub trait CoarseWaiter {
    fn sleep_until(&mut self, deadline: u64) -> Wake;

    fn waker(&self) -> Waker;
}

pub fn wait_precisely<C: Clock + ?Sized, W: CoarseWaiter + ?Sized>(
    clock: &C,
    waiter: &mut W,
    deadline: u64,
    spin_window: u64,
) -> Wake {
    loop {
        let now = clock.now();
        if now >= deadline {
            return Wake::Deadline;
        }
        if deadline - now > spin_window {
            if waiter.sleep_until(deadline - spin_window) == Wake::Signal {
                return Wake::Signal;
            }
            continue;
        }
        while clock.now() < deadline {
            std::hint::spin_loop();
        }
        return Wake::Deadline;
    }
}

#[cfg(any(test, feature = "test-util"))]
pub mod fake {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    use super::super::clock::FakeClock;
    use super::{CoarseWaiter, Wake, Waker};

    #[derive(Debug)]
    pub struct FakeWaiter {
        clock: FakeClock,
        signalled: Arc<AtomicBool>,
        signal_at: Arc<AtomicU64>,
        pub sleeps: u64,
    }

    impl FakeWaiter {
        pub fn new(clock: FakeClock) -> Self {
            Self {
                clock,
                signalled: Arc::default(),
                signal_at: Arc::new(AtomicU64::new(u64::MAX)),
                sleeps: 0,
            }
        }

        pub fn signal_at(&self, t: u64) {
            self.signal_at.store(t, Ordering::SeqCst);
        }
    }

    impl CoarseWaiter for FakeWaiter {
        fn sleep_until(&mut self, deadline: u64) -> Wake {
            self.sleeps += 1;
            if self.signalled.swap(false, Ordering::SeqCst) {
                return Wake::Signal;
            }
            let at = self.signal_at.load(Ordering::SeqCst);
            if at <= deadline {
                self.signal_at.store(u64::MAX, Ordering::SeqCst);
                self.clock.set(at);
                return Wake::Signal;
            }
            self.clock.set(deadline);
            Wake::Deadline
        }

        fn waker(&self) -> Waker {
            let flag = Arc::clone(&self.signalled);
            Arc::new(move || flag.store(true, Ordering::SeqCst))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::FakeWaiter;
    use super::*;
    use crate::engine::clock::FakeClock;

    #[test]
    fn sleeps_then_spins_to_the_exact_deadline() {
        let clock = FakeClock::with_auto_step(1_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        assert_eq!(
            wait_precisely(&clock, &mut waiter, 10_000_000, 1_000_000),
            Wake::Deadline
        );
        assert_eq!(waiter.sleeps, 1);
        let t = clock.peek();
        assert!((10_000_000..10_002_000).contains(&t), "ended at {t}");
    }

    #[test]
    fn short_waits_only_spin() {
        let clock = FakeClock::with_auto_step(1_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        assert_eq!(
            wait_precisely(&clock, &mut waiter, 500_000, 1_000_000),
            Wake::Deadline
        );
        assert_eq!(waiter.sleeps, 0);
    }

    #[test]
    fn past_deadlines_return_immediately() {
        let clock = FakeClock::with_auto_step(1);
        clock.set(5_000);
        let mut waiter = FakeWaiter::new(clock.clone());
        assert_eq!(
            wait_precisely(&clock, &mut waiter, 1_000, 1_000_000),
            Wake::Deadline
        );
        assert_eq!(waiter.sleeps, 0);
    }

    #[test]
    fn signals_interrupt_the_sleep() {
        let clock = FakeClock::with_auto_step(1);
        let mut waiter = FakeWaiter::new(clock.clone());
        (waiter.waker())();
        assert_eq!(
            wait_precisely(&clock, &mut waiter, 50_000_000, 1_000_000),
            Wake::Signal
        );

        waiter.signal_at(20_000_000);
        assert_eq!(
            wait_precisely(&clock, &mut waiter, 50_000_000, 1_000_000),
            Wake::Signal
        );
        assert!(clock.peek() >= 20_000_000 && clock.peek() < 21_000_000);
    }
}
