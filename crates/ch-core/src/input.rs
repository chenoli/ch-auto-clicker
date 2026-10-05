//! Input injection contract (ARCHITECTURE §10.2). Implemented by `ch-win` (SendInput) and by a
//! recording fake for tests.

use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    X1,
    X2,
}

impl MouseButton {
    pub const ALL: [MouseButton; 5] = [Self::Left, Self::Middle, Self::Right, Self::X1, Self::X2];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClickKind {
    Single,
    Double,
}

impl ClickKind {
    /// Clicks delivered per tick; the UI counts and shows CPS with this factor.
    pub const fn clicks(self) -> u32 {
        match self {
            Self::Single => 1,
            Self::Double => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    /// The OS accepted fewer events than sent (UIPI, secure desktop, another app blocking input).
    #[error("input blocked: {sent} of {expected} events were accepted")]
    Blocked { sent: u32, expected: u32 },
    #[error("target ({x}, {y}) is outside the virtual desktop")]
    OutsideDesktop { x: i32, y: i32 },
}

pub trait InputBackend {
    /// Moves the cursor to an absolute physical pixel on the virtual desktop.
    fn move_to(&mut self, target: Point) -> Result<(), InputError>;

    /// Clicks at the current cursor position. Down and up go out together, so a stop between
    /// them can never leave a button held.
    fn click(&mut self, button: MouseButton, kind: ClickKind) -> Result<(), InputError>;

    fn button_down(&mut self, button: MouseButton) -> Result<(), InputError>;

    fn button_up(&mut self, button: MouseButton) -> Result<(), InputError>;
}

/// Records calls instead of injecting input. For engine tests in any crate.
#[cfg(any(test, feature = "test-util"))]
pub mod fake {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Call {
        MoveTo(Point),
        Click(MouseButton, ClickKind),
        Down(MouseButton),
        Up(MouseButton),
    }

    #[derive(Debug, Default)]
    pub struct RecordingBackend {
        pub calls: Vec<Call>,
        /// When set, every call fails with this error (and is still recorded).
        pub fail_with: Option<InputError>,
    }

    impl RecordingBackend {
        fn record(&mut self, call: Call) -> Result<(), InputError> {
            self.calls.push(call);
            self.fail_with.clone().map_or(Ok(()), Err)
        }
    }

    impl InputBackend for RecordingBackend {
        fn move_to(&mut self, target: Point) -> Result<(), InputError> {
            self.record(Call::MoveTo(target))
        }

        fn click(&mut self, button: MouseButton, kind: ClickKind) -> Result<(), InputError> {
            self.record(Call::Click(button, kind))
        }

        fn button_down(&mut self, button: MouseButton) -> Result<(), InputError> {
            self.record(Call::Down(button))
        }

        fn button_up(&mut self, button: MouseButton) -> Result<(), InputError> {
            self.record(Call::Up(button))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{Call, RecordingBackend};
    use super::*;

    #[test]
    fn double_click_counts_twice() {
        assert_eq!(ClickKind::Single.clicks(), 1);
        assert_eq!(ClickKind::Double.clicks(), 2);
    }

    #[test]
    fn recording_backend_records_in_order() {
        let mut backend = RecordingBackend::default();
        backend.move_to(Point::new(10, -5)).expect("ok");
        backend
            .click(MouseButton::X2, ClickKind::Double)
            .expect("ok");
        backend.button_down(MouseButton::Left).expect("ok");
        backend.button_up(MouseButton::Left).expect("ok");
        assert_eq!(
            backend.calls,
            vec![
                Call::MoveTo(Point::new(10, -5)),
                Call::Click(MouseButton::X2, ClickKind::Double),
                Call::Down(MouseButton::Left),
                Call::Up(MouseButton::Left),
            ]
        );
    }

    #[test]
    fn recording_backend_can_simulate_failures() {
        let blocked = InputError::Blocked {
            sent: 0,
            expected: 2,
        };
        let mut backend = RecordingBackend {
            fail_with: Some(blocked.clone()),
            ..Default::default()
        };
        assert_eq!(
            backend.click(MouseButton::Left, ClickKind::Single),
            Err(blocked)
        );
        assert_eq!(backend.calls.len(), 1);
        assert_eq!(
            blocked_message(),
            "input blocked: 0 of 2 events were accepted"
        );
    }

    fn blocked_message() -> String {
        InputError::Blocked {
            sent: 0,
            expected: 2,
        }
        .to_string()
    }

    #[test]
    fn exposes_every_button() {
        assert_eq!(MouseButton::ALL.len(), 5);
    }
}
