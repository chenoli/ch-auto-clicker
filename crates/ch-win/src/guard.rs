use std::sync::Mutex;

use ch_core::geometry::Point;
use ch_core::input::TargetGuard;
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOT, GetAncestor, GetWindowThreadProcessId, WindowFromPoint,
};

use crate::input::{InputSink, SystemSink};
use crate::integrity::{current_integrity, process_integrity};

#[derive(Debug)]
pub struct OwnWindowGuard {
    pid: u32,
    integrity: Option<u32>,
    elevated: Mutex<Option<(u32, bool)>>,
}

fn window_pid_at(p: Point) -> Option<u32> {
    // SAFETY: plain value arguments; handles are only compared, never dereferenced.
    unsafe {
        let hwnd = WindowFromPoint(POINT { x: p.x, y: p.y });
        if hwnd.is_invalid() {
            return None;
        }
        let root = GetAncestor(hwnd, GA_ROOT);
        let mut pid = 0u32;
        GetWindowThreadProcessId(if root.is_invalid() { hwnd } else { root }, Some(&mut pid));
        (pid != 0).then_some(pid)
    }
}

impl OwnWindowGuard {
    pub fn new() -> Self {
        // SAFETY: no preconditions.
        let pid = unsafe { GetCurrentProcessId() };
        Self {
            pid,
            integrity: current_integrity(),
            elevated: Mutex::new(None),
        }
    }

    pub fn is_own_window_at(&self, p: Point) -> bool {
        window_pid_at(p) == Some(self.pid)
    }

    pub fn is_elevated_window_at(&self, p: Point) -> bool {
        let Some(own) = self.integrity else {
            return false;
        };
        let Some(pid) = window_pid_at(p).filter(|pid| *pid != self.pid) else {
            return false;
        };
        let mut cache = self.elevated.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((cached, elevated)) = *cache
            && cached == pid
        {
            return elevated;
        }
        let elevated = process_integrity(pid).is_some_and(|level| level > own);
        *cache = Some((pid, elevated));
        elevated
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

    fn target_is_elevated(&self) -> bool {
        SystemSink
            .cursor_pos()
            .is_some_and(|p| self.is_elevated_window_at(p))
    }

    fn session_locked(&self) -> bool {
        crate::session::is_locked()
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
        assert!(
            !guard.is_elevated_window_at(Point::new(x + 60, y + 60)),
            "our own window is never elevated"
        );
        let _ = guard.blocks_click();
        let _ = guard.target_is_elevated();
        assert!(!guard.session_locked(), "tests run on an unlocked desktop");
    }
}
