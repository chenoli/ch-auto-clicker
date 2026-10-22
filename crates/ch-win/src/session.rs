use windows::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SESSION, WTS_SESSIONSTATE_LOCK, WTSFreeMemory, WTSINFOEXW,
    WTSQuerySessionInformationW, WTSSessionInfoEx,
};
use windows::core::PWSTR;

pub fn is_locked() -> bool {
    let mut buffer = PWSTR::null();
    let mut len = 0u32;
    // SAFETY: WTS allocates the buffer, which is read as WTSINFOEXW only when it is large
    // enough and is level 1, and is freed with WTSFreeMemory before returning.
    unsafe {
        if WTSQuerySessionInformationW(
            None,
            WTS_CURRENT_SESSION,
            WTSSessionInfoEx,
            &mut buffer,
            &mut len,
        )
        .is_err()
            || buffer.is_null()
        {
            return false;
        }
        let info = buffer.0.cast::<WTSINFOEXW>();
        let locked = len as usize >= size_of::<WTSINFOEXW>()
            && (*info).Level == 1
            && (*info).Data.WTSInfoExLevel1.SessionFlags == WTS_SESSIONSTATE_LOCK as i32;
        WTSFreeMemory(buffer.0.cast());
        locked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_test_desktop_is_not_locked() {
        assert!(!is_locked());
    }
}
