use std::time::Duration;

use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
use windows::Win32::UI::Shell::{SEE_MASK_NOASYNC, SHELLEXECUTEINFOW, ShellExecuteExW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, PCWSTR, w};

use crate::integrity::current_integrity;

const HIGH_INTEGRITY: u32 = 0x3000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relaunch {
    Started,
    Declined,
}

pub fn is_elevated() -> bool {
    current_integrity().is_some_and(|level| level >= HIGH_INTEGRITY)
}

pub fn parameters(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if a.is_empty() || a.contains([' ', '\t', '"']) {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn relaunch_elevated(args: &[String]) -> Result<Relaunch, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let file = HSTRING::from(exe.as_os_str());
    let params = HSTRING::from(parameters(args));
    let mut info = SHELLEXECUTEINFOW {
        cbSize: size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOASYNC,
        lpVerb: w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: PCWSTR(params.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };
    // SAFETY: `info` is fully initialized, sized by cbSize, and its strings outlive the call.
    match unsafe { ShellExecuteExW(&mut info) } {
        Ok(()) => Ok(Relaunch::Started),
        Err(e) if e.code() == ERROR_CANCELLED.to_hresult() => Ok(Relaunch::Declined),
        Err(e) => Err(e.to_string()),
    }
}

pub fn wait_for_exit(pid: u32, timeout: Duration) -> bool {
    // SAFETY: the handle is checked by `let Ok` and closed before returning.
    unsafe {
        let Ok(process) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else {
            return true;
        };
        let ms = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX);
        let ended = WaitForSingleObject(process, ms).0 == 0;
        let _ = CloseHandle(process);
        ended
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_only_what_needs_quoting() {
        let args = ["--minimized", "--after-pid", "42", "C:\\a b\\c", ""].map(String::from);
        assert_eq!(
            parameters(&args),
            "--minimized --after-pid 42 \"C:\\a b\\c\" \"\""
        );
    }

    #[test]
    fn a_test_run_is_not_elevated_unless_the_shell_is() {
        assert_eq!(
            is_elevated(),
            current_integrity().is_some_and(|l| l >= HIGH_INTEGRITY)
        );
    }

    #[test]
    fn waiting_for_a_missing_process_returns_at_once() {
        assert!(wait_for_exit(0, Duration::from_secs(5)));
    }

    #[test]
    fn waiting_for_a_live_process_times_out() {
        assert!(!wait_for_exit(
            std::process::id(),
            Duration::from_millis(10)
        ));
    }
}
