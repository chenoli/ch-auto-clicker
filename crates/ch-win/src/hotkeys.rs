use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use ch_core::hotkeys::{RecordStep, Recorder, is_mouse_key};
use ch_core::model::{Hotkey, Modifier};
use windows::Win32::Foundation::{ERROR_HOTKEY_ALREADY_REGISTERED, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    HHOOK, HWND_MESSAGE, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, MSLLHOOKSTRUCT, PostMessageW,
    PostQuitMessage, RegisterClassW, SetTimer, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_KEYBOARD_LL, WH_MOUSE_LL, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_HOTKEY, WM_KEYDOWN,
    WM_KEYUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WM_XBUTTONDOWN,
    WM_XBUTTONUP, WNDCLASSW, XBUTTON1,
};
use windows::core::w;

use crate::keymap::{hotkey_modifiers, name_from_vk, vk_from_name};

pub const EMERGENCY_ID: u32 = 0xE5C;
const WM_REQUEST: u32 = WM_APP + 1;
const WATCHDOG_TIMER: usize = 1;
const WATCHDOG_PERIOD_MS: u32 = 10_000;
const LLMHF_INJECTED: u32 = 0x1;
const REPLY_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceEvent {
    Triggered(u32),
    Recording(RecordStep),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    #[error("another application already uses this shortcut")]
    AlreadyRegistered,
    #[error("unsupported key {0}")]
    UnknownKey(String),
    #[error("RegisterHotKey failed: {0}")]
    Os(String),
    #[error("the shortcut thread did not answer")]
    NoReply,
}

impl RegisterError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::AlreadyRegistered => "HOTKEY_TAKEN",
            Self::UnknownKey(_) => "HOTKEY_UNKNOWN_KEY",
            Self::Os(_) => "HOTKEY_OS_ERROR",
            Self::NoReply => "HOTKEY_NO_REPLY",
        }
    }
}

pub type EventSink = Box<dyn Fn(ServiceEvent) + Send + Sync>;
pub type EmergencyFn = Arc<dyn Fn() + Send + Sync>;

struct HookState {
    sink: EventSink,
    on_emergency: EmergencyFn,
    emergency: Mutex<Option<Hotkey>>,
    recording: AtomicBool,
    recorder: Mutex<Recorder>,
    mouse: Mutex<Vec<(u32, Hotkey)>>,
}

static ACTIVE: RwLock<Option<Arc<HookState>>> = RwLock::new(None);

fn active() -> Option<Arc<HookState>> {
    ACTIVE.read().ok().and_then(|g| g.clone())
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn held_modifiers() -> Vec<Modifier> {
    // SAFETY: GetAsyncKeyState only reads input state.
    let down = |vk: i32| unsafe { GetAsyncKeyState(vk) } < 0;
    let mut mods = Vec::new();
    if down(0x11) {
        mods.push(Modifier::Ctrl);
    }
    if down(0x12) {
        mods.push(Modifier::Alt);
    }
    if down(0x10) {
        mods.push(Modifier::Shift);
    }
    if down(0x5B) || down(0x5C) {
        mods.push(Modifier::Win);
    }
    mods
}

pub fn matches_combo(combo: &Hotkey, key: &str, mods: &[Modifier]) -> bool {
    combo.key == key && Hotkey::new(mods, key).mods == combo.mods
}

enum Request {
    Register(u32, Hotkey, mpsc::Sender<Result<(), RegisterError>>),
    UnregisterAll(mpsc::Sender<()>),
    SetEmergency(Hotkey, mpsc::Sender<Result<(), RegisterError>>),
    Record(bool),
}

pub struct HotkeyService {
    tx: mpsc::Sender<Request>,
    hwnd: isize,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for HotkeyService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HotkeyService")
            .field("hwnd", &self.hwnd)
            .finish_non_exhaustive()
    }
}

#[derive(Default)]
struct Hooks {
    keyboard: Option<HHOOK>,
    mouse: Option<HHOOK>,
}

