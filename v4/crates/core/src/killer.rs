//! Process management and termination. Implements safe process killing with strict, immutable OS-specific blocklists to prevent accidental termination of critical system services.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;
use sysinfo::{Pid, ProcessRefreshKind, Signal, System};

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

/// Result of a process kill attempt, including the target PID, name, and outcome.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillResult {
    pub pid: u32,
    pub process_name: String,
    pub killed: bool,
}

/// Errors that can occur when attempting to kill a process.
#[derive(Debug)]
pub enum KillError {
    /// The provided PID is invalid (e.g., <= 1).
    InvalidPid(i32),
    /// No process with this PID was found.
    ProcessNotFound(u32),
    /// The process is on the protected blocklist and cannot be killed.
    Blocked(String),
    /// The kill signal was sent but the process did not terminate.
    KillFailed(u32),
    /// PID is still in use but it is a different process (PID reuse).
    IdentityMismatch { pid: u32 },
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
            KillError::IdentityMismatch { pid } => {
                write!(
                    f,
                    "process identity changed for pid {pid} (possible PID reuse); kill aborted"
                )
            }
        }
    }
}

/// Snapshot of a live process used to decide whether a PID still names the
/// intended target. `start_time` is unix seconds from sysinfo, which maps to
/// macOS `kinfo_proc.p_starttime`, Linux `/proc/<pid>/stat` starttime, and
/// Windows `FILETIME` process creation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<PathBuf>,
    pub start_time: u64,
}

/// Caller-supplied identity for a kill. PID alone is not an identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KillIdentity {
    pub pid: u32,
    pub start_time: Option<u64>,
    pub name: Option<String>,
    pub exe_path: Option<PathBuf>,
}

/// Returns true when `live` is still the process the caller intended to kill.
pub fn identity_still_same(expected: &KillIdentity, live: &ProcessSnapshot) -> bool {
    if expected.pid != live.pid {
        return false;
    }
    if let Some(start) = expected.start_time {
        // 0 is not a process identity. Treating it as a wildcard would let a
        // recycled PID through. Callers that truly do not know start_time must
        // pass None (CLI/TUI only) rather than Some(0).
        if start == 0 || live.start_time != start {
            return false;
        }
    }
    if let Some(ref name) = expected.name {
        if !name.eq_ignore_ascii_case(&live.name) {
            return false;
        }
    }
    match (&expected.exe_path, &live.exe_path) {
        (Some(exp), Some(got)) => exp == got,
        (Some(_), None) => false,
        (None, _) => true,
    }
}

pub fn refuse_if_identity_changed(
    expected: &KillIdentity,
    live: Option<&ProcessSnapshot>,
) -> Result<(), KillError> {
    let Some(live) = live else {
        return Err(KillError::ProcessNotFound(expected.pid));
    };
    if !identity_still_same(expected, live) {
        return Err(KillError::IdentityMismatch { pid: expected.pid });
    }
    Ok(())
}

impl std::error::Error for KillError {}

/// Returns `true` if the process name matches a hardcoded OS-critical protected process.
///
/// Uses a lazily-initialized `HashSet` for O(1) lookups instead of linear scans,
/// since this function is called for every process on every watcher tick.
pub fn is_immutable_blocked_process_name(process_name: &str) -> bool {
    use std::collections::HashSet;
    use std::sync::OnceLock;

    static BLOCKED_SET: OnceLock<HashSet<&'static str>> = OnceLock::new();
    let blocked = BLOCKED_SET.get_or_init(|| {
        let mut set: HashSet<&'static str> = DEFAULT_PROTECTED_PROCESSES.iter().copied().collect();

        #[cfg(target_os = "macos")]
        set.extend(MACOS_PROTECTED_PROCESSES.iter().copied());

        #[cfg(target_os = "windows")]
        set.extend(WINDOWS_PROTECTED_PROCESSES.iter().copied());

        #[cfg(target_os = "linux")]
        set.extend(LINUX_PROTECTED_PROCESSES.iter().copied());

        set
    });

    let lowered_name = process_name.to_ascii_lowercase();
    blocked.contains(lowered_name.as_str())
}

