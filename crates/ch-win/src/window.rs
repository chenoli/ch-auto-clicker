use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GWL_EXSTYLE, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowThreadProcessId, IsWindow, SetForegroundWindow, SetWindowLongPtrW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

pub fn set_no_activate(hwnd: isize) {
    let hwnd = HWND(hwnd as *mut core::ffi::c_void);
    // SAFETY: reads and writes the extended style of a window this process created.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize,
        );
    }
}

pub fn ctrl_down() -> bool {
    // SAFETY: GetAsyncKeyState only reads input state.
    unsafe { GetAsyncKeyState(i32::from(VK_CONTROL.0)) < 0 }
}

pub fn is_foreground(hwnd: isize) -> bool {
    // SAFETY: GetForegroundWindow only reads which window is active.
    let foreground = unsafe { GetForegroundWindow() };
    hwnd != 0 && foreground.0 as isize == hwnd
}

pub fn bring_to_front(hwnd: isize) -> bool {
    let hwnd = HWND(hwnd as *mut core::ffi::c_void);
    // SAFETY: plain user32 calls on a window handle checked with IsWindow; the input
    // attachment is undone before returning, and no input is synthesized.
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return false;
        }
        let foreground = GetForegroundWindow();
        if foreground == hwnd || SetForegroundWindow(hwnd).as_bool() {
            return true;
        }
        let current = GetCurrentThreadId();
        let owner = GetWindowThreadProcessId(foreground, None);
        let attached =
            owner != 0 && owner != current && AttachThreadInput(current, owner, true).as_bool();
        let _ = BringWindowToTop(hwnd);
        let raised = SetForegroundWindow(hwnd).as_bool();
        if attached {
            let _ = AttachThreadInput(current, owner, false);
        }
        raised
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bring_to_front_rejects_a_handle_that_is_not_a_window() {
        assert!(!bring_to_front(0));
    }

    #[test]
    fn a_null_handle_is_never_the_foreground_window() {
        assert!(!is_foreground(0));
    }
}