impl Hooks {
    fn install(&mut self, want_mouse: bool) {
        self.remove();
        // SAFETY: hook procedures are 'static functions; the module handle is this executable.
        unsafe {
            let module = GetModuleHandleW(None).ok();
            self.keyboard = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_proc),
                module.map(Into::into),
                0,
            )
            .ok();
            if want_mouse {
                self.mouse =
                    SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), module.map(Into::into), 0)
                        .ok();
            }
        }
    }

    fn remove(&mut self) {
        for hook in [self.keyboard.take(), self.mouse.take()]
            .into_iter()
            .flatten()
        {
            // SAFETY: the handle came from SetWindowsHookExW on this thread and is removed once.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
    }
}

fn wants_mouse_hook(state: &HookState) -> bool {
    state.recording.load(Ordering::Acquire) || !lock(&state.mouse).is_empty()
}

fn register_keyboard(hwnd: HWND, id: u32, hotkey: &Hotkey) -> Result<(), RegisterError> {
    let vk =
        vk_from_name(&hotkey.key).ok_or_else(|| RegisterError::UnknownKey(hotkey.key.clone()))?;
    // SAFETY: hwnd is our live message window; registration is undone by UnregisterHotKey.
    unsafe {
        RegisterHotKey(
            Some(hwnd),
            id as i32,
            hotkey_modifiers(&hotkey.mods) | MOD_NOREPEAT,
            u32::from(vk),
        )
    }
    .map_err(|e| {
        if e.code() == ERROR_HOTKEY_ALREADY_REGISTERED.to_hresult() {
            RegisterError::AlreadyRegistered
        } else {
            RegisterError::Os(e.message())
        }
    })
}

fn unregister(hwnd: HWND, id: u32) {
    // SAFETY: unregistering an id that is not registered is harmless.
    let _ = unsafe { UnregisterHotKey(Some(hwnd), id as i32) };
}

impl HotkeyService {
    pub fn spawn(sink: EventSink, on_emergency: EmergencyFn) -> windows::core::Result<Self> {
        let state = Arc::new(HookState {
            sink,
            on_emergency,
            emergency: Mutex::new(None),
            recording: AtomicBool::new(false),
            recorder: Mutex::new(Recorder::default()),
            mouse: Mutex::new(Vec::new()),
        });
        if let Ok(mut slot) = ACTIVE.write() {
            *slot = Some(Arc::clone(&state));
        }
        let (tx, rx) = mpsc::channel::<Request>();
        let (ready_tx, ready_rx) = mpsc::channel::<windows::core::Result<isize>>();

        let thread = std::thread::Builder::new()
            .name("ch-hotkeys".into())
            .spawn(move || {
                // SAFETY: standard message-only window on this thread, destroyed before it exits.
                let hwnd = unsafe {
                    let instance = match GetModuleHandleW(None) {
                        Ok(i) => i,
                        Err(e) => {
                            let _ = ready_tx.send(Err(e));
                            return;
                        }
                    };
                    RegisterClassW(&WNDCLASSW {
                        lpfnWndProc: Some(window_proc),
                        hInstance: instance.into(),
                        lpszClassName: w!("ChAutoClickerHotkeys"),
                        ..Default::default()
                    });
                    match CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        w!("ChAutoClickerHotkeys"),
                        w!("CH Auto Clicker hotkeys"),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        Some(HWND_MESSAGE),
                        None,
                        Some(instance.into()),
                        None,
                    ) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = ready_tx.send(Err(e));
                            return;
                        }
                    }
                };
                let _ = ready_tx.send(Ok(hwnd.0 as isize));
                // SAFETY: timer on our own window; it lives as long as the window.
                unsafe { SetTimer(Some(hwnd), WATCHDOG_TIMER, WATCHDOG_PERIOD_MS, None) };

