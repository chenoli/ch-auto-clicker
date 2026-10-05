//! Scheduler core (ARCHITECTURE §5.2), generic over the clock and the OS waiter so it runs on
//! virtual time in tests.

pub mod clock;
pub mod run;
pub mod scheduler;
pub mod state;
pub mod timeline;
pub mod wait;