fn path_is_trusted_for_blocked_process(exe_path: &Path) -> bool {
    // Normalize path separators for consistent comparison
    let path_normalized = exe_path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();

    if cfg!(target_os = "macos") {
        path_normalized.starts_with("/system/")
            || path_normalized.starts_with("/usr/libexec/")
            || path_normalized.starts_with("/usr/sbin/")
            || path_normalized == "/sbin/launchd"
    } else if cfg!(target_os = "windows") {
        // Normalized paths with forward slashes for comparison
        path_normalized.starts_with("c:/windows/system32/")
            || path_normalized.starts_with("c:/windows/syswow64/")
            || path_normalized == "c:/windows/explorer.exe"
    } else if cfg!(target_os = "linux") {
        path_normalized.starts_with("/sbin/")
            || path_normalized.starts_with("/usr/sbin/")
            || path_normalized.starts_with("/lib/systemd/")
            || path_normalized.starts_with("/usr/lib/systemd/")
            || path_normalized == "/usr/bin/xorg"
            || path_normalized == "/usr/lib/xorg/xorg"
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

/// Attempt to terminate a process by PID, respecting the protected-process blocklist.
///
/// Sends SIGTERM first, then escalates to a force kill if the process survives.
/// Returns an error if the PID is invalid, not found, blocked, or if the kill fails.
pub fn kill_process_safe(pid: i32, extra_blocklist: &[String]) -> Result<KillResult, KillError> {
    if pid <= 1 {
        return Err(KillError::InvalidPid(pid));
    }
    kill_process_identified(
        KillIdentity {
            pid: pid as u32,
            start_time: None,
            name: None,
            exe_path: None,
        },
        extra_blocklist,
    )
}

/// Kill only if the live process still matches `expected`.
///
/// When `start_time` is set, a recycled PID with a new creation time is
/// refused *before* any signal is sent (POSIX/Windows process-handle rule).
pub fn kill_process_identified(
    expected: KillIdentity,
    extra_blocklist: &[String],
) -> Result<KillResult, KillError> {
    if expected.pid <= 1 {
        return Err(KillError::InvalidPid(expected.pid as i32));
    }

    let pid_u32 = expected.pid;
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessRefreshKind::everything());

    let process_pid = Pid::from_u32(pid_u32);
    let live = system.process(process_pid).map(|process| ProcessSnapshot {
        pid: pid_u32,
        name: process.name().to_string(),
        exe_path: process.exe().map(|p| p.to_path_buf()),
        start_time: process.start_time(),
    });
    refuse_if_identity_changed(&expected, live.as_ref())?;
    let snapshot = live.expect("refuse_if_identity_changed checked Some");

    if is_blocked_process_name(
        &snapshot.name,
        snapshot.exe_path.as_deref(),
        extra_blocklist,
    ) {
        return Err(KillError::Blocked(snapshot.name));
    }

    {
        let process = system
            .process(process_pid)
            .ok_or(KillError::ProcessNotFound(pid_u32))?;
        let _ = process.kill_with(Signal::Term).unwrap_or(false) || process.kill();
    }

    std::thread::sleep(Duration::from_millis(300));

    let original_gone = |system: &mut System| {
        !process_is_alive(system, pid_u32)
            || !identity_matches(
                system,
                pid_u32,
                &snapshot.name,
                snapshot.exe_path.as_deref(),
                Some(snapshot.start_time),
            )
    };

    let killed = if original_gone(&mut system) {
        true
    } else if crate::os_native::kill_process_force(
        pid_u32,
        &snapshot.name,
        snapshot.exe_path.as_deref(),
    )
    .is_ok()
    {
        std::thread::sleep(Duration::from_millis(200));
        if original_gone(&mut system) {
            true
        } else {
            std::thread::sleep(Duration::from_millis(200));
            original_gone(&mut system)
        }
    } else {
        false
    };

    if !killed {
        return Err(KillError::KillFailed(pid_u32));
    }

    Ok(KillResult {
        pid: pid_u32,
        process_name: snapshot.name,
        killed,
    })
}

fn process_is_alive(system: &mut System, pid: u32) -> bool {
    system.refresh_processes_specifics(ProcessRefreshKind::new());
    system.process(Pid::from_u32(pid)).is_some()
}

fn identity_matches(
    system: &mut System,
    pid: u32,
    expected_name: &str,
    expected_exe: Option<&Path>,
    expected_start_time: Option<u64>,
) -> bool {
    system.refresh_processes_specifics(ProcessRefreshKind::everything());
    let Some(current) = system.process(Pid::from_u32(pid)) else {
        return false;
    };
    identity_still_same(
        &KillIdentity {
            pid,
            start_time: expected_start_time,
            name: Some(expected_name.to_string()),
            exe_path: expected_exe.map(Path::to_path_buf),
        },
        &ProcessSnapshot {
            pid,
            name: current.name().to_string(),
            exe_path: current.exe().map(|p| p.to_path_buf()),
            start_time: current.start_time(),
        },
    )
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
        let mut system = System::new();
        system.refresh_processes_specifics(ProcessRefreshKind::new());

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
    fn invalid_negative_pid_is_rejected() {
        let result = kill_process_safe(-42, &[]);
        assert!(matches!(result, Err(KillError::InvalidPid(-42))));
    }

    #[test]
    fn protected_name_without_trusted_path_is_not_immutable_blocked() {
        #[cfg(target_os = "macos")]
        let proc_name = "WindowServer";
        #[cfg(target_os = "windows")]
        let proc_name = "svchost.exe";
        #[cfg(target_os = "linux")]
        let proc_name = "systemd";

        assert!(!is_immutable_blocked_process(proc_name, None));
    }

    #[test]
    fn extra_blocklist_is_case_insensitive() {
        let extra = vec!["MyDaemon".to_string()];
        let result = kill_process_by_name(
            78,
            "mydaemon".to_string(),
            Some(Path::new("/opt/mydaemon")),
            &extra,
            || true,
        );
        assert!(matches!(result, Err(KillError::Blocked(name)) if name == "mydaemon"));
    }

    #[test]
    fn non_blocked_process_name_is_allowed_without_executable_path() {
        let result = kill_process_by_name(55, "user-app".to_string(), None, &[], || true);
        assert!(matches!(
            result,
            Ok(KillResult {
                pid: 55,
                process_name,
                killed: true,
            }) if process_name == "user-app"
        ));
    }

    #[test]
    fn display_for_blocked_error_mentions_protected_process() {
        let err = KillError::Blocked("system-service".to_string());
        assert!(err.to_string().contains("protected process"));
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
    fn display_messages_are_human_readable() {
        assert_eq!(KillError::InvalidPid(0).to_string(), "invalid pid: 0");
        assert_eq!(
            KillError::ProcessNotFound(42).to_string(),
            "process not found: 42"
        );
        assert_eq!(
            KillError::Blocked("launchd".to_string()).to_string(),
            "refusing to kill protected process: launchd"
        );
        assert_eq!(
            KillError::KillFailed(99).to_string(),
            "failed to kill process: 99"
        );
        assert!(KillError::IdentityMismatch { pid: 8 }
            .to_string()
            .contains("PID reuse"));
    }

    #[test]
    fn kill_process_safe_terminates_spawned_child() {
        let mut child = if cfg!(target_os = "windows") {
            std::process::Command::new("ping")
                .args(["-n", "30", "127.0.0.1"])
                .stdout(std::process::Stdio::null())
                .spawn()
                .expect("spawn long-running child process")
        } else {
            std::process::Command::new("sleep")
                .arg("30")
                .spawn()
                .expect("spawn long-running child process")
        };
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

    #[test]
    fn identity_matches_returns_false_for_missing_process() {
        let mut system = System::new();
        system.refresh_processes_specifics(ProcessRefreshKind::everything());

        let mut candidate: u32 = 600_000;
        while system.process(Pid::from_u32(candidate)).is_some() {
            candidate = candidate.saturating_add(1);
        }

        assert!(!identity_matches(
            &mut system,
            candidate,
            "missing",
            None,
            None
        ));
    }

    #[test]
    fn identity_matches_checks_name_and_executable() {
        let current_pid = std::process::id();
        let mut system = System::new();
        system.refresh_processes_specifics(ProcessRefreshKind::everything());
        let process = system
            .process(Pid::from_u32(current_pid))
            .expect("current process should exist");

        let current_name = process.name().to_string();
        let current_exe = process.exe().map(|p| p.to_path_buf());

        assert!(!identity_matches(
            &mut system,
            current_pid,
            "definitely-not-the-current-process",
            current_exe.as_deref(),
            None,
        ));

        assert!(identity_matches(
            &mut system,
            current_pid,
            &current_name,
            None,
            None
        ));

        let fake_exe = Path::new("/tmp/not-the-real-executable");
        assert!(!identity_matches(
            &mut system,
            current_pid,
            &current_name,
            Some(fake_exe),
            None,
        ));
    }

    fn snap(pid: u32, name: &str, start_time: u64, exe: Option<&str>) -> ProcessSnapshot {
        ProcessSnapshot {
            pid,
            name: name.to_string(),
            exe_path: exe.map(PathBuf::from),
            start_time,
        }
    }

    fn want(pid: u32, name: &str, start_time: Option<u64>, exe: Option<&str>) -> KillIdentity {
        KillIdentity {
            pid,
            start_time,
            name: Some(name.to_string()),
            exe_path: exe.map(PathBuf::from),
        }
    }

    #[test]
    fn same_pid_and_start_time_is_the_same_process() {
        let expected = want(100, "chrome", Some(1_700_000_000), Some("/opt/chrome"));
        let live = snap(100, "chrome", 1_700_000_000, Some("/opt/chrome"));
        assert!(identity_still_same(&expected, &live));
    }

    #[test]
    fn pid_reuse_with_new_start_time_is_not_the_same_process() {
        let expected = want(100, "chrome", Some(1_700_000_000), None);
        let live = snap(100, "chrome", 1_700_000_500, None);
        assert!(!identity_still_same(&expected, &live));
        assert!(matches!(
            refuse_if_identity_changed(&expected, Some(&live)),
            Err(KillError::IdentityMismatch { pid: 100 })
        ));
    }

    #[test]
    fn name_mismatch_is_not_the_same_process() {
        let expected = want(7, "node", Some(50), None);
        let live = snap(7, "python", 50, None);
        assert!(!identity_still_same(&expected, &live));
    }

    #[test]
    fn missing_live_process_is_not_found() {
        let expected = want(9, "node", Some(1), None);
        assert!(matches!(
            refuse_if_identity_changed(&expected, None),
            Err(KillError::ProcessNotFound(9))
        ));
    }

    #[test]
    fn unspecified_start_time_still_requires_name() {
        let expected = want(3, "sleep", None, None);
        let live = snap(3, "sleep", 99, None);
        assert!(identity_still_same(&expected, &live));
        let other = snap(3, "bash", 99, None);
        assert!(!identity_still_same(&expected, &other));
    }

    #[test]
    fn start_time_zero_is_not_an_identity() {
        let expected = want(100, "chrome", Some(0), None);
        let live = snap(100, "chrome", 1_700_000_000, None);
        assert!(!identity_still_same(&expected, &live));
        assert!(matches!(
            refuse_if_identity_changed(&expected, Some(&live)),
            Err(KillError::IdentityMismatch { pid: 100 })
        ));
    }
}
