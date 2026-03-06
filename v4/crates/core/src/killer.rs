use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::time::Duration;
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

fn path_is_trusted_for_blocked_process(exe_path: &Path) -> bool {
    let path_lc = exe_path.to_string_lossy().to_ascii_lowercase();
    if cfg!(target_os = "macos") {
        path_lc.starts_with("/system/")
            || path_lc.starts_with("/usr/libexec/")
            || path_lc.starts_with("/usr/sbin/")
            || path_lc == "/sbin/launchd"
    } else if cfg!(target_os = "windows") {
        path_lc.starts_with("c:\\windows\\system32\\")
            || path_lc.starts_with("c:\\windows\\syswow64\\")
            || path_lc == "c:\\windows\\explorer.exe"
    } else if cfg!(target_os = "linux") {
        path_lc.starts_with("/sbin/")
            || path_lc.starts_with("/usr/sbin/")
            || path_lc.starts_with("/lib/systemd/")
            || path_lc.starts_with("/usr/lib/systemd/")
            || path_lc == "/usr/bin/xorg"
            || path_lc == "/usr/lib/xorg/xorg"
    } else {
        false
    }
}

pub(crate) fn is_immutable_blocked_process(process_name: &str, exe_path: Option<&Path>) -> bool {
    if !is_immutable_blocked_process_name(process_name) {
        return false;
    }

    match exe_path {
        Some(path) => path_is_trusted_for_blocked_process(path),
        None => false,
    }
}

fn is_blocked_process_name(
    process_name: &str,
    exe_path: Option<&Path>,
    extra_blocklist: &[String],
) -> bool {
    let is_default_blocked = is_immutable_blocked_process(process_name, exe_path);

    let is_extra_blocked = extra_blocklist
        .iter()
        .any(|name| name.eq_ignore_ascii_case(process_name));

    is_default_blocked || is_extra_blocked
}

