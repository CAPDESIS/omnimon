use crate::killer::{is_immutable_blocked_process_name, KillError};

#[cfg(target_os = "windows")]
pub(crate) fn kill_process_force(pid: u32, process_name: &str) -> Result<(), KillError> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    if is_immutable_blocked_process_name(process_name) {
        return Err(KillError::Blocked(process_name.to_string()));
    }

    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }
        .map_err(|_| KillError::KillFailed(pid))?;

    let terminated = unsafe { TerminateProcess(handle, 1) }
        .map(|_| true)
        .unwrap_or(false);

    unsafe {
        let _ = CloseHandle(handle);
    }

    if terminated {
        Ok(())
    } else {
        Err(KillError::KillFailed(pid))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(crate) fn kill_process_force(pid: u32, process_name: &str) -> Result<(), KillError> {
    if is_immutable_blocked_process_name(process_name) {
        return Err(KillError::Blocked(process_name.to_string()));
    }

    let rc = unsafe { libc::kill(pid as libc::pid_t, libc::SIGKILL) };
    if rc == 0 {
        Ok(())
    } else {
        Err(KillError::KillFailed(pid))
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
pub(crate) fn kill_process_force(pid: u32, process_name: &str) -> Result<(), KillError> {
    if is_immutable_blocked_process_name(process_name) {
        return Err(KillError::Blocked(process_name.to_string()));
    }
    Err(KillError::KillFailed(pid))
}