                let mut hooks = Hooks::default();
                hooks.install(wants_mouse_hook(&state));
                let mut registered: Vec<u32> = Vec::new();
                let mut msg = MSG::default();
                // SAFETY: classic message loop on the thread that owns the window.
                while unsafe { GetMessageW(&mut msg, None, 0, 0) }.as_bool() {
                    match msg.message {
                        WM_REQUEST => {
                            while let Ok(request) = rx.try_recv() {
                                handle_request(request, hwnd, &state, &mut registered);
                            }
                            hooks.install(wants_mouse_hook(&state));
                        }
                        WM_TIMER if msg.wParam.0 == WATCHDOG_TIMER => {
                            hooks.install(wants_mouse_hook(&state))
                        }
                        _ => {
                            // SAFETY: dispatching a message retrieved by GetMessageW.
                            unsafe { DispatchMessageW(&msg) };
                        }
                    }
                }
                hooks.remove();
                for id in registered {
                    unregister(hwnd, id);
                }
                unregister(hwnd, EMERGENCY_ID);
                // SAFETY: our own window, destroyed once.
                let _ = unsafe { DestroyWindow(hwnd) };
            })
            .expect("failed to spawn the hotkey thread");

        let hwnd = ready_rx
            .recv()
            .map_err(|_| windows::core::Error::empty())??;
        Ok(Self {
            tx,
            hwnd,
            thread: Some(thread),
        })
    }

    fn send(&self, request: Request) {
        let _ = self.tx.send(request);
        // SAFETY: posting to our own message window; it ignores stale posts after shutdown.
        let _ = unsafe {
            PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_REQUEST,
                WPARAM(0),
                LPARAM(0),
            )
        };
    }

    pub fn register(&self, id: u32, hotkey: &Hotkey) -> Result<(), RegisterError> {
        let (reply, answer) = mpsc::channel();
        self.send(Request::Register(id, hotkey.clone(), reply));
        answer
            .recv_timeout(REPLY_TIMEOUT)
            .unwrap_or(Err(RegisterError::NoReply))
    }

    pub fn unregister_all(&self) {
        let (reply, answer) = mpsc::channel();
        self.send(Request::UnregisterAll(reply));
        let _ = answer.recv_timeout(REPLY_TIMEOUT);
    }

    pub fn set_emergency(&self, hotkey: &Hotkey) -> Result<(), RegisterError> {
        let (reply, answer) = mpsc::channel();
        self.send(Request::SetEmergency(hotkey.clone(), reply));
        answer
            .recv_timeout(REPLY_TIMEOUT)
            .unwrap_or(Err(RegisterError::NoReply))
    }

    pub fn set_recording(&self, on: bool) {
        self.send(Request::Record(on));
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd
    }
}

fn handle_request(request: Request, hwnd: HWND, state: &HookState, registered: &mut Vec<u32>) {
    match request {
        Request::Register(id, hotkey, reply) => {
            let result = if is_mouse_key(&hotkey.key) {
                lock(&state.mouse).push((id, hotkey));
                Ok(())
            } else {
                register_keyboard(hwnd, id, &hotkey).inspect(|()| registered.push(id))
            };
            let _ = reply.send(result);
        }
        Request::UnregisterAll(reply) => {
            for id in registered.drain(..) {
                unregister(hwnd, id);
            }
            lock(&state.mouse).clear();
            let _ = reply.send(());
        }
        Request::SetEmergency(hotkey, reply) => {
            unregister(hwnd, EMERGENCY_ID);
            *lock(&state.emergency) = Some(hotkey.clone());
            let result = if is_mouse_key(&hotkey.key) {
                Ok(())
            } else {
                register_keyboard(hwnd, EMERGENCY_ID, &hotkey)
            };
            let _ = reply.send(result);
        }
        Request::Record(on) => {
            *lock(&state.recorder) = Recorder::default();
            state.recording.store(on, Ordering::Release);
        }
    }
}

