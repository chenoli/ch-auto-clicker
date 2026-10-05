use crate::geometry::Point;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "specta", derive(specta::Type))]
#[serde(rename_all = "lowercase")]
pub enum ClickKind {
    Single,
    Double,
}

impl ClickKind {
    pub const fn clicks(self) -> u32 {
        match self {
            Self::Single => 1,
            Self::Double => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InputError {
    #[error("input blocked: {sent} of {expected} events were accepted")]
    Blocked { sent: u32, expected: u32 },
    #[error("target ({x}, {y}) is outside the virtual desktop")]
    OutsideDesktop { x: i32, y: i32 },
}

pub trait InputBackend {
    fn move_to(&mut self, target: Point) -> Result<(), InputError>;

    fn click(&mut self, button: MouseButton, kind: ClickKind) -> Result<(), InputError>;

    fn button_down(&mut self, button: MouseButton) -> Result<(), InputError>;

    fn button_up(&mut self, button: MouseButton) -> Result<(), InputError>;

    fn cursor_pos(&mut self) -> Option<Point> {
        None
    }
}

pub trait TargetGuard: Send {
    fn blocks_click(&self) -> bool;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAll;

impl TargetGuard for AllowAll {
    fn blocks_click(&self) -> bool {
        false
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NoopBackend;

impl InputBackend for NoopBackend {
    fn move_to(&mut self, _target: Point) -> Result<(), InputError> {
        Ok(())
    }

    fn click(&mut self, _button: MouseButton, _kind: ClickKind) -> Result<(), InputError> {
        Ok(())
    }

    fn button_down(&mut self, _button: MouseButton) -> Result<(), InputError> {
        Ok(())
    }

    fn button_up(&mut self, _button: MouseButton) -> Result<(), InputError> {
        Ok(())
    }
}

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
        pub fail_with: Option<InputError>,
        pub cursor: Option<Point>,
    }

    impl RecordingBackend {
        fn record(&mut self, call: Call) -> Result<(), InputError> {
            self.calls.push(call);
            self.fail_with.clone().map_or(Ok(()), Err)
        }
    }

    impl InputBackend for RecordingBackend {
        fn move_to(&mut self, target: Point) -> Result<(), InputError> {
            if self.cursor.is_some() {
                self.cursor = Some(target);
            }
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

        fn cursor_pos(&mut self) -> Option<Point> {
            self.cursor
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
    fn noop_backend_accepts_everything() {
        let mut b = NoopBackend;
        assert!(b.move_to(Point::new(0, 0)).is_ok());
        assert!(b.click(MouseButton::Left, ClickKind::Double).is_ok());
        assert!(b.button_down(MouseButton::X1).is_ok());
        assert!(b.button_up(MouseButton::X1).is_ok());
    }

    #[test]
    fn allow_all_never_blocks() {
        assert!(!AllowAll.blocks_click());
    }

    #[test]
    fn exposes_every_button() {
        assert_eq!(MouseButton::ALL.len(), 5);
    }
}