#[cfg(test)]
fn kill_process_by_name(
    pid: u32,
    process_name: String,
    exe_path: Option<&Path>,
    extra_blocklist: &[String],
    terminate: impl FnOnce() -> bool,
) -> Result<KillResult, KillError> {
    if is_blocked_process_name(&process_name, exe_path, extra_blocklist) {
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

    // Extract process info and attempt graceful kill while the borrow is active.
    let (process_name, process_exe, graceful) = {
        let process = system
            .process(process_pid)
            .ok_or(KillError::ProcessNotFound(pid_u32))?;

        let name = process.name().to_string();
        let exe = process.exe().map(|p| p.to_path_buf());

        // Check blocklist before attempting any kill
        if is_blocked_process_name(&name, exe.as_deref(), extra_blocklist) {
            return Err(KillError::Blocked(name));
        }

        let graceful = process.kill_with(Signal::Term).unwrap_or(false) || process.kill();
        (name, exe, graceful)
    };
    // The immutable borrow on `system` is now dropped.

    let killed = if graceful {
        std::thread::sleep(Duration::from_millis(120));
        if !process_is_alive(&mut system, pid_u32) {
            true
        } else if !identity_matches(&mut system, pid_u32, &process_name, process_exe.as_deref())
            || crate::os_native::kill_process_force(pid_u32, &process_name, process_exe.as_deref())
                .is_err()
        {
            false
        } else {
            std::thread::sleep(Duration::from_millis(120));
            !process_is_alive(&mut system, pid_u32)
        }
    } else {
        false
    };

    if !killed {
        return Err(KillError::KillFailed(pid_u32));
    }

    Ok(KillResult {
        pid: pid_u32,
        process_name,
        killed,
    })
}

fn process_is_alive(system: &mut System, pid: u32) -> bool {
    system.refresh_all();
    system.process(Pid::from_u32(pid)).is_some()
}

fn identity_matches(
    system: &mut System,
    pid: u32,
    expected_name: &str,
    expected_exe: Option<&Path>,
) -> bool {
    system.refresh_all();
    let Some(current) = system.process(Pid::from_u32(pid)) else {
        return false;
    };

    if current.name() != expected_name {
        return false;
    }

    match (expected_exe, current.exe()) {
        (Some(expected), Some(current_exe)) => current_exe == expected,
        (None, _) => true,
        (Some(_), None) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn blocklist_rejects_protected_process_and_does_not_kill() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);

        #[cfg(target_os = "macos")]
        let (proc_name, path) = ("WindowServer", "/System/Library/CoreServices/WindowServer");
        #[cfg(target_os = "windows")]
        let (proc_name, path) = ("svchost.exe", "C:\\Windows\\System32\\svchost.exe");
        #[cfg(target_os = "linux")]
        let (proc_name, path) = ("systemd", "/usr/lib/systemd/systemd");

        let result = kill_process_by_name(
            1234,
            proc_name.to_string(),
            Some(Path::new(path)),
            &[],
            move || {
                *called_clone.lock().expect("lock kill flag") = true;
                true
            },
        );

        assert!(matches!(result, Err(KillError::Blocked(name)) if name == proc_name));
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

    #[test]
    fn spoofed_blocked_name_with_untrusted_path_is_not_blocked() {
        #[cfg(target_os = "macos")]
        let (proc_name, path) = ("WindowServer", "/tmp/WindowServer");
        #[cfg(target_os = "windows")]
        let (proc_name, path) = ("svchost.exe", "C:\\Temp\\svchost.exe");
        #[cfg(target_os = "linux")]
        let (proc_name, path) = ("systemd", "/tmp/systemd");

        let result = kill_process_by_name(
            99,
            proc_name.to_string(),
            Some(Path::new(path)),
            &[],
            || true,
        );
        assert!(matches!(result, Ok(KillResult { killed: true, .. })));
    }

    #[test]
    fn extra_blocklist_blocks_process_by_name() {
        let extra = vec!["mydaemon".to_string()];
        let result = kill_process_by_name(
            77,
            "mydaemon".to_string(),
            Some(Path::new("/opt/mydaemon")),
            &extra,
            || true,
        );
        assert!(matches!(result, Err(KillError::Blocked(name)) if name == "mydaemon"));
    }

    #[test]
    fn denied_termination_path_maps_to_kill_failed() {
        let result = kill_process_by_name(
            100,
            "user-app".to_string(),
            Some(Path::new("/tmp/user-app")),
            &[],
            || false,
        );
        assert!(matches!(result, Err(KillError::KillFailed(100))));
    }

    #[test]
    fn immutable_blocked_requires_trusted_executable_path() {
        #[cfg(target_os = "macos")]
        {
            assert!(is_immutable_blocked_process(
                "WindowServer",
                Some(Path::new("/System/Library/CoreServices/WindowServer"))
            ));
            assert!(!is_immutable_blocked_process(
                "WindowServer",
                Some(Path::new("/tmp/WindowServer"))
            ));
        }

        #[cfg(target_os = "windows")]
        {
            assert!(is_immutable_blocked_process(
                "svchost.exe",
                Some(Path::new("C:\\Windows\\System32\\svchost.exe"))
            ));
            assert!(!is_immutable_blocked_process(
                "svchost.exe",
                Some(Path::new("C:\\Temp\\svchost.exe"))
            ));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(is_immutable_blocked_process(
                "systemd",
                Some(Path::new("/usr/lib/systemd/systemd"))
            ));
            assert!(!is_immutable_blocked_process(
                "systemd",
                Some(Path::new("/tmp/systemd"))
            ));
        }
    }

    #[test]
    fn kill_process_safe_terminates_spawned_child() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("spawn sleep child process");
        let pid = child.id() as i32;

        // Spawn a thread to wait on the child so it doesn't become a zombie on Linux
        std::thread::spawn(move || {
            let _ = child.wait();
        });

        // Give sysinfo a moment to definitely see the new process
        std::thread::sleep(Duration::from_millis(200));

        let result = kill_process_safe(pid, &[]);
        assert!(result.is_ok(), "expected kill success, got: {result:?}");
    }
}
