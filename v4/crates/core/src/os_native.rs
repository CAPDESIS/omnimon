use crate::killer::{is_immutable_blocked_process, KillError};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct NativeMemorySnapshot {
    pub total_memory_bytes: u64,
    pub free_memory_bytes: u64,
    pub used_memory_bytes: u64,
    pub free_percent: u32,
    pub swap_used_mb: u64,
}

#[cfg(target_os = "macos")]
pub fn collect_native_memory_snapshot() -> Option<NativeMemorySnapshot> {
    use std::process::Command;

    let total_memory_bytes = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|out| {
            if !out.status.success() {
                return None;
            }
            String::from_utf8(out.stdout).ok()
        })
        .and_then(|s| s.trim().parse::<u64>().ok())?;

    let vm_stat_output = Command::new("vm_stat").output().ok().and_then(|out| {
        if out.status.success() {
            String::from_utf8(out.stdout).ok()
        } else {
            None
        }
    })?;

    let mut page_size: u64 = 4096;
    let mut free_pages: u64 = 0;
    let mut inactive_pages: u64 = 0;
    for line in vm_stat_output.lines() {
        if line.contains("page size of") && line.contains("bytes") {
            let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(parsed) = digits.parse::<u64>() {
                page_size = parsed;
            }
        } else if line.starts_with("Pages free") {
            let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(parsed) = digits.parse::<u64>() {
                free_pages = parsed;
            }
        } else if line.starts_with("Pages inactive") {
            let digits: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(parsed) = digits.parse::<u64>() {
                inactive_pages = parsed;
            }
        }
    }

    let free_bytes = free_pages.saturating_mul(page_size);
    let inactive_bytes = inactive_pages.saturating_mul(page_size);
    let available_bytes = free_bytes.saturating_add(inactive_bytes);
    let used_memory_bytes = total_memory_bytes.saturating_sub(available_bytes);
    let free_percent = if total_memory_bytes > 0 {
        ((available_bytes as f64 / total_memory_bytes as f64) * 100.0)
            .round()
            .clamp(0.0, 100.0) as u32
    } else {
        0
    };

    let swap_used_mb = Command::new("sysctl")
        .args(["-n", "vm.swapusage"])
        .output()
        .ok()
        .and_then(|out| {
            if !out.status.success() {
                return None;
            }
            String::from_utf8(out.stdout).ok()
        })
        .and_then(|line| {
            let marker = "used =";
            let pos = line.find(marker)?;
            let tail = &line[pos + marker.len()..];
            let mut num = String::new();
            for ch in tail.chars() {
                if ch.is_ascii_digit() || ch == '.' {
                    num.push(ch);
                } else if !num.is_empty() {
                    break;
                }
            }
            let value = num.parse::<f64>().ok()?;
            if tail.contains('G') {
                Some((value * 1024.0) as u64)
            } else {
                Some(value as u64)
            }
        })
        .unwrap_or(0);

    Some(NativeMemorySnapshot {
        total_memory_bytes,
        free_memory_bytes: available_bytes,
        used_memory_bytes,
        free_percent,
        swap_used_mb,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn collect_native_memory_snapshot() -> Option<NativeMemorySnapshot> {
    None
}

#[cfg(target_os = "windows")]
pub(crate) fn kill_process_force(
    pid: u32,
    process_name: &str,
    exe_path: Option<&Path>,
) -> Result<(), KillError> {
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

    if is_immutable_blocked_process(process_name, exe_path) {
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
pub(crate) fn kill_process_force(
    pid: u32,
    process_name: &str,
    exe_path: Option<&Path>,
) -> Result<(), KillError> {
    if is_immutable_blocked_process(process_name, exe_path) {
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
pub(crate) fn kill_process_force(
    pid: u32,
    process_name: &str,
    exe_path: Option<&Path>,
) -> Result<(), KillError> {
    if is_immutable_blocked_process(process_name, exe_path) {
        return Err(KillError::Blocked(process_name.to_string()));
    }
    Err(KillError::KillFailed(pid))
}
