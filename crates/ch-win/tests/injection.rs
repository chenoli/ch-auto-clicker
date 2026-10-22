#![cfg(windows)]

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use ch_core::geometry::{Point, Rect};
use ch_core::input::{ClickKind, InputBackend, MouseButton};
use ch_win::display::{display_layout, ensure_per_monitor_v2};
use ch_win::input::{InputSink, SendInputBackend, SystemSink};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CS_DBLCLKS, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG, PM_REMOVE,
    PeekMessageW, RegisterClassW, TranslateMessage, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_MBUTTONDOWN, WM_RBUTTONDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP, WNDCLASSW, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE, XBUTTON1, XBUTTON2,
};
use windows::core::w;

static LEFT_DOWN: AtomicU32 = AtomicU32::new(0);
static LEFT_UP: AtomicU32 = AtomicU32::new(0);
static LEFT_DBL: AtomicU32 = AtomicU32::new(0);
static MIDDLE_DOWN: AtomicU32 = AtomicU32::new(0);
static RIGHT_DOWN: AtomicU32 = AtomicU32::new(0);
static X1_DOWN: AtomicU32 = AtomicU32::new(0);
static X2_DOWN: AtomicU32 = AtomicU32::new(0);

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let bump = |c: &AtomicU32| {
        c.fetch_add(1, Ordering::SeqCst);
    };
    match msg {
        WM_LBUTTONDOWN => bump(&LEFT_DOWN),
        WM_LBUTTONUP => bump(&LEFT_UP),
        WM_LBUTTONDBLCLK => bump(&LEFT_DBL),
        WM_MBUTTONDOWN => bump(&MIDDLE_DOWN),
        WM_RBUTTONDOWN => bump(&RIGHT_DOWN),
        WM_XBUTTONDOWN => {
            let which = ((wparam.0 >> 16) & 0xFFFF) as u16;
            if which == XBUTTON1 {
                bump(&X1_DOWN);
            } else if which == XBUTTON2 {
                bump(&X2_DOWN);
            }
            return LRESULT(1);
        }
        WM_XBUTTONUP => return LRESULT(1),
        _ => {}
    }
    // SAFETY: forwarding the exact arguments the system gave us.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

struct TestWindow {
    hwnd: HWND,
    rect: Rect,
}

impl TestWindow {
    fn open(rect: Rect) -> Self {
        // SAFETY: standard window class registration and creation with a static class name and
        // a wndproc that lives for the whole process.
        unsafe {
            let instance = GetModuleHandleW(None).expect("module handle");
            let class = WNDCLASSW {
                style: CS_DBLCLKS,
                lpfnWndProc: Some(wnd_proc),
                hInstance: instance.into(),
                lpszClassName: w!("ChAutoClickerInjectionTest"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                w!("ChAutoClickerInjectionTest"),
                w!("CH Auto Clicker injection test"),
                WS_POPUP | WS_VISIBLE,
                rect.left,
                rect.top,
                rect.width,
                rect.height,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .expect("test window");
            let window = Self { hwnd, rect };
            window.pump(Duration::from_millis(150));
            window
        }
    }

    fn center(&self) -> Point {
        Point::new(
            self.rect.left + self.rect.width / 2,
            self.rect.top + self.rect.height / 2,
        )
    }

    fn pump(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        let mut msg = MSG::default();
        while Instant::now() < deadline {
            // SAFETY: standard message pump on the thread that owns the window.
            unsafe {
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for TestWindow {
    fn drop(&mut self) {
        // SAFETY: the window was created by this thread and is destroyed exactly once.
        let _ = unsafe { DestroyWindow(self.hwnd) };
    }
}

struct CursorGuard(Option<Point>);

impl CursorGuard {
    fn save() -> Self {
        Self(SystemSink.cursor_pos())
    }
}

impl Drop for CursorGuard {
    fn drop(&mut self) {
        if let Some(p) = self.0 {
            SystemSink.set_cursor_pos(p);
        }
    }
}

#[test]
#[ignore = "injects real clicks; run on request with games closed"]
fn clicks_reach_a_window_with_every_button() {
    ensure_per_monitor_v2();
    let _cursor = CursorGuard::save();
    let primary = display_layout().primary().expect("primary monitor").rect;
    let window = TestWindow::open(Rect::new(
        primary.left + primary.width / 2 - 120,
        primary.top + primary.height / 2 - 120,
        240,
        240,
    ));

    let mut backend = SendInputBackend::system();
    backend.move_to(window.center()).expect("move");
    window.pump(Duration::from_millis(100));

    backend
        .click(MouseButton::Left, ClickKind::Single)
        .expect("left");
    window.pump(Duration::from_millis(600));
    backend
        .click(MouseButton::Left, ClickKind::Double)
        .expect("double");
    window.pump(Duration::from_millis(150));
    for button in [
        MouseButton::Middle,
        MouseButton::Right,
        MouseButton::X1,
        MouseButton::X2,
    ] {
        backend.click(button, ClickKind::Single).expect("click");
        window.pump(Duration::from_millis(80));
    }
    window.pump(Duration::from_millis(200));

    let load = |c: &AtomicU32| c.load(Ordering::SeqCst);
    assert_eq!(load(&LEFT_DOWN), 2, "left down");
    assert_eq!(load(&LEFT_DBL), 1, "double-click");
    assert_eq!(load(&LEFT_UP), 3, "left up");
    assert_eq!(load(&MIDDLE_DOWN), 1, "middle");
    assert_eq!(load(&RIGHT_DOWN), 1, "right");
    assert_eq!(load(&X1_DOWN), 1, "x1");
    assert_eq!(load(&X2_DOWN), 1, "x2");
}

#[test]
#[ignore = "moves the real cursor; run on request"]
fn absolute_moves_land_on_the_exact_pixel_on_every_monitor() {
    ensure_per_monitor_v2();
    let _cursor = CursorGuard::save();
    let layout = display_layout();
    let mut backend = SendInputBackend::system();
    let mut checked = 0;

    let filter = std::env::var("CH_TEST_MONITORS").ok();
    let selected = layout.monitors.iter().filter(|m| match filter.as_deref() {
        None | Some("") => true,
        Some("primary") => m.primary,
        Some(list) => list.split(',').any(|id| id.trim() == m.id),
    });

    for monitor in selected {
        let r = monitor.rect;
        for fx in [0, 1, 2, 3, 4] {
            for fy in [0, 1, 2, 3, 4] {
                let target = Point::new(
                    r.left + (r.width - 1) * fx / 4,
                    r.top + (r.height - 1) * fy / 4,
                );
                backend.move_to(target).expect("move");
                std::thread::sleep(Duration::from_millis(5));
                let actual = SystemSink.cursor_pos().expect("cursor");
                assert_eq!(actual, target, "monitor {} at {target:?}", monitor.id);
                checked += 1;
            }
        }
    }

    assert!(
        checked > 0,
        "no monitor matched CH_TEST_MONITORS={filter:?}"
    );
    assert_eq!(
        backend.corrections(),
        0,
        "normalization should be exact without SetCursorPos fix-ups ({checked} points)"
    );
}
