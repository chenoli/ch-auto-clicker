//! `SendInput` backend (ARCHITECTURE §10.2): user-mode injection, events flagged as injected.

use ch_core::geometry::{self, Point, Rect};
use ch_core::input::{ClickKind, InputBackend, InputError, MouseButton};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSE_EVENT_FLAGS, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_MOVE_NOCOALESCE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEEVENTF_VIRTUALDESK, MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, SetCursorPos, XBUTTON1, XBUTTON2,
};

/// Larger cursor offsets after a move mean the user moved the mouse, not a rounding error, so
/// the backend leaves the cursor alone instead of fighting them.
const MAX_CORRECTION_PX: i32 = 2;

/// The OS surface the backend talks to; faked in tests so nothing is really injected.
pub trait InputSink {
    /// Returns how many events the OS accepted.
    fn send(&self, inputs: &[INPUT]) -> u32;
    fn cursor_pos(&self) -> Option<Point>;
    fn set_cursor_pos(&self, p: Point) -> bool;
    fn virtual_desktop(&self) -> Rect;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemSink;

impl InputSink for SystemSink {
    fn send(&self, inputs: &[INPUT]) -> u32 {
        // SAFETY: `inputs` is a valid slice of initialized INPUT structs and cbsize matches.
        unsafe { SendInput(inputs, size_of::<INPUT>() as i32) }
    }

    fn cursor_pos(&self) -> Option<Point> {
        let mut p = POINT::default();
        // SAFETY: `p` is a valid, writable POINT for the duration of the call.
        unsafe { GetCursorPos(&mut p) }.ok()?;
        Some(Point::new(p.x, p.y))
    }

    fn set_cursor_pos(&self, p: Point) -> bool {
        // SAFETY: plain value arguments; the call has no memory preconditions.
        unsafe { SetCursorPos(p.x, p.y) }.is_ok()
    }

    fn virtual_desktop(&self) -> Rect {
        // SAFETY: GetSystemMetrics only reads system state.
        unsafe {
            Rect::new(
                GetSystemMetrics(SM_XVIRTUALSCREEN),
                GetSystemMetrics(SM_YVIRTUALSCREEN),
                GetSystemMetrics(SM_CXVIRTUALSCREEN),
                GetSystemMetrics(SM_CYVIRTUALSCREEN),
            )
        }
    }
}

#[derive(Debug)]
pub struct SendInputBackend<S: InputSink = SystemSink> {
    sink: S,
    corrections: u64,
}

impl SendInputBackend<SystemSink> {
    pub fn system() -> Self {
        Self::new(SystemSink)
    }
}

impl<S: InputSink> SendInputBackend<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            corrections: 0,
        }
    }

    /// Moves that needed a ±1–2 px fix-up after the absolute move (diagnostics).
    pub fn corrections(&self) -> u64 {
        self.corrections
    }

    pub fn sink(&self) -> &S {
        &self.sink
    }

    fn send_all(&self, inputs: &[INPUT]) -> Result<(), InputError> {
        let expected = inputs.len() as u32;
        let sent = self.sink.send(inputs);
        if sent == expected {
            Ok(())
        } else {
            Err(InputError::Blocked { sent, expected })
        }
    }
}

impl<S: InputSink> InputBackend for SendInputBackend<S> {
    fn move_to(&mut self, target: Point) -> Result<(), InputError> {
        let desktop = self.sink.virtual_desktop();
        let n = geometry::normalize(target, desktop).map_err(|_| InputError::OutsideDesktop {
            x: target.x,
            y: target.y,
        })?;
        self.send_all(&[mouse_input(
            n.x,
            n.y,
            0,
            MOUSEEVENTF_MOVE
                | MOUSEEVENTF_ABSOLUTE
                | MOUSEEVENTF_VIRTUALDESK
                | MOUSEEVENTF_MOVE_NOCOALESCE,
        )])?;

        if let Some(actual) = self.sink.cursor_pos() {
            let (dx, dy) = ((actual.x - target.x).abs(), (actual.y - target.y).abs());
            if (dx, dy) != (0, 0)
                && dx <= MAX_CORRECTION_PX
                && dy <= MAX_CORRECTION_PX
                && self.sink.set_cursor_pos(target)
            {
                self.corrections += 1;
            }
        }
        Ok(())
    }

