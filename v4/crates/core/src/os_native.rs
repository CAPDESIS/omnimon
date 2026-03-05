use crate::killer::{is_immutable_blocked_process_name, KillError};

#[cfg(target_os = "windows")]
pub(crate) fn kill_process_force(pid: u32, process_name: &str) -> Result<(), KillError> {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    struct HandleGuard(HANDLE);

    impl HandleGuard {
        fn new(handle: HANDLE) -> Self {
            Self(handle)
        }

        fn raw(&self) -> HANDLE {
            self.0
        }
    }

    impl Drop for HandleGuard {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: `self.0` was returned by OpenProcess and is owned by this guard.
                // We close it exactly once on drop, preventing handle leaks.
                unsafe {
                    let _ = CloseHandle(self.0);
                }
            }
        }
    }

    if is_immutable_blocked_process_name(process_name) {
        return Err(KillError::Blocked(process_name.to_string()));
    }

    // SAFETY: OpenProcess is an FFI call. Inputs are plain values and do not borrow memory.
    // We immediately wrap the returned HANDLE in HandleGuard for deterministic cleanup.
    let handle = unsafe { OpenProcess(PROCESS_TERMINATE, false, pid) }
        .map_err(|_| KillError::KillFailed(pid))?;
    let handle = HandleGuard::new(handle);

    // SAFETY: TerminateProcess is called with a valid process HANDLE owned by HandleGuard.
    let terminated = unsafe { TerminateProcess(handle.raw(), 1) }
        .map(|_| true)
        .unwrap_or(false);

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

    let native_pid = libc::pid_t::try_from(pid).map_err(|_| KillError::KillFailed(pid))?;
    // SAFETY: libc::kill is an FFI call with POD parameters. `native_pid` conversion is checked.
    let rc = unsafe { libc::kill(native_pid, libc::SIGKILL) };
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
