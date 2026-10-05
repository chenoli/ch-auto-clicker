use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, SetWindowLongPtrW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
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