    fn click(&mut self, button: MouseButton, kind: ClickKind) -> Result<(), InputError> {
        self.send_all(&click_inputs(button, kind))
    }

    fn button_down(&mut self, button: MouseButton) -> Result<(), InputError> {
        let (down, _, data) = button_flags(button);
        self.send_all(&[mouse_input(0, 0, data, down)])
    }

    fn button_up(&mut self, button: MouseButton) -> Result<(), InputError> {
        let (_, up, data) = button_flags(button);
        self.send_all(&[mouse_input(0, 0, data, up)])
    }
}

/// Down/up pairs for one tick, in a single batch so `SendInput` delivers them atomically.
pub fn click_inputs(button: MouseButton, kind: ClickKind) -> Vec<INPUT> {
    let (down, up, data) = button_flags(button);
    (0..kind.clicks())
        .flat_map(|_| [mouse_input(0, 0, data, down), mouse_input(0, 0, data, up)])
        .collect()
}

/// (down flag, up flag, mouseData) for a button. X buttons share flags and differ in mouseData.
fn button_flags(button: MouseButton) -> (MOUSE_EVENT_FLAGS, MOUSE_EVENT_FLAGS, u32) {
    match button {
        MouseButton::Left => (MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, 0),
        MouseButton::Middle => (MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, 0),
        MouseButton::Right => (MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, 0),
        MouseButton::X1 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, u32::from(XBUTTON1)),
        MouseButton::X2 => (MOUSEEVENTF_XDOWN, MOUSEEVENTF_XUP, u32::from(XBUTTON2)),
    }
}