impl Drop for HotkeyService {
    fn drop(&mut self) {
        // SAFETY: WM_QUIT-equivalent request to our own window's thread.
        let _ = unsafe {
            PostMessageW(
                Some(HWND(self.hwnd as *mut _)),
                WM_APP + 2,
                WPARAM(0),
                LPARAM(0),
            )
        };
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        if let Ok(mut slot) = ACTIVE.write() {
            *slot = None;
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_HOTKEY => {
            if let Some(state) = active() {
                let id = wparam.0 as u32;
                if id == EMERGENCY_ID {
                    (state.on_emergency)();
                } else {
                    (state.sink)(ServiceEvent::Triggered(id));
                }
            }
            LRESULT(0)
        }
        m if m == WM_APP + 2 => {
            // SAFETY: ends this thread's message loop.
            unsafe { PostQuitMessage(0) };
            LRESULT(0)
        }
        // SAFETY: forwarding unchanged arguments.
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

fn next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: forwarding the hook arguments unchanged.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return next(code, wparam, lparam);
    }
    let Some(state) = active() else {
        return next(code, wparam, lparam);
    };
    // SAFETY: for WH_KEYBOARD_LL, lparam points to a KBDLLHOOKSTRUCT valid during this call.
    let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let Some(key) = name_from_vk(info.vkCode as u16) else {
        return next(code, wparam, lparam);
    };
    let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
    let up = matches!(wparam.0 as u32, WM_KEYUP | WM_SYSKEYUP);

    if down
        && let Some(combo) = lock(&state.emergency).as_ref()
        && matches_combo(combo, &key, &held_modifiers())
    {
        (state.on_emergency)();
    }

    let injected = info.flags.0 & LLKHF_INJECTED.0 != 0;
    if state.recording.load(Ordering::Acquire) && !injected && (down || up) {
        let step = if down {
            lock(&state.recorder).key_down(&key)
        } else {
            lock(&state.recorder).key_up(&key)
        };
        (state.sink)(ServiceEvent::Recording(step));
        return LRESULT(1);
    }
    next(code, wparam, lparam)
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return next(code, wparam, lparam);
    }
    let Some(state) = active() else {
        return next(code, wparam, lparam);
    };
    // SAFETY: for WH_MOUSE_LL, lparam points to an MSLLHOOKSTRUCT valid during this call.
    let info = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
    if info.flags & LLMHF_INJECTED != 0 {
        return next(code, wparam, lparam);
    }
    let message = wparam.0 as u32;
    let key = match message {
        WM_MBUTTONDOWN | WM_MBUTTONUP => "MouseMiddle",
        WM_XBUTTONDOWN | WM_XBUTTONUP if (info.mouseData >> 16) as u16 == XBUTTON1 => "MouseX1",
        WM_XBUTTONDOWN | WM_XBUTTONUP => "MouseX2",
        _ => return next(code, wparam, lparam),
    };
    let down = matches!(message, WM_MBUTTONDOWN | WM_XBUTTONDOWN);

    if state.recording.load(Ordering::Acquire) {
        if down {
            let step = lock(&state.recorder).key_down(key);
            (state.sink)(ServiceEvent::Recording(step));
        }
        return LRESULT(1);
    }
    let mods = held_modifiers();
    let hit = lock(&state.mouse)
        .iter()
        .find(|(_, combo)| matches_combo(combo, key, &mods))
        .map(|(id, _)| *id);
    if let Some(id) = hit {
        if down {
            (state.sink)(ServiceEvent::Triggered(id));
        }
        return LRESULT(1);
    }
    next(code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use ch_core::model::Modifier::{Alt, Ctrl, Shift};
    use windows::Win32::UI::Input::KeyboardAndMouse::{MOD_ALT, MOD_CONTROL, MOD_SHIFT};

    use super::*;

    fn unusual(key: &str) -> Hotkey {
        Hotkey::new(&[Ctrl, Alt, Shift], key)
    }

    struct Fixture {
        service: HotkeyService,
        events: mpsc::Receiver<ServiceEvent>,
        emergencies: Arc<AtomicUsize>,
    }

    fn fixture() -> Fixture {
        let (tx, events) = mpsc::channel();
        let tx = Mutex::new(tx);
        let emergencies = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&emergencies);
        let service = HotkeyService::spawn(
            Box::new(move |e| {
                let _ = lock(&tx).send(e);
            }),
            Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        )
        .expect("service");
        Fixture {
            service,
            events,
            emergencies,
        }
    }

