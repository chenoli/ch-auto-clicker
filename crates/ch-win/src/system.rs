//! Machine identity used to invalidate cached timing measurements (ARCHITECTURE §5.3).

use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows::core::{HSTRING, PCWSTR};

const CURRENT_VERSION: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion";
const CPU_KEY: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";

fn read_string(key: &str, value: &str) -> Option<String> {
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let mut buffer = [0u16; 256];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: buffer and size describe the same writable allocation; strings are null-terminated.
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&mut size),
        )
    }
    .ok()
    .ok()?;
    let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..len]).trim().to_owned())
}

fn read_dword(key: &str, value: &str) -> Option<u32> {
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let mut data = 0u32;
    let mut size = 4u32;
    // SAFETY: `data` is a writable 4-byte DWORD and `size` says so.
    unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            PCWSTR(key.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut data as *mut u32).cast()),
            Some(&mut size),
        )
    }
    .ok()
    .ok()?;
    Some(data)
}

/// e.g. `26300.1234 | AMD Ryzen 7 5800X 8-Core Processor | 16 threads`.
pub fn machine_fingerprint() -> String {
    let build = read_string(CURRENT_VERSION, "CurrentBuild").unwrap_or_else(|| "?".into());
    let ubr = read_dword(CURRENT_VERSION, "UBR").map_or_else(String::new, |u| format!(".{u}"));
    let cpu = read_string(CPU_KEY, "ProcessorNameString").unwrap_or_else(|| "unknown CPU".into());
    let threads = std::thread::available_parallelism().map_or(0, |n| n.get());
    format!("{build}{ubr} | {cpu} | {threads} threads")
}

/// `Some(true)` on battery, `Some(false)` on AC, `None` when Windows cannot tell.
pub fn on_battery() -> Option<bool> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: writes into a valid, correctly sized local struct.
    unsafe { GetSystemPowerStatus(&mut status) }.ok()?;
    match status.ACLineStatus {
        0 => Some(true),
        1 => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_names_build_cpu_and_threads() {
        let fp = machine_fingerprint();
        let parts: Vec<_> = fp.split(" | ").collect();
        assert_eq!(parts.len(), 3, "{fp}");
        assert!(
            parts[0].chars().next().is_some_and(|c| c.is_ascii_digit()),
            "{fp}"
        );
        assert_ne!(parts[1], "unknown CPU");
        assert!(parts[2].ends_with("threads"));
        assert_eq!(fp, machine_fingerprint(), "stable across calls");
    }

    #[test]
    fn power_status_is_readable() {
        // Desktops report AC; laptops either. Only "unknown" would be suspicious here.
        let _ = on_battery();
    }

    #[test]
    fn missing_values_are_none() {
        assert_eq!(read_string(CURRENT_VERSION, "DefinitelyNotAValue"), None);
        assert_eq!(read_dword(CURRENT_VERSION, "DefinitelyNotAValue"), None);
    }
}