fn mouse_input(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Default)]
    struct FakeSink {
        sent: RefCell<Vec<MOUSEINPUT>>,
        accept: Option<u32>,
        cursor: Cell<Option<Point>>,
        set_calls: RefCell<Vec<Point>>,
    }

    impl InputSink for FakeSink {
        fn send(&self, inputs: &[INPUT]) -> u32 {
            // SAFETY: every INPUT built by this module is a mouse input.
            self.sent
                .borrow_mut()
                .extend(inputs.iter().map(|i| unsafe { i.Anonymous.mi }));
            self.accept.unwrap_or(inputs.len() as u32)
        }

        fn cursor_pos(&self) -> Option<Point> {
            self.cursor.get()
        }

        fn set_cursor_pos(&self, p: Point) -> bool {
            self.set_calls.borrow_mut().push(p);
            true
        }

        fn virtual_desktop(&self) -> Rect {
            Rect::from_edges(-1680, 0, 1920, 1080)
        }
    }

    fn flags(sink: &FakeSink) -> Vec<(MOUSE_EVENT_FLAGS, u32)> {
        sink.sent
            .borrow()
            .iter()
            .map(|mi| (mi.dwFlags, mi.mouseData))
            .collect()
    }

    #[test]
    fn single_click_is_one_down_up_pair() {
        let mut backend = SendInputBackend::new(FakeSink::default());
        backend
            .click(MouseButton::Left, ClickKind::Single)
            .expect("accepted");
        assert_eq!(
            flags(backend.sink()),
            vec![(MOUSEEVENTF_LEFTDOWN, 0), (MOUSEEVENTF_LEFTUP, 0)]
        );
    }

    #[test]
    fn double_click_is_two_pairs_in_one_batch() {
        let inputs = click_inputs(MouseButton::Right, ClickKind::Double);
        assert_eq!(inputs.len(), 4);
        let mut backend = SendInputBackend::new(FakeSink::default());
        backend
            .click(MouseButton::Right, ClickKind::Double)
            .expect("accepted");
        assert_eq!(
            flags(backend.sink()),
            vec![
                (MOUSEEVENTF_RIGHTDOWN, 0),
                (MOUSEEVENTF_RIGHTUP, 0),
                (MOUSEEVENTF_RIGHTDOWN, 0),
                (MOUSEEVENTF_RIGHTUP, 0),
            ]
        );
    }

    #[test]
    fn side_buttons_use_x_flags_with_button_data() {
        let mut backend = SendInputBackend::new(FakeSink::default());
        backend
            .click(MouseButton::X1, ClickKind::Single)
            .expect("accepted");
        backend
            .click(MouseButton::X2, ClickKind::Single)
            .expect("accepted");
        backend
            .click(MouseButton::Middle, ClickKind::Single)
            .expect("accepted");
        assert_eq!(
            flags(backend.sink()),
            vec![
                (MOUSEEVENTF_XDOWN, u32::from(XBUTTON1)),
                (MOUSEEVENTF_XUP, u32::from(XBUTTON1)),
                (MOUSEEVENTF_XDOWN, u32::from(XBUTTON2)),
                (MOUSEEVENTF_XUP, u32::from(XBUTTON2)),
                (MOUSEEVENTF_MIDDLEDOWN, 0),
                (MOUSEEVENTF_MIDDLEUP, 0),
            ]
        );
    }

    #[test]
    fn separate_down_and_up_for_holds() {
        let mut backend = SendInputBackend::new(FakeSink::default());
        backend.button_down(MouseButton::Left).expect("accepted");
        backend.button_up(MouseButton::Left).expect("accepted");
        assert_eq!(
            flags(backend.sink()),
            vec![(MOUSEEVENTF_LEFTDOWN, 0), (MOUSEEVENTF_LEFTUP, 0)]
        );
    }

    #[test]
    fn partial_acceptance_is_reported_as_blocked() {
        let mut backend = SendInputBackend::new(FakeSink {
            accept: Some(1),
            ..Default::default()
        });
        assert_eq!(
            backend.click(MouseButton::Left, ClickKind::Double),
            Err(InputError::Blocked {
                sent: 1,
                expected: 4
            })
        );
    }

    #[test]
    fn move_sends_normalized_absolute_coordinates_on_the_virtual_desktop() {
        let sink = FakeSink::default();
        sink.cursor.set(Some(Point::new(-840, 610)));
        let mut backend = SendInputBackend::new(sink);
        backend.move_to(Point::new(-840, 610)).expect("accepted");

        let sent = backend.sink().sent.borrow();
        assert_eq!(sent.len(), 1);
        let mi = sent[0];
        assert_eq!(
            mi.dwFlags,
            MOUSEEVENTF_MOVE
                | MOUSEEVENTF_ABSOLUTE
                | MOUSEEVENTF_VIRTUALDESK
                | MOUSEEVENTF_MOVE_NOCOALESCE
        );
        let desk = backend.sink().virtual_desktop();
        let n = geometry::normalize(Point::new(-840, 610), desk).expect("inside");
        assert_eq!((mi.dx, mi.dy), (n.x, n.y));
        assert_eq!(backend.corrections(), 0);
        assert!(backend.sink().set_calls.borrow().is_empty());
    }

    #[test]
    fn move_fixes_small_rounding_offsets() {
        let sink = FakeSink::default();
        sink.cursor.set(Some(Point::new(101, 199)));
        let mut backend = SendInputBackend::new(sink);
        backend.move_to(Point::new(100, 200)).expect("accepted");
        assert_eq!(
            *backend.sink().set_calls.borrow(),
            vec![Point::new(100, 200)]
        );
        assert_eq!(backend.corrections(), 1);
    }

    #[test]
    fn move_does_not_fight_the_user() {
        let sink = FakeSink::default();
        sink.cursor.set(Some(Point::new(140, 200)));
        let mut backend = SendInputBackend::new(sink);
        backend.move_to(Point::new(100, 200)).expect("accepted");
        assert!(backend.sink().set_calls.borrow().is_empty());
        assert_eq!(backend.corrections(), 0);
    }

    #[test]
    fn move_outside_the_desktop_is_rejected_without_sending() {
        let mut backend = SendInputBackend::new(FakeSink::default());
        assert_eq!(
            backend.move_to(Point::new(5000, 10)),
            Err(InputError::OutsideDesktop { x: 5000, y: 10 })
        );
        assert!(backend.sink().sent.borrow().is_empty());
    }

    #[test]
    fn system_sink_reads_the_real_virtual_desktop() {
        // Read-only queries: safe to run anywhere, injects nothing.
        let sink = SystemSink;
        let desk = sink.virtual_desktop();
        assert!(!desk.is_empty());
        if let Some(p) = sink.cursor_pos() {
            assert!(desk.contains(p));
        }
    }
}
