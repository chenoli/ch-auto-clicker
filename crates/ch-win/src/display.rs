use ch_core::geometry::{DisplayLayout, Monitor, Rect};
use windows::Win32::Foundation::{LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::UI::HiDpi::{
    AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
    DPI_AWARENESS_CONTEXT_UNAWARE, DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED, GetDpiForMonitor,
    GetThreadDpiAwarenessContext, MDT_EFFECTIVE_DPI, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetSystemMetrics, MONITORINFOF_PRIMARY, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
    SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
};
use windows::core::BOOL;

const BASE_DPI: f32 = 96.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DpiAwareness {
    Unaware,
    UnawareGdiScaled,
    SystemAware,
    PerMonitor,
    PerMonitorV2,
    Unknown,
}

impl DpiAwareness {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unaware => "Unaware",
            Self::UnawareGdiScaled => "UnawareGdiScaled",
            Self::SystemAware => "SystemAware",
            Self::PerMonitor => "PerMonitor",
            Self::PerMonitorV2 => "PerMonitorV2",
            Self::Unknown => "Unknown",
        }
    }
}

pub fn current_dpi_awareness() -> DpiAwareness {
    // SAFETY: no preconditions; reads the calling thread's context.
    let current = unsafe { GetThreadDpiAwarenessContext() };
    let known: [(DPI_AWARENESS_CONTEXT, DpiAwareness); 5] = [
        (
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            DpiAwareness::PerMonitorV2,
        ),
        (
            DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
            DpiAwareness::PerMonitor,
        ),
        (
            DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
            DpiAwareness::SystemAware,
        ),
        (
            DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED,
            DpiAwareness::UnawareGdiScaled,
        ),
        (DPI_AWARENESS_CONTEXT_UNAWARE, DpiAwareness::Unaware),
    ];
    known
        .into_iter()
        // SAFETY: both arguments are valid context handles (system pseudo-handles or the current one).
        .find(|(ctx, _)| unsafe { AreDpiAwarenessContextsEqual(current, *ctx) }.as_bool())
        .map_or(DpiAwareness::Unknown, |(_, mode)| mode)
}

pub fn ensure_per_monitor_v2() -> DpiAwareness {
    // SAFETY: plain value argument; fails harmlessly with ACCESS_DENIED once a mode is set.
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    current_dpi_awareness()
}

pub fn virtual_desktop() -> Rect {
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

pub fn display_layout() -> DisplayLayout {
    let mut handles: Vec<HMONITOR> = Vec::new();
    // SAFETY: the callback only runs during this call and `handles` outlives it; LPARAM carries
    // a pointer to it, cast back in `collect_monitor`.
    let _ = unsafe {
        EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut handles as *mut Vec<HMONITOR> as isize),
        )
    };
    DisplayLayout {
        virtual_desktop: virtual_desktop(),
        monitors: handles.into_iter().filter_map(describe_monitor).collect(),
    }
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _clip: *mut RECT,
    data: LPARAM,
) -> BOOL {
    // SAFETY: `data` is the `&mut Vec<HMONITOR>` passed by `display_layout`, alive for the whole
    // enumeration and not aliased elsewhere during it.
    let handles = unsafe { &mut *(data.0 as *mut Vec<HMONITOR>) };
    handles.push(monitor);
    true.into()
}

fn describe_monitor(monitor: HMONITOR) -> Option<Monitor> {
    let mut info = MONITORINFOEXW {
        monitorInfo: MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: `info` is a MONITORINFOEXW with cbSize set, so the OS may write the extended form.
    let ok = unsafe { GetMonitorInfoW(monitor, &mut info.monitorInfo) };
    if !ok.as_bool() {
        return None;
    }

    let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
    // SAFETY: valid monitor handle and writable out-parameters.
    let scale = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) }
        .map_or(1.0, |()| dpi_x as f32 / BASE_DPI);

    let name_len = info
        .szDevice
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(info.szDevice.len());
    let m = info.monitorInfo;
    Some(Monitor {
        id: String::from_utf16_lossy(&info.szDevice[..name_len]),
        rect: from_win_rect(m.rcMonitor),
        work_area: from_win_rect(m.rcWork),
        scale,
        primary: m.dwFlags & MONITORINFOF_PRIMARY != 0,
    })
}

fn from_win_rect(r: RECT) -> Rect {
    Rect::from_edges(r.left, r.top, r.right, r.bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opts_into_per_monitor_v2() {
        assert_eq!(ensure_per_monitor_v2(), DpiAwareness::PerMonitorV2);
        assert_eq!(DpiAwareness::PerMonitorV2.as_str(), "PerMonitorV2");
    }

    #[test]
    fn enumerates_the_real_monitors() {
        ensure_per_monitor_v2();
        let layout = display_layout();
        assert!(!layout.monitors.is_empty(), "at least one monitor");
        assert_eq!(layout.monitors.iter().filter(|m| m.primary).count(), 1);
        for m in &layout.monitors {
            assert!(m.id.starts_with("\\\\.\\"), "device name: {}", m.id);
            assert!(layout.virtual_desktop.contains_rect(&m.rect));
            assert!(m.rect.contains_rect(&m.work_area));
            assert!(m.scale >= 1.0);
        }
        let primary = layout.primary().expect("primary");
        assert_eq!((primary.rect.left, primary.rect.top), (0, 0));
    }
}
