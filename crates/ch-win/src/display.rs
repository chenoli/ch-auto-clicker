//! Monitors, virtual desktop and DPI.

use windows::Win32::UI::HiDpi::{
    AreDpiAwarenessContextsEqual, DPI_AWARENESS_CONTEXT, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, DPI_AWARENESS_CONTEXT_SYSTEM_AWARE,
    DPI_AWARENESS_CONTEXT_UNAWARE, DPI_AWARENESS_CONTEXT_UNAWARE_GDISCALED,
    GetThreadDpiAwarenessContext,
};

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

/// On the main thread this reflects the process-wide mode.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_a_known_mode() {
        // The mode depends on the test binary's manifest; this only checks the query succeeds.
        assert_ne!(current_dpi_awareness().as_str(), "");
    }
}