    fn post_hotkey(service: &HotkeyService, id: u32) {
        // SAFETY: synthetic WM_HOTKEY to our own window, as Windows would send it.
        unsafe {
            PostMessageW(
                Some(HWND(service.hwnd() as *mut _)),
                WM_HOTKEY,
                WPARAM(id as usize),
                LPARAM(0),
            )
        }
        .expect("post");
    }

    static SERIAL: Mutex<()> = Mutex::new(());

    #[test]
    fn registers_dispatches_and_detects_taken_combos() {
        let _serial = lock(&SERIAL);
        let f = fixture();
        f.service.register(7, &unusual("F24")).expect("free combo");
        post_hotkey(&f.service, 7);
        assert_eq!(
            f.events.recv_timeout(REPLY_TIMEOUT),
            Ok(ServiceEvent::Triggered(7))
        );

        f.service.unregister_all();
        // SAFETY: thread-level hotkey on the test thread, removed below.
        unsafe { RegisterHotKey(None, 99, MOD_CONTROL | MOD_ALT | MOD_SHIFT, 0x87) }
            .expect("test registration");
        assert_eq!(
            f.service.register(8, &unusual("F24")),
            Err(RegisterError::AlreadyRegistered)
        );
        // SAFETY: undoing the registration above.
        unsafe { UnregisterHotKey(None, 99) }.expect("unregister");
        f.service.register(8, &unusual("F24")).expect("free again");
    }

    #[test]
    fn the_emergency_id_calls_the_emergency_handler() {
        let _serial = lock(&SERIAL);
        let f = fixture();
        f.service.set_emergency(&unusual("F23")).expect("emergency");
        post_hotkey(&f.service, EMERGENCY_ID);
        let deadline = std::time::Instant::now() + REPLY_TIMEOUT;
        while f.emergencies.load(Ordering::SeqCst) == 0 {
            assert!(
                std::time::Instant::now() < deadline,
                "emergency handler never ran"
            );
            std::thread::yield_now();
        }
        assert!(
            f.events.try_recv().is_err(),
            "the emergency stop is not a regular event"
        );
    }

    #[test]
    fn mouse_combos_and_bad_keys() {
        let _serial = lock(&SERIAL);
        let f = fixture();
        f.service
            .register(3, &Hotkey::new(&[Ctrl], "MouseX2"))
            .expect("mouse combos go to the hook");
        assert_eq!(
            f.service.register(4, &Hotkey::new(&[], "Banana")),
            Err(RegisterError::UnknownKey("Banana".into()))
        );
        f.service.set_recording(true);
        f.service.set_recording(false);
        f.service.unregister_all();
        assert!(format!("{:?}", f.service).contains("HotkeyService"));
    }

    #[test]
    fn matches_combos_exactly() {
        let combo = Hotkey::new(&[Ctrl, Alt], "Escape");
        assert!(matches_combo(&combo, "Escape", &[Alt, Ctrl]));
        assert!(!matches_combo(&combo, "Escape", &[Ctrl]));
        assert!(!matches_combo(&combo, "Escape", &[Ctrl, Alt, Shift]));
        assert!(!matches_combo(&combo, "F6", &[Ctrl, Alt]));
    }

    #[test]
    fn error_codes() {
        assert_eq!(RegisterError::AlreadyRegistered.code(), "HOTKEY_TAKEN");
        assert_eq!(
            RegisterError::UnknownKey("x".into()).code(),
            "HOTKEY_UNKNOWN_KEY"
        );
        assert_eq!(RegisterError::Os("x".into()).code(), "HOTKEY_OS_ERROR");
        assert_eq!(RegisterError::NoReply.code(), "HOTKEY_NO_REPLY");
    }
}
