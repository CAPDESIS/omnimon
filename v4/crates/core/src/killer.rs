use serde::{Deserialize, Serialize};
use std::fmt;
use sysinfo::{Pid, Signal, System};

const DEFAULT_PROTECTED_PROCESSES: &[&str] = &[
    "launchd",
    "kernel_task",
    "windowserver",
    "systemd",
    "init",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
];

#[cfg(target_os = "macos")]
const MACOS_PROTECTED_PROCESSES: &[&str] = &[
    "coreaudiod",
    "audiocomponentregistrar",
    "coremediaiod",
    "vtdecoderxpcservice",
    "vtencoderxpcservice",
    "loginwindow",
    "bluetoothd",
    "fseventsd",
    "mds",
    "mds_stores",
    "opendirectoryd",
    "syslogd",
    "configd",
    "diskarbitrationd",
    "powerd",
    "thermalmonitord",
    "usereventagent",
    "cfprefsd",
    "distnoted",
    "logd",
    "notifyd",
];

#[cfg(target_os = "windows")]
const WINDOWS_PROTECTED_PROCESSES: &[&str] = &[
    "svchost.exe",
    "explorer.exe",
    "winlogon.exe",
    "dwm.exe",
    "csrss.exe",
    "smss.exe",
    "wininit.exe",
    "services.exe",
    "lsass.exe",
];

#[cfg(target_os = "linux")]
const LINUX_PROTECTED_PROCESSES: &[&str] = &[
    "systemd",
    "init",
    "xorg",
    "xwayland",
    "dbus-daemon",
    "networkmanager",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillResult {
    pub pid: u32,
    pub process_name: String,
    pub killed: bool,
}

#[derive(Debug)]
pub enum KillError {
    InvalidPid(i32),
    ProcessNotFound(u32),
    Blocked(String),
    KillFailed(u32),
}

impl fmt::Display for KillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KillError::InvalidPid(pid) => write!(f, "invalid pid: {pid}"),
            KillError::ProcessNotFound(pid) => write!(f, "process not found: {pid}"),
            KillError::Blocked(name) => {
                write!(f, "refusing to kill protected process: {name}")
            }
            KillError::KillFailed(pid) => write!(f, "failed to kill process: {pid}"),
        }
    }
}

impl std::error::Error for KillError {}

pub fn is_immutable_blocked_process_name(process_name: &str) -> bool {
    let lowered_name = process_name.to_ascii_lowercase();
    if DEFAULT_PROTECTED_PROCESSES
        .iter()
        .any(|name| *name == lowered_name)
    {
        return true;
    }

    #[cfg(target_os = "macos")]
    {
        if MACOS_PROTECTED_PROCESSES
            .iter()
            .any(|name| *name == lowered_name)
        {
            return true;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if WINDOWS_PROTECTED_PROCESSES
            .iter()
            .any(|name| *name == lowered_name)
        {
            return true;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if LINUX_PROTECTED_PROCESSES
            .iter()
            .any(|name| *name == lowered_name)
        {
            return true;
        }
    }

    false
}

fn is_blocked_process_name(process_name: &str, extra_blocklist: &[String]) -> bool {
    let is_default_blocked = is_immutable_blocked_process_name(process_name);

    let is_extra_blocked = extra_blocklist
        .iter()
        .any(|name| name.eq_ignore_ascii_case(process_name));

    is_default_blocked || is_extra_blocked
}

fn kill_process_by_name(
    pid: u32,
    process_name: String,
    extra_blocklist: &[String],
    terminate: impl FnOnce() -> bool,
) -> Result<KillResult, KillError> {
    if is_blocked_process_name(&process_name, extra_blocklist) {
        return Err(KillError::Blocked(process_name));
    }

    let killed = terminate();
    if !killed {
        return Err(KillError::KillFailed(pid));
    }

    Ok(KillResult {
        pid,
        process_name,
        killed,
    })
}

pub fn kill_process_safe(pid: i32, extra_blocklist: &[String]) -> Result<KillResult, KillError> {
    if pid <= 1 {
        return Err(KillError::InvalidPid(pid));
    }

    let pid_u32 = pid as u32;
    let mut system = System::new_all();
    system.refresh_all();

    let process_pid = Pid::from_u32(pid_u32);
    let process = system
        .process(process_pid)
        .ok_or(KillError::ProcessNotFound(pid_u32))?;

    let process_name = process.name().to_string();
    kill_process_by_name(pid_u32, process_name, extra_blocklist, || {
        let graceful = process.kill_with(Signal::Term).unwrap_or(false) || process.kill();
        if graceful {
            return true;
        }

        crate::os_native::kill_process_force(pid_u32, process.name()).is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn blocklist_rejects_protected_process_and_does_not_kill() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);

        let result = kill_process_by_name(1234, "WindowServer".to_string(), &[], move || {
            *called_clone.lock().expect("lock kill flag") = true;
            true
        });

        assert!(matches!(result, Err(KillError::Blocked(name)) if name == "WindowServer"));
        assert!(!*called.lock().expect("lock kill flag"));
    }

    #[test]
    fn pid_zero_is_rejected() {
        let result = kill_process_safe(0, &[]);
        assert!(matches!(result, Err(KillError::InvalidPid(0))));
    }

    #[test]
    fn non_existent_pid_returns_process_not_found() {
        let mut system = System::new_all();
        system.refresh_all();

        let mut candidate: u32 = 500_000;
        while system.process(Pid::from_u32(candidate)).is_some() {
            candidate = candidate.saturating_add(1);
        }

        let result = kill_process_safe(candidate as i32, &[]);
        assert!(matches!(result, Err(KillError::ProcessNotFound(pid)) if pid == candidate));
    }
}
