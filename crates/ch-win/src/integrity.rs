use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{
    GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TOKEN_MANDATORY_LABEL,
    TOKEN_QUERY, TokenIntegrityLevel,
};
use windows::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
};

pub fn current_integrity() -> Option<u32> {
    // SAFETY: the pseudo handle of the current process needs no closing.
    token_integrity(unsafe { GetCurrentProcess() })
}

pub fn process_integrity(pid: u32) -> Option<u32> {
    // SAFETY: the handle is checked by `?` and closed before returning.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let level = token_integrity(process);
        let _ = CloseHandle(process);
        level
    }
}

fn token_integrity(process: HANDLE) -> Option<u32> {
    // SAFETY: the token is closed before returning; the label is read from a buffer sized
    // and aligned for TOKEN_MANDATORY_LABEL by GetTokenInformation, and its SID points into it.
    unsafe {
        let mut token = HANDLE::default();
        OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
        let mut len = 0u32;
        let _ = GetTokenInformation(token, TokenIntegrityLevel, None, 0, &mut len);
        let mut buffer = vec![0u64; (len as usize).div_ceil(8).max(1)];
        let read = GetTokenInformation(
            token,
            TokenIntegrityLevel,
            Some(buffer.as_mut_ptr().cast()),
            len,
            &mut len,
        );
        let _ = CloseHandle(token);
        read.ok()?;
        let label = &*buffer.as_ptr().cast::<TOKEN_MANDATORY_LABEL>();
        let sid = label.Label.Sid;
        let count = *GetSidSubAuthorityCount(sid);
        Some(*GetSidSubAuthority(sid, u32::from(count.checked_sub(1)?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_level_of_this_process() {
        let own = current_integrity().expect("own level");
        assert!(own >= 0x2000, "at least medium: {own:#x}");
        assert_eq!(process_integrity(std::process::id()), Some(own));
    }

    #[test]
    fn an_unknown_process_has_no_level() {
        assert_eq!(process_integrity(0), None);
    }
}
