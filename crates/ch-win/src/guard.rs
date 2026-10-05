//! "Never click our own window" (ARCHITECTURE §6.6). The HUD is WS_EX_TRANSPARENT, so hit-testing
//! goes through it and it never counts as ours here.

use ch_core::geometry::Point;
use ch_core::input::TargetGuard;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetWindowThreadProcessId, WindowFromPoint,
};

use crate::input::{InputSink, SystemSink};

#[derive(Debug, Clone, Copy)]
pub struct OwnWindowGuard {
    pid: u32,
}

impl OwnWindowGuard {
    pub fn new() -> Self {
        // SAFETY: no preconditions.
        let pid = unsafe { GetCurrentProcessId() };
        Self { pid }
    }

    /// Whether the top-level window under `p` belongs to this process.
    pub fn is_own_window_at(&self, p: Point) -> bool {
        // SAFETY: plain value arguments; handles are only compared, never dereferenced.
        unsafe {
            let hwnd = WindowFromPoint(POINT { x: p.x, y: p.y });
            if hwnd.is_invalid() {
                return false;
            }
            let root = GetAncestor(hwnd, GA_ROOT);
            let mut pid = 0u32;
            GetWindowThreadProcessId(if root.is_invalid() { hwnd } else { root }, Some(&mut pid));
            pid == self.pid
        }
    }
}

impl Default for OwnWindowGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetGuard for OwnWindowGuard {
    fn blocks_click(&self) -> bool {
        SystemSink
            .cursor_pos()
            .is_some_and(|p| self.is_own_window_at(p))
    }
}

#[cfg(test)]
mod tests {
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_POPUP, WS_VISIBLE,
    };
    use windows::core::w;

    use super::*;
    use crate::display::{display_layout, ensure_per_monitor_v2};

    #[test]
    fn recognizes_our_window_and_nothing_else() {
        ensure_per_monitor_v2();
        let primary = display_layout().primary().expect("primary").rect;
        let (x, y) = (
            primary.left + primary.width / 2 - 60,
            primary.top + primary.height / 2 - 60,
        );
        // A built-in BUTTON as a top-level popup: no class registration, no focus steal. (STATIC would
        // not work: it answers HTTRANSPARENT, so WindowFromPoint looks through it.)
        // SAFETY: valid class name and styles; the window is destroyed before the test returns.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("BUTTON"),
                w!("guard test"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                120,
                120,
                None,
                None,
                None,
                None,
            )
        }
        .expect("window");

        let guard = OwnWindowGuard::default();
        let inside = guard.is_own_window_at(Point::new(x + 60, y + 60));
        let outside = guard.is_own_window_at(Point::new(x - 200, y));
        // SAFETY: created above on this thread.
        let _ = unsafe { DestroyWindow(hwnd) };

        assert!(inside, "our popup is under its own center");
        assert!(!outside, "no other window in the test process");
        let _ = guard.blocks_click();
    }
}
