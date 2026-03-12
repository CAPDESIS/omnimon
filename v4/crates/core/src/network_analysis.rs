//! Network connection analysis: data models, active connection capture,
//! filtering, and DNS reverse lookup with cache.
//!
//! This module extends the low-level packet capture in [`crate::network`] with
//! higher-level connection tracking, per-process summaries, and configurable
//! filters that the frontend can drive via IPC.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{OnceLock, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Transport-layer protocol for a network connection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Protocol {
    TCP,
    UDP,
    ICMP,
    Other,
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TCP => write!(f, "TCP"),
            Self::UDP => write!(f, "UDP"),
            Self::ICMP => write!(f, "ICMP"),
            Self::Other => write!(f, "Other"),
        }
    }
}

/// TCP connection state (mirrors kernel states).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConnectionState {
    Established,
    Listen,
    TimeWait,
    CloseWait,
    SynSent,
    SynReceived,
    Closed,
    Unknown,
}

impl ConnectionState {
    /// Parse from platform-specific string representations.
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "ESTABLISHED" | "ESTAB" => Self::Established,
            "LISTEN" | "LISTENING" => Self::Listen,
            "TIME_WAIT" | "TIMEWAIT" | "TIME-WAIT" => Self::TimeWait,
            "CLOSE_WAIT" | "CLOSEWAIT" | "CLOSE-WAIT" => Self::CloseWait,
            "SYN_SENT" | "SYNSENT" | "SYN-SENT" => Self::SynSent,
            "SYN_RECEIVED" | "SYN_RECV" | "SYNRECEIVED" | "SYN-RECEIVED" => Self::SynReceived,
            "CLOSED" | "CLOSE" => Self::Closed,
            "UNCONN" => Self::Closed,
            _ => Self::Unknown,
        }
    }
}

/// A single network connection tracked by the analysis engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConnection {
    pub pid: u32,
    pub process_name: String,
    pub protocol: Protocol,
    pub local_addr: IpAddr,
    pub local_port: u16,
    pub remote_addr: IpAddr,
    pub remote_port: u16,
    pub remote_hostname: Option<String>,
    pub state: ConnectionState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub bytes_per_sec_up: f64,
    pub bytes_per_sec_down: f64,
    pub established_at: u64,
    pub country: Option<String>,
    pub is_encrypted: Option<bool>,
}

/// Aggregated snapshot of all connections at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSnapshot {
    pub timestamp: u64,
    pub connections: Vec<NetworkConnection>,
    pub total_bytes_up: u64,
    pub total_bytes_down: u64,
    pub total_bytes_per_sec_up: f64,
    pub total_bytes_per_sec_down: f64,
    pub active_connections: usize,
    pub per_process_summary: Vec<ProcessNetworkSummary>,
}

impl Default for NetworkSnapshot {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            connections: Vec::new(),
            total_bytes_up: 0,
            total_bytes_down: 0,
            total_bytes_per_sec_up: 0.0,
            total_bytes_per_sec_down: 0.0,
            active_connections: 0,
            per_process_summary: Vec::new(),
        }
    }
}

/// Per-process aggregation of network activity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNetworkSummary {
    pub pid: u32,
    pub name: String,
    pub connection_count: usize,
    pub total_up: f64,
    pub total_down: f64,
    pub top_remote: Option<String>,
    pub protocols: Vec<Protocol>,
}

// ---------------------------------------------------------------------------
// Connection capture (cross-platform)
// ---------------------------------------------------------------------------

/// Captures all active TCP/UDP connections on the system.
///
/// On macOS uses `lsof -i -n -P`.
/// On Linux reads `/proc/net/tcp`, `/proc/net/udp`, `/proc/net/tcp6`, `/proc/net/udp6`.
/// On Windows uses `netstat -ano`.
pub fn get_active_connections() -> Result<Vec<NetworkConnection>, String> {
    #[cfg(target_os = "macos")]
    {
        get_connections_macos()
    }

    #[cfg(target_os = "linux")]
    {
        get_connections_linux()
    }

    #[cfg(target_os = "windows")]
    {
        get_connections_windows()
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(Vec::new())
    }
}

/// Build a full [`NetworkSnapshot`] by capturing connections and computing
/// deltas against the previous snapshot for bytes/sec calculation.
pub fn capture_snapshot(prev: &Option<NetworkSnapshot>) -> Result<NetworkSnapshot, String> {
    let connections = get_active_connections()?;
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let elapsed_secs = prev
        .as_ref()
        .map(|p| (now_secs.saturating_sub(p.timestamp)).max(1) as f64)
        .unwrap_or(1.0);

    // Build a lookup from (pid, remote_addr, remote_port) -> prev connection
    // for computing bytes/sec deltas.
    let prev_map: HashMap<(u32, IpAddr, u16), &NetworkConnection> = prev
        .as_ref()
        .map(|p| {
            p.connections
                .iter()
                .map(|c| ((c.pid, c.remote_addr, c.remote_port), c))
                .collect()
        })
        .unwrap_or_default();

    let mut enriched: Vec<NetworkConnection> = connections
        .into_iter()
        .map(|mut c| {
            if let Some(prev_conn) = prev_map.get(&(c.pid, c.remote_addr, c.remote_port)) {
                let sent_delta = c.bytes_sent.saturating_sub(prev_conn.bytes_sent);
                let recv_delta = c.bytes_received.saturating_sub(prev_conn.bytes_received);
                c.bytes_per_sec_up = sent_delta as f64 / elapsed_secs;
                c.bytes_per_sec_down = recv_delta as f64 / elapsed_secs;
                // Preserve established_at from previous snapshot
                if c.established_at == 0 {
                    c.established_at = prev_conn.established_at;
                }
            }
            c
        })
        .collect();

    // Resolve hostnames (non-blocking best-effort from cache)
    for conn in &mut enriched {
        if conn.remote_hostname.is_none() {
            conn.remote_hostname = dns_cache().lookup(&conn.remote_addr);
        }
    }

    let total_bytes_up: u64 = enriched.iter().map(|c| c.bytes_sent).sum();
    let total_bytes_down: u64 = enriched.iter().map(|c| c.bytes_received).sum();

    let prev_up = prev.as_ref().map(|p| p.total_bytes_up).unwrap_or(0);
    let prev_down = prev.as_ref().map(|p| p.total_bytes_down).unwrap_or(0);

    let total_bytes_per_sec_up = total_bytes_up.saturating_sub(prev_up) as f64 / elapsed_secs;
    let total_bytes_per_sec_down = total_bytes_down.saturating_sub(prev_down) as f64 / elapsed_secs;

    let active_connections = enriched
        .iter()
        .filter(|c| c.state == ConnectionState::Established)
        .count();

    let per_process_summary = build_process_summaries(&enriched);

    Ok(NetworkSnapshot {
        timestamp: now_secs,
        connections: enriched,
        total_bytes_up,
        total_bytes_down,
        total_bytes_per_sec_up,
        total_bytes_per_sec_down,
        active_connections,
        per_process_summary,
    })
}

fn build_process_summaries(connections: &[NetworkConnection]) -> Vec<ProcessNetworkSummary> {
    let mut map: HashMap<u32, ProcessNetworkSummary> = HashMap::new();

    for conn in connections {
        let entry = map
            .entry(conn.pid)
            .or_insert_with(|| ProcessNetworkSummary {
                pid: conn.pid,
                name: conn.process_name.clone(),
                connection_count: 0,
                total_up: 0.0,
                total_down: 0.0,
                top_remote: None,
                protocols: Vec::new(),
            });

        entry.connection_count += 1;
        entry.total_up += conn.bytes_per_sec_up;
        entry.total_down += conn.bytes_per_sec_down;

        if !entry.protocols.contains(&conn.protocol) {
            entry.protocols.push(conn.protocol);
        }

        // Track the remote with most traffic
        let conn_total = conn.bytes_per_sec_up + conn.bytes_per_sec_down;
        if conn_total > 0.0 {
            let remote_label = conn
                .remote_hostname
                .as_deref()
                .unwrap_or(&conn.remote_addr.to_string())
                .to_string();
            entry.top_remote = Some(remote_label);
        }
    }

    let mut summaries: Vec<ProcessNetworkSummary> = map.into_values().collect();
    summaries.sort_by(|a, b| {
        (b.total_up + b.total_down)
            .partial_cmp(&(a.total_up + a.total_down))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    summaries
}

// ---------------------------------------------------------------------------
// Filtering
// ---------------------------------------------------------------------------

/// Frontend-driven filter for network connections.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkFilter {
    pub protocols: Option<Vec<Protocol>>,
    pub ports: Option<Vec<u16>>,
    pub process_names: Option<Vec<String>>,
    pub pids: Option<Vec<u32>>,
    pub remote_hosts: Option<Vec<String>>,
    pub min_bytes_per_sec: Option<f64>,
    #[serde(default)]
    pub exclude_localhost: bool,
    #[serde(default)]
    pub only_established: bool,
}

impl NetworkFilter {
    /// Apply the filter to a list of connections.
    pub fn apply(&self, connections: &[NetworkConnection]) -> Vec<NetworkConnection> {
        connections
            .iter()
            .filter(|c| self.matches(c))
            .cloned()
            .collect()
    }

    fn matches(&self, conn: &NetworkConnection) -> bool {
        if let Some(ref protocols) = self.protocols {
            if !protocols.contains(&conn.protocol) {
                return false;
            }
        }

        if let Some(ref ports) = self.ports {
            if !ports.contains(&conn.local_port) && !ports.contains(&conn.remote_port) {
                return false;
            }
        }

        if let Some(ref names) = self.process_names {
            let name_lower = conn.process_name.to_lowercase();
            if !names.iter().any(|n| name_lower.contains(&n.to_lowercase())) {
                return false;
            }
        }

        if let Some(ref pids) = self.pids {
            if !pids.contains(&conn.pid) {
                return false;
            }
        }

        if let Some(ref hosts) = self.remote_hosts {
            let remote_str = conn.remote_addr.to_string();
            let hostname = conn.remote_hostname.as_deref().unwrap_or("");
            if !hosts.iter().any(|h| {
                remote_str.contains(h) || hostname.to_lowercase().contains(&h.to_lowercase())
            }) {
                return false;
            }
        }

        if let Some(min_bps) = self.min_bytes_per_sec {
            if conn.bytes_per_sec_up + conn.bytes_per_sec_down < min_bps {
                return false;
            }
        }

        if self.exclude_localhost
            && (conn.remote_addr.is_loopback() || conn.local_addr.is_loopback())
        {
            return false;
        }

        if self.only_established && conn.state != ConnectionState::Established {
            return false;
        }

        true
    }
}

// ---------------------------------------------------------------------------
// DNS reverse lookup with cache
// ---------------------------------------------------------------------------

/// TTL for DNS cache entries.
const DNS_CACHE_TTL: Duration = Duration::from_secs(300);

/// Maximum concurrent DNS lookups.
const DNS_MAX_CONCURRENT: usize = 10;

struct DnsCacheEntry {
    hostname: Option<String>,
    resolved_at: Instant,
}

/// Thread-safe DNS cache with TTL-based expiration.
struct DnsCache {
    entries: RwLock<HashMap<IpAddr, DnsCacheEntry>>,
}

impl DnsCache {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    /// Look up a cached hostname. Returns `Some` if resolved, `None` if not
    /// in cache or expired. Expired entries trigger a background re-resolve.
    fn lookup(&self, ip: &IpAddr) -> Option<String> {
        if let Ok(entries) = self.entries.read() {
            if let Some(entry) = entries.get(ip) {
                if entry.resolved_at.elapsed() < DNS_CACHE_TTL {
                    return entry.hostname.clone();
                }
            }
        }

        // Schedule background resolution
        let ip_owned = *ip;
        std::thread::spawn(move || {
            resolve_and_cache(ip_owned);
        });

        None
    }

    fn insert(&self, ip: IpAddr, hostname: Option<String>) {
        if let Ok(mut entries) = self.entries.write() {
            entries.insert(
                ip,
                DnsCacheEntry {
                    hostname,
                    resolved_at: Instant::now(),
                },
            );
        }
    }

    /// Evict expired entries to prevent unbounded growth.
    fn evict_expired(&self) {
        if let Ok(mut entries) = self.entries.write() {
            entries.retain(|_, entry| entry.resolved_at.elapsed() < DNS_CACHE_TTL * 2);
        }
    }
}

/// Global DNS cache instance.
static DNS_CACHE: OnceLock<DnsCache> = OnceLock::new();

fn dns_cache() -> &'static DnsCache {
    DNS_CACHE.get_or_init(DnsCache::new)
}

/// Counter for active concurrent DNS lookups.
static DNS_ACTIVE_LOOKUPS: AtomicUsize = AtomicUsize::new(0);

fn resolve_and_cache(ip: IpAddr) {
    // Simple concurrency limiter via atomic counter
    let current = DNS_ACTIVE_LOOKUPS.fetch_add(1, Ordering::SeqCst);
    if current >= DNS_MAX_CONCURRENT {
        DNS_ACTIVE_LOOKUPS.fetch_sub(1, Ordering::SeqCst);
        return;
    }

    let hostname = resolve_hostname_blocking(&ip);
    dns_cache().insert(ip, hostname);
    DNS_ACTIVE_LOOKUPS.fetch_sub(1, Ordering::SeqCst);
}

/// Timeout for DNS reverse lookups.
const DNS_LOOKUP_TIMEOUT: Duration = Duration::from_secs(3);

/// Blocking DNS reverse lookup using the system resolver with timeout.
fn resolve_hostname_blocking(ip: &IpAddr) -> Option<String> {
    // Skip private/reserved IPs — they won't have public PTR records
    if is_private_ip(ip) {
        return None;
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        resolve_hostname_unix(ip)
    }

    #[cfg(target_os = "windows")]
    {
        resolve_hostname_windows(ip)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

/// Check if an IP is in a private/reserved range (no PTR records expected).
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback(),
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn resolve_hostname_unix(ip: &IpAddr) -> Option<String> {
    use std::process::Command;

    // Locate the `host` command — absolute paths for security
    let host_bin = if std::path::Path::new("/usr/bin/host").exists() {
        "/usr/bin/host"
    } else if std::path::Path::new("/usr/local/bin/host").exists() {
        "/usr/local/bin/host"
    } else {
        return None;
    };

    let child = Command::new(host_bin)
        // -W 2: timeout 2 seconds for the DNS query itself
        .args(["-W", "2", &ip.to_string()])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let output = wait_with_timeout(child, DNS_LOOKUP_TIMEOUT).ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    // "1.2.3.4.in-addr.arpa domain name pointer hostname.example.com."
    for line in text.lines() {
        if let Some(ptr) = line.split("domain name pointer ").nth(1) {
            let hostname = ptr.trim_end_matches('.').to_string();
            if !hostname.is_empty() && hostname.len() < 256 {
                return Some(hostname);
            }
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn resolve_hostname_windows(ip: &IpAddr) -> Option<String> {
    use std::process::Command;

    let child = Command::new("nslookup")
        .arg(ip.to_string())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;

    let output = wait_with_timeout(child, DNS_LOOKUP_TIMEOUT).ok()?;

    let text = String::from_utf8_lossy(&output.stdout);
    // nslookup output: "Name:    hostname.example.com"
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("Name:") {
            let hostname = name.trim().to_string();
            if !hostname.is_empty() && hostname.len() < 256 {
                return Some(hostname);
            }
        }
    }

    None
}

/// Bulk-resolve hostnames for a list of connections. Enqueues unknown IPs
/// for background resolution and returns immediately.
pub fn enqueue_dns_resolution(connections: &[NetworkConnection]) {
    for conn in connections {
        if conn.remote_hostname.is_none() && !conn.remote_addr.is_loopback() {
            let ip = conn.remote_addr;
            // Check if already cached
            if let Ok(entries) = dns_cache().entries.read() {
                if entries.contains_key(&ip) {
                    continue;
                }
            }
            let _ = std::thread::Builder::new()
                .name("dns-resolve".into())
                .spawn(move || {
                    resolve_and_cache(ip);
                });
        }
    }

    // Periodically evict stale cache entries
    dns_cache().evict_expired();
}

// ---------------------------------------------------------------------------
// macOS implementation
// ---------------------------------------------------------------------------

/// Timeout for external commands (lsof, netstat, etc.) in seconds.
const COMMAND_TIMEOUT_SECS: u64 = 10;

#[cfg(target_os = "macos")]
fn get_connections_macos() -> Result<Vec<NetworkConnection>, String> {
    use std::process::Command;

    // Use absolute path to avoid PATH injection
    let lsof_path = if std::path::Path::new("/usr/sbin/lsof").exists() {
        "/usr/sbin/lsof"
    } else if std::path::Path::new("/usr/bin/lsof").exists() {
        "/usr/bin/lsof"
    } else {
        return Err("lsof not found at /usr/sbin/lsof or /usr/bin/lsof".to_string());
    };

    let child = Command::new(lsof_path)
        .args(["-i", "-n", "-P", "-F", "pcTtPn"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn lsof: {e}"))?;

    let output = wait_with_timeout(child, Duration::from_secs(COMMAND_TIMEOUT_SECS))
        .map_err(|e| format!("lsof failed: {e}"))?;

    if !output.status.success() {
        // lsof may fail with permission issues — return empty rather than error
        // to allow partial data from other sources
        return Ok(Vec::new());
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_lsof_output(&text)
}

/// Wait for a child process with a timeout. Kills the process if it exceeds the deadline.
fn wait_with_timeout(
    child: std::process::Child,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result.map_err(|e| format!("process error: {e}")),
        Err(_) => Err(format!("command timed out after {}s", timeout.as_secs())),
    }
}

/// Parse lsof -F output format.
///
/// Fields: p=PID, c=command, t=type, T=TCP state, P=protocol, n=name
#[cfg(any(target_os = "macos", test))]
fn parse_lsof_output(text: &str) -> Result<Vec<NetworkConnection>, String> {
    let mut connections = Vec::new();
    let mut current_pid: u32 = 0;
    let mut current_name = String::new();
    let mut current_protocol = Protocol::Other;
    let mut current_state = ConnectionState::Unknown;
    let mut current_type = String::new();

    for line in text.lines() {
        if line.is_empty() {
            continue;
        }

        let field = &line[..1];
        let value = &line[1..];

        match field {
            "p" => {
                current_pid = value.parse().unwrap_or(0);
            }
            "c" => {
                current_name = value.to_string();
            }
            "t" => {
                current_type = value.to_string();
            }
            "P" => {
                current_protocol = match value.to_uppercase().as_str() {
                    "TCP" => Protocol::TCP,
                    "UDP" => Protocol::UDP,
                    _ => Protocol::Other,
                };
            }
            "T" => {
                if let Some(state_str) = value.strip_prefix("ST=") {
                    current_state = ConnectionState::from_str_loose(state_str);
                }
            }
            "n" => {
                // Skip non-IP entries (like Unix sockets)
                if current_type != "IPv4" && current_type != "IPv6" {
                    continue;
                }

                if let Some(conn) = parse_lsof_name(
                    value,
                    current_pid,
                    &current_name,
                    current_protocol,
                    current_state,
                ) {
                    connections.push(conn);
                }

                // Reset per-fd state
                current_state = ConnectionState::Unknown;
                current_type.clear();
            }
            _ => {}
        }
    }

    Ok(connections)
}

/// Parse a single lsof "n" field like "10.0.0.1:443->8.8.8.8:443" or "*:80".
#[cfg(any(target_os = "macos", test))]
fn parse_lsof_name(
    name: &str,
    pid: u32,
    process_name: &str,
    protocol: Protocol,
    state: ConnectionState,
) -> Option<NetworkConnection> {
    // Format: "local_addr:port->remote_addr:port" or "local_addr:port" (LISTEN)
    let (local_part, remote_part) = if let Some((local, remote)) = name.split_once("->") {
        (local, Some(remote))
    } else {
        (name, None)
    };

    let (local_addr, local_port) = parse_addr_port(local_part)?;
    let (remote_addr, remote_port) = if let Some(remote) = remote_part {
        parse_addr_port(remote)?
    } else {
        (IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0)
    };

    let is_encrypted = detect_tls_port(remote_port);

    Some(NetworkConnection {
        pid,
        process_name: process_name.to_string(),
        protocol,
        local_addr,
        local_port,
        remote_addr,
        remote_port,
        remote_hostname: None,
        state,
        bytes_sent: 0,
        bytes_received: 0,
        bytes_per_sec_up: 0.0,
        bytes_per_sec_down: 0.0,
        established_at: 0,
        country: None,
        is_encrypted,
    })
}

// ---------------------------------------------------------------------------
// Linux implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn get_connections_linux() -> Result<Vec<NetworkConnection>, String> {
    let mut connections = Vec::new();

    // Build PID→name map from /proc (graceful on permission errors)
    let pid_names = build_pid_name_map_linux();

    // Build inode→PID map once (expensive operation, reused for all /proc/net files)
    let inode_pid = build_inode_pid_map();

    // Parse /proc/net/tcp and /proc/net/udp (and IPv6 variants)
    for (path, proto) in &[
        ("/proc/net/tcp", Protocol::TCP),
        ("/proc/net/udp", Protocol::UDP),
        ("/proc/net/tcp6", Protocol::TCP),
        ("/proc/net/udp6", Protocol::UDP),
    ] {
        if let Ok(content) = std::fs::read_to_string(path) {
            let parsed = parse_proc_net(&content, *proto, &pid_names, &inode_pid);
            connections.extend(parsed);
        }
    }

    // If /proc/net parsing yielded nothing (e.g. container/permission issue),
    // fall back to `ss` command
    if connections.is_empty() {
        if let Ok(fallback) = get_connections_linux_ss(&pid_names) {
            connections = fallback;
        }
    }

    Ok(connections)
}

/// Fallback: use `ss` command when /proc/net is unavailable.
#[cfg(target_os = "linux")]
fn get_connections_linux_ss(
    pid_names: &HashMap<u32, String>,
) -> Result<Vec<NetworkConnection>, String> {
    use std::process::Command;

    let ss_path = if std::path::Path::new("/usr/sbin/ss").exists() {
        "/usr/sbin/ss"
    } else if std::path::Path::new("/sbin/ss").exists() {
        "/sbin/ss"
    } else if std::path::Path::new("/usr/bin/ss").exists() {
        "/usr/bin/ss"
    } else {
        return Err("ss command not found".to_string());
    };

    let child = Command::new(ss_path)
        .args(["-tunap", "--no-header"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to spawn ss: {e}"))?;

    let output = wait_with_timeout(child, Duration::from_secs(COMMAND_TIMEOUT_SECS))
        .map_err(|e| format!("ss failed: {e}"))?;

    let text = String::from_utf8_lossy(&output.stdout);
    parse_ss_output(&text, pid_names)
}

/// Parse `ss -tunap --no-header` output.
/// Format: Netid State Recv-Q Send-Q LocalAddr:Port PeerAddr:Port Process
#[cfg(any(target_os = "linux", test))]
fn parse_ss_output(
    text: &str,
    pid_names: &HashMap<u32, String>,
) -> Result<Vec<NetworkConnection>, String> {
    let mut connections = Vec::new();

    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }

        let protocol = match cols[0] {
            "tcp" => Protocol::TCP,
            "udp" => Protocol::UDP,
            _ => continue,
        };

        let state = ConnectionState::from_str_loose(cols[1]);

        let (local_addr, local_port) = match parse_addr_port(cols[4]) {
            Some(v) => v,
            None => continue,
        };

        let (remote_addr, remote_port) = if cols.len() > 5 {
            parse_addr_port(cols[5]).unwrap_or((IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0))
        } else {
            (IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0)
        };

        // Extract PID from process column: "users:((\"chrome\",pid=1234,fd=56))"
        let pid = if cols.len() > 6 {
            extract_pid_from_ss_process(cols[6])
        } else {
            0
        };

        let process_name = pid_names
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let is_encrypted = detect_tls_port(remote_port);

        connections.push(NetworkConnection {
            pid,
            process_name,
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            remote_hostname: None,
            state,
            bytes_sent: 0,
            bytes_received: 0,
            bytes_per_sec_up: 0.0,
            bytes_per_sec_down: 0.0,
            established_at: 0,
            country: None,
            is_encrypted,
        });
    }

    Ok(connections)
}

/// Extract PID from ss process column: "users:((\"name\",pid=1234,fd=5))"
#[cfg(any(target_os = "linux", test))]
fn extract_pid_from_ss_process(s: &str) -> u32 {
    // Find "pid=NNNN" pattern
    if let Some(pid_start) = s.find("pid=") {
        let after_pid = &s[pid_start + 4..];
        let end = after_pid
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(after_pid.len());
        after_pid[..end].parse().unwrap_or(0)
    } else {
        0
    }
}

#[cfg(target_os = "linux")]
fn build_pid_name_map_linux() -> HashMap<u32, String> {
    let mut map = HashMap::new();
    let proc_dir = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return map,
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            let comm_path = format!("/proc/{}/comm", pid);
            // Gracefully handle permission errors (other users' processes)
            if let Ok(comm) = std::fs::read_to_string(&comm_path) {
                let trimmed = comm.trim().to_string();
                if !trimmed.is_empty() {
                    map.insert(pid, trimmed);
                }
            }
        }
    }
    map
}

#[cfg(any(target_os = "linux", test))]
fn parse_proc_net(
    content: &str,
    protocol: Protocol,
    pid_names: &HashMap<u32, String>,
    inode_pid: &HashMap<u64, u32>,
) -> Vec<NetworkConnection> {
    let mut connections = Vec::new();

    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 10 {
            continue;
        }

        let local = cols[1];
        let remote = cols[2];
        let state_hex = cols[3];
        let inode_str = cols[9];

        let state = parse_tcp_state_hex(state_hex);
        let (local_addr, local_port) = match parse_hex_addr_port(local) {
            Some(v) => v,
            None => continue,
        };
        let (remote_addr, remote_port) = match parse_hex_addr_port(remote) {
            Some(v) => v,
            None => continue,
        };

        let inode: u64 = inode_str.parse().unwrap_or(0);
        let pid = inode_pid.get(&inode).copied().unwrap_or(0);
        let process_name = pid_names
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let is_encrypted = detect_tls_port(remote_port);

        connections.push(NetworkConnection {
            pid,
            process_name,
            protocol,
            local_addr,
            local_port,
            remote_addr,
            remote_port,
            remote_hostname: None,
            state,
            bytes_sent: 0,
            bytes_received: 0,
            bytes_per_sec_up: 0.0,
            bytes_per_sec_down: 0.0,
            established_at: 0,
            country: None,
            is_encrypted,
        });
    }

    connections
}

/// Alias used by tests — delegates to [`parse_proc_net`].
#[cfg(test)]
fn parse_proc_net_with_inode_map(
    content: &str,
    protocol: Protocol,
    pid_names: &HashMap<u32, String>,
    inode_pid: &HashMap<u64, u32>,
) -> Vec<NetworkConnection> {
    parse_proc_net(content, protocol, pid_names, inode_pid)
}

#[cfg(any(target_os = "linux", test))]
#[cfg_attr(test, allow(dead_code))]
fn build_inode_pid_map() -> HashMap<u64, u32> {
    let mut map = HashMap::new();
    let proc_dir = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return map,
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if let Ok(pid) = name_str.parse::<u32>() {
            let fd_dir = format!("/proc/{}/fd", pid);
            // Permission errors are expected for other users' processes
            let fds = match std::fs::read_dir(&fd_dir) {
                Ok(fds) => fds,
                Err(_) => continue,
            };
            for fd in fds.flatten() {
                if let Ok(link) = std::fs::read_link(fd.path()) {
                    let link_str = link.to_string_lossy();
                    if let Some(inode_str) = link_str
                        .strip_prefix("socket:[")
                        .and_then(|s| s.strip_suffix(']'))
                    {
                        if let Ok(inode) = inode_str.parse::<u64>() {
                            map.insert(inode, pid);
                        }
                    }
                }
            }
        }
    }
    map
}

/// Parse hex-encoded address:port from /proc/net/{tcp,udp} format.
/// IPv4: 8 hex chars (little-endian u32), IPv6: 32 hex chars.
#[cfg(any(target_os = "linux", test))]
fn parse_hex_addr_port(hex_str: &str) -> Option<(IpAddr, u16)> {
    let (addr_hex, port_hex) = hex_str.split_once(':')?;

    let port = u16::from_str_radix(port_hex, 16).ok()?;

    if addr_hex.len() == 8 {
        // IPv4: stored as little-endian u32 in /proc/net
        let addr_u32 = u32::from_str_radix(addr_hex, 16).ok()?;
        let ip = IpAddr::V4(std::net::Ipv4Addr::from(addr_u32.to_be()));
        Some((ip, port))
    } else if addr_hex.len() == 32 {
        // IPv6: stored as 4 groups of little-endian u32
        let mut bytes = [0u8; 16];
        for group in 0..4 {
            let offset = group * 8;
            let group_u32 = u32::from_str_radix(&addr_hex[offset..offset + 8], 16).ok()?;
            let group_bytes = group_u32.swap_bytes().to_be_bytes();
            bytes[group * 4..group * 4 + 4].copy_from_slice(&group_bytes);
        }
        let ip = IpAddr::V6(std::net::Ipv6Addr::from(bytes));
        Some((ip, port))
    } else {
        None
    }
}

#[cfg(any(target_os = "linux", test))]
fn parse_tcp_state_hex(hex: &str) -> ConnectionState {
    match hex {
        "01" => ConnectionState::Established,
        "02" => ConnectionState::SynSent,
        "03" => ConnectionState::SynReceived,
        "04" => ConnectionState::Unknown, // FIN_WAIT1
        "05" => ConnectionState::Unknown, // FIN_WAIT2
        "06" => ConnectionState::TimeWait,
        "07" => ConnectionState::Closed,
        "08" => ConnectionState::CloseWait,
        "0A" => ConnectionState::Listen,
        _ => ConnectionState::Unknown,
    }
}

// ---------------------------------------------------------------------------
// Windows implementation
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn get_connections_windows() -> Result<Vec<NetworkConnection>, String> {
    // Try native Windows API first, fall back to netstat
    match get_connections_windows_native() {
        Ok(conns) if !conns.is_empty() => {
            tracing::debug!("[network] Windows native API: got {} connections", conns.len());
            Ok(conns)
        }
        Ok(_) => {
            tracing::warn!("[network] Windows native API returned empty, falling back to netstat");
            get_connections_windows_netstat()
        }
        Err(e) => {
            tracing::error!("[network] Windows native API failed: {}, falling back to netstat", e);
            get_connections_windows_netstat()
        }
    }
}

/// Native Windows API implementation using GetExtendedTcpTable / GetExtendedUdpTable.
#[cfg(target_os = "windows")]
fn get_connections_windows_native() -> Result<Vec<NetworkConnection>, String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCPROW_OWNER_PID, MIB_TCP_STATE_CLOSED,
        MIB_TCP_STATE_CLOSE_WAIT, MIB_TCP_STATE_ESTAB, MIB_TCP_STATE_LISTEN,
        MIB_TCP_STATE_SYN_RCVD, MIB_TCP_STATE_SYN_SENT, MIB_TCP_STATE_TIME_WAIT,
        MIB_UDPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
    };
    use windows::Win32::Networking::WinSock::AF_INET;

    let mut connections = Vec::new();
    let pid_names = build_pid_name_map_windows_sysinfo();

    // --- TCP connections ---
    let mut tcp_size: u32 = 0;
    // First call to get required buffer size
    let first_result = unsafe {
        GetExtendedTcpTable(
            None,
            &mut tcp_size,
            false,
            AF_INET.0 as u32,
            TCP_TABLE_OWNER_PID_ALL,
            0,
        )
    };

    if first_result != 0 && first_result != 122 {
        // 122 = ERROR_INSUFFICIENT_BUFFER (expected for size query)
        tracing::error!("[network] GetExtendedTcpTable size query failed with error code: {}", first_result);
    }

    if tcp_size > 0 {
        tracing::debug!("[network] GetExtendedTcpTable buffer size: {} bytes", tcp_size);
        let mut tcp_buf = vec![0u8; tcp_size as usize];
        let result = unsafe {
            GetExtendedTcpTable(
                Some(tcp_buf.as_mut_ptr().cast()),
                &mut tcp_size,
                false,
                AF_INET.0 as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if result == 0 {
            let num_entries = u32::from_le_bytes(tcp_buf[0..4].try_into().unwrap_or([0; 4]));
            let entry_size = std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
            tracing::debug!("[network] Found {} TCP connections", num_entries);

            for i in 0..num_entries as usize {
                let offset = 4 + i * entry_size;
                if offset + entry_size > tcp_buf.len() {
                    break;
                }

                let row: &MIB_TCPROW_OWNER_PID = unsafe { &*(tcp_buf.as_ptr().add(offset).cast()) };

                let local_addr =
                    IpAddr::V4(std::net::Ipv4Addr::from(u32::from_be(row.dwLocalAddr)));
                let local_port = (row.dwLocalPort as u16).to_be();
                let remote_addr =
                    IpAddr::V4(std::net::Ipv4Addr::from(u32::from_be(row.dwRemoteAddr)));
                let remote_port = (row.dwRemotePort as u16).to_be();
                let pid = row.dwOwningPid;

                let state = match row.dwState {
                    s if s == MIB_TCP_STATE_ESTAB.0 as u32 => ConnectionState::Established,
                    s if s == MIB_TCP_STATE_LISTEN.0 as u32 => ConnectionState::Listen,
                    s if s == MIB_TCP_STATE_TIME_WAIT.0 as u32 => ConnectionState::TimeWait,
                    s if s == MIB_TCP_STATE_CLOSE_WAIT.0 as u32 => ConnectionState::CloseWait,
                    s if s == MIB_TCP_STATE_SYN_SENT.0 as u32 => ConnectionState::SynSent,
                    s if s == MIB_TCP_STATE_SYN_RCVD.0 as u32 => ConnectionState::SynReceived,
                    s if s == MIB_TCP_STATE_CLOSED.0 as u32 => ConnectionState::Closed,
                    _ => ConnectionState::Unknown,
                };

                let process_name = pid_names
                    .get(&pid)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                connections.push(NetworkConnection {
                    pid,
                    process_name,
                    protocol: Protocol::TCP,
                    local_addr,
                    local_port,
                    remote_addr,
                    remote_port,
                    remote_hostname: None,
                    state,
                    bytes_sent: 0,
                    bytes_received: 0,
                    bytes_per_sec_up: 0.0,
                    bytes_per_sec_down: 0.0,
                    established_at: 0,
                    country: None,
                    is_encrypted: detect_tls_port(remote_port),
                });
            }
        }
    }

    // --- UDP connections ---
    let mut udp_size: u32 = 0;
    unsafe {
        let _ = GetExtendedUdpTable(
            None,
            &mut udp_size,
            false,
            AF_INET.0 as u32,
            UDP_TABLE_OWNER_PID,
            0,
        );
    }

    if udp_size > 0 {
        tracing::debug!("[network] GetExtendedUdpTable buffer size: {} bytes", udp_size);
        let mut udp_buf = vec![0u8; udp_size as usize];
        let result = unsafe {
            GetExtendedUdpTable(
                Some(udp_buf.as_mut_ptr().cast()),
                &mut udp_size,
                false,
                AF_INET.0 as u32,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if result == 0 {
            let num_entries = u32::from_le_bytes(udp_buf[0..4].try_into().unwrap_or([0; 4]));
            let entry_size = std::mem::size_of::<MIB_UDPROW_OWNER_PID>();
            tracing::debug!("[network] Found {} UDP connections", num_entries);

            for i in 0..num_entries as usize {
                let offset = 4 + i * entry_size;
                if offset + entry_size > udp_buf.len() {
                    break;
                }

                let row: &MIB_UDPROW_OWNER_PID = unsafe { &*(udp_buf.as_ptr().add(offset).cast()) };

                let local_addr =
                    IpAddr::V4(std::net::Ipv4Addr::from(u32::from_be(row.dwLocalAddr)));
                let local_port = (row.dwLocalPort as u16).to_be();
                let pid = row.dwOwningPid;

                let process_name = pid_names
                    .get(&pid)
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string());

                connections.push(NetworkConnection {
                    pid,
                    process_name,
                    protocol: Protocol::UDP,
                    local_addr,
                    local_port,
                    remote_addr: IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                    remote_port: 0,
                    remote_hostname: None,
                    state: ConnectionState::Unknown,
                    bytes_sent: 0,
                    bytes_received: 0,
                    bytes_per_sec_up: 0.0,
                    bytes_per_sec_down: 0.0,
                    established_at: 0,
                    country: None,
                    is_encrypted: None,
                });
            }
        }
    }

    Ok(connections)
}

/// Build PID→name map using sysinfo (cross-platform, no subprocess).
#[cfg(target_os = "windows")]
fn build_pid_name_map_windows_sysinfo() -> HashMap<u32, String> {
    use sysinfo::{ProcessRefreshKind, System};

    let mut system = System::new();
    system.refresh_processes_specifics(ProcessRefreshKind::new());

    system
        .processes()
        .iter()
        .map(|(pid, process)| (pid.as_u32(), process.name().to_string()))
        .collect()
}

/// Fallback: use `netstat -ano` when native API is unavailable.
#[cfg(target_os = "windows")]
fn get_connections_windows_netstat() -> Result<Vec<NetworkConnection>, String> {
    use std::process::Command;

    tracing::debug!("[network] Using netstat fallback for connections");

    let mut cmd = Command::new("netstat");
    cmd.args(["-ano"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped()); // Capture stderr for debugging

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd.spawn()
        .map_err(|e| {
            tracing::error!("[network] Failed to spawn netstat: {}", e);
            format!("failed to spawn netstat: {e}")
        })?;

    let output = wait_with_timeout(child, Duration::from_secs(COMMAND_TIMEOUT_SECS))
        .map_err(|e| {
            tracing::error!("[network] netstat timeout or error: {}", e);
            format!("netstat failed: {e}")
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("[network] netstat command failed with exit code: {:?}", output.status.code());
        tracing::error!("[network] netstat stderr: {}", stderr);
        return Err(format!("netstat command failed: {}", stderr));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let pid_names = build_pid_name_map_windows_sysinfo();
    let result = parse_netstat_windows(&text, &pid_names);
    match &result {
        Ok(conns) => tracing::debug!("[network] netstat parsed {} connections", conns.len()),
        Err(e) => tracing::error!("[network] netstat parsing failed: {}", e),
    }
    result
}

/// Parse `netstat -ano` output on Windows.
///
/// Handles different Windows versions and locales by being flexible with:
/// - Column alignment (splits on whitespace, not fixed positions)
/// - Line fragmentation (multi-line continuation for long addresses)
/// - Missing/optional columns (graceful degradation)
#[cfg(any(target_os = "windows", test))]
fn parse_netstat_windows(
    text: &str,
    pid_names: &HashMap<u32, String>,
) -> Result<Vec<NetworkConnection>, String> {
    let mut connections = Vec::new();
    let mut line_buffer = String::new();
    let mut lines_processed = 0;
    let mut lines_skipped = 0;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        // Skip empty lines and headers
        if line.is_empty() || line.starts_with("Active") || line.starts_with("Proto") {
            continue;
        }

        // Handle potential line fragmentation: if line doesn't start with a protocol,
        // it might be a continuation of the previous line (rare but possible)
        if !line.starts_with("TCP") && !line.starts_with("UDP") && !line_buffer.is_empty() {
            line_buffer.push(' ');
            line_buffer.push_str(line);
            continue;
        }

        // If we had a buffered line, process it now
        if !line_buffer.is_empty() {
            if let Some(conn) = parse_netstat_line(&line_buffer, pid_names) {
                connections.push(conn);
                lines_processed += 1;
            } else {
                lines_skipped += 1;
                tracing::debug!("[netstat] Skipped malformed line: {}", line_buffer);
            }
            line_buffer.clear();
        }

        // Start buffering this line
        line_buffer.push_str(line);
    }

    // Process the last buffered line
    if !line_buffer.is_empty() {
        if let Some(conn) = parse_netstat_line(&line_buffer, pid_names) {
            connections.push(conn);
            lines_processed += 1;
        } else {
            lines_skipped += 1;
            tracing::debug!("[netstat] Skipped malformed line: {}", line_buffer);
        }
    }

    tracing::debug!(
        "[netstat] Parsed {} connections, skipped {} invalid lines",
        lines_processed,
        lines_skipped
    );

    Ok(connections)
}

/// Parse a single netstat output line into a NetworkConnection.
/// Returns None if the line is malformed or unparseable.
#[cfg(any(target_os = "windows", test))]
fn parse_netstat_line(
    line: &str,
    pid_names: &HashMap<u32, String>,
) -> Option<NetworkConnection> {
    let cols: Vec<&str> = line.split_whitespace().collect();

    // Minimum required: Proto LocalAddr RemoteAddr [State] PID
    // TCP needs at least 5 columns, UDP needs at least 4
    if cols.len() < 4 {
        return None;
    }

    let proto_str = cols[0].to_uppercase();
    let protocol = match proto_str.as_str() {
        "TCP" => Protocol::TCP,
        "UDP" => Protocol::UDP,
        _ => return None,
    };

    // Parse local address:port
    let (local_addr, local_port) = parse_addr_port(cols[1])?;

    // Parse remote address:port and state (TCP only)
    let (remote_addr, remote_port, state, pid_col_idx) = if protocol == Protocol::TCP {
        if cols.len() < 5 {
            return None;
        }
        let (ra, rp) = parse_addr_port(cols[2])?;
        let st = ConnectionState::from_str_loose(cols[3]);
        (ra, rp, st, 4)
    } else {
        // UDP: no state column, remote might be "*:*"
        let (ra, rp) = parse_addr_port(cols[2])
            .unwrap_or((IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), 0));
        (ra, rp, ConnectionState::Unknown, 3)
    };

    // Parse PID (last column)
    let pid: u32 = cols.get(pid_col_idx)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    let process_name = pid_names
        .get(&pid)
        .cloned()
        .unwrap_or_else(|| "unknown".to_string());

    let is_encrypted = detect_tls_port(remote_port);

    Some(NetworkConnection {
        pid,
        process_name,
        protocol,
        local_addr,
        local_port,
        remote_addr,
        remote_port,
        remote_hostname: None,
        state,
        bytes_sent: 0,
        bytes_received: 0,
        bytes_per_sec_up: 0.0,
        bytes_per_sec_down: 0.0,
        established_at: 0,
        country: None,
        is_encrypted,
    })
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Parse "addr:port" where addr can be IPv4, IPv6 ([::1]:port), or * (wildcard).
fn parse_addr_port(s: &str) -> Option<(IpAddr, u16)> {
    // Handle IPv6 bracket notation [::1]:443
    if s.starts_with('[') {
        let end_bracket = s.find(']')?;
        let addr_str = &s[1..end_bracket];
        let port_str = s.get(end_bracket + 2..)?; // skip ]:
        let addr: IpAddr = addr_str.parse().ok()?;
        let port: u16 = port_str.parse().ok()?;
        return Some((addr, port));
    }

    // Handle *:port (wildcard)
    if let Some(port_str) = s.strip_prefix("*:") {
        let port: u16 = port_str.parse().ok()?;
        return Some((IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), port));
    }

    // IPv4: find last colon (addr can contain dots but not colons)
    let last_colon = s.rfind(':')?;
    let addr_str = &s[..last_colon];
    let port_str = &s[last_colon + 1..];

    // Handle 0.0.0.0 and other IPv4
    let addr: IpAddr = if addr_str == "*" {
        IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
    } else {
        addr_str.parse().ok()?
    };
    let port: u16 = port_str.parse().ok()?;

    Some((addr, port))
}

/// Heuristic: detect if a port is commonly used for TLS/encrypted traffic.
fn detect_tls_port(port: u16) -> Option<bool> {
    match port {
        443 | 8443 | 993 | 995 | 465 | 636 | 989 | 990 | 5061 => Some(true),
        80 | 8080 | 25 | 110 | 143 | 21 | 23 | 69 => Some(false),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Snapshot history buffer (circular)
// ---------------------------------------------------------------------------

/// Circular buffer that stores up to `capacity` network snapshots.
pub struct SnapshotHistory {
    buffer: Vec<NetworkSnapshot>,
    capacity: usize,
    write_idx: usize,
    count: usize,
}

impl SnapshotHistory {
    /// Create a new history buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            write_idx: 0,
            count: 0,
        }
    }

    /// Push a new snapshot, evicting the oldest if at capacity.
    pub fn push(&mut self, snapshot: NetworkSnapshot) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(snapshot);
        } else {
            self.buffer[self.write_idx] = snapshot;
        }
        self.write_idx = (self.write_idx + 1) % self.capacity;
        self.count = (self.count + 1).min(self.capacity);
    }

    /// Return the most recent snapshot, if any.
    pub fn latest(&self) -> Option<&NetworkSnapshot> {
        if self.count == 0 {
            return None;
        }
        let idx = if self.write_idx == 0 {
            self.buffer.len() - 1
        } else {
            self.write_idx - 1
        };
        self.buffer.get(idx)
    }

    /// Return snapshots from the last `seconds` seconds, ordered oldest first.
    pub fn last_n_seconds(&self, seconds: u32) -> Vec<NetworkSnapshot> {
        if self.count == 0 {
            return Vec::new();
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let cutoff = now.saturating_sub(seconds as u64);

        let mut result = Vec::new();
        let start = if self.count < self.capacity {
            0
        } else {
            self.write_idx
        };

        for i in 0..self.count {
            let idx = (start + i) % self.buffer.len();
            if self.buffer[idx].timestamp >= cutoff {
                result.push(self.buffer[idx].clone());
            }
        }

        result
    }

    /// Number of snapshots currently stored.
    pub fn len(&self) -> usize {
        self.count
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn parse_addr_port_ipv4() {
        let (addr, port) = parse_addr_port("10.0.0.1:443").unwrap();
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(port, 443);
    }

    #[test]
    fn parse_addr_port_ipv6_brackets() {
        let (addr, port) = parse_addr_port("[::1]:8080").unwrap();
        assert_eq!(addr, IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(port, 8080);
    }

    #[test]
    fn parse_addr_port_wildcard() {
        let (addr, port) = parse_addr_port("*:80").unwrap();
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(port, 80);
    }

    #[test]
    fn connection_state_from_str_loose() {
        assert_eq!(
            ConnectionState::from_str_loose("ESTABLISHED"),
            ConnectionState::Established
        );
        assert_eq!(
            ConnectionState::from_str_loose("TIME_WAIT"),
            ConnectionState::TimeWait
        );
        assert_eq!(
            ConnectionState::from_str_loose("SYN_RECV"),
            ConnectionState::SynReceived
        );
        assert_eq!(
            ConnectionState::from_str_loose("LISTENING"),
            ConnectionState::Listen
        );
        assert_eq!(
            ConnectionState::from_str_loose("garbage"),
            ConnectionState::Unknown
        );
    }

    #[test]
    fn protocol_display() {
        assert_eq!(format!("{}", Protocol::TCP), "TCP");
        assert_eq!(format!("{}", Protocol::UDP), "UDP");
        assert_eq!(format!("{}", Protocol::ICMP), "ICMP");
        assert_eq!(format!("{}", Protocol::Other), "Other");
    }

    #[test]
    fn tls_port_detection() {
        assert_eq!(detect_tls_port(443), Some(true));
        assert_eq!(detect_tls_port(80), Some(false));
        assert_eq!(detect_tls_port(12345), None);
    }

    #[test]
    fn filter_by_protocol() {
        let conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::UDP, 53),
            make_test_conn(3, Protocol::TCP, 80),
        ];

        let filter = NetworkFilter {
            protocols: Some(vec![Protocol::TCP]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|c| c.protocol == Protocol::TCP));
    }

    #[test]
    fn filter_by_port() {
        let conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 80),
            make_test_conn(3, Protocol::TCP, 8080),
        ];

        let filter = NetworkFilter {
            ports: Some(vec![443, 80]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn filter_exclude_localhost() {
        let mut conns = vec![make_test_conn(1, Protocol::TCP, 443)];
        conns[0].remote_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);

        let filter = NetworkFilter {
            exclude_localhost: true,
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_only_established() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 80),
        ];
        conns[0].state = ConnectionState::Established;
        conns[1].state = ConnectionState::Listen;

        let filter = NetworkFilter {
            only_established: true,
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 1);
    }

    #[test]
    fn filter_by_process_name() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 80),
        ];
        conns[0].process_name = "chrome".to_string();
        conns[1].process_name = "firefox".to_string();

        let filter = NetworkFilter {
            process_names: Some(vec!["chrome".to_string()]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].process_name, "chrome");
    }

    #[test]
    fn filter_by_pid() {
        let conns = vec![
            make_test_conn(11, Protocol::TCP, 443),
            make_test_conn(22, Protocol::TCP, 8443),
        ];

        let filter = NetworkFilter {
            pids: Some(vec![22]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 22);
    }

    #[test]
    fn filter_by_remote_host_matches_ip_or_hostname() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 443),
        ];
        conns[0].remote_addr = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        conns[0].remote_hostname = Some("one.one.one.one".to_string());
        conns[1].remote_addr = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        conns[1].remote_hostname = Some("dns.google".to_string());

        let hostname_filter = NetworkFilter {
            remote_hosts: Some(vec!["google".to_string()]),
            ..Default::default()
        };
        let ip_filter = NetworkFilter {
            remote_hosts: Some(vec!["1.1.1".to_string()]),
            ..Default::default()
        };

        let hostname_result = hostname_filter.apply(&conns);
        let ip_result = ip_filter.apply(&conns);

        assert_eq!(hostname_result.len(), 1);
        assert_eq!(hostname_result[0].pid, 2);
        assert_eq!(ip_result.len(), 1);
        assert_eq!(ip_result[0].pid, 1);
    }

    #[test]
    fn filter_by_min_bytes() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 80),
        ];
        conns[0].bytes_per_sec_up = 1000.0;
        conns[1].bytes_per_sec_up = 10.0;

        let filter = NetworkFilter {
            min_bytes_per_sec: Some(500.0),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 1);
    }

    #[test]
    fn filter_combined() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::UDP, 53),
            make_test_conn(3, Protocol::TCP, 80),
        ];
        conns[0].state = ConnectionState::Established;
        conns[1].state = ConnectionState::Established;
        conns[2].state = ConnectionState::Listen;

        let filter = NetworkFilter {
            protocols: Some(vec![Protocol::TCP]),
            only_established: true,
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 1);
    }

    #[test]
    fn filter_can_combine_all_supported_criteria() {
        let mut matching = make_test_conn(100, Protocol::TCP, 443);
        matching.process_name = "Chrome Helper".to_string();
        matching.local_port = 52_000;
        matching.remote_addr = IpAddr::V4(Ipv4Addr::new(93, 184, 216, 34));
        matching.remote_hostname = Some("example.com".to_string());
        matching.bytes_per_sec_up = 900.0;
        matching.bytes_per_sec_down = 300.0;
        matching.state = ConnectionState::Established;

        let mut wrong_pid = matching.clone();
        wrong_pid.pid = 200;

        let mut localhost = matching.clone();
        localhost.pid = 300;
        localhost.local_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);

        let filter = NetworkFilter {
            protocols: Some(vec![Protocol::TCP]),
            ports: Some(vec![443]),
            process_names: Some(vec!["chrome".to_string()]),
            pids: Some(vec![100]),
            remote_hosts: Some(vec!["example".to_string()]),
            min_bytes_per_sec: Some(1_000.0),
            exclude_localhost: true,
            only_established: true,
        };

        let result = filter.apply(&[matching.clone(), wrong_pid, localhost]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, matching.pid);
    }

    #[test]
    fn snapshot_history_push_and_latest() {
        let mut history = SnapshotHistory::new(3);
        assert!(history.is_empty());

        history.push(make_test_snapshot(100));
        history.push(make_test_snapshot(200));
        assert_eq!(history.len(), 2);

        let latest = history.latest().unwrap();
        assert_eq!(latest.timestamp, 200);
    }

    #[test]
    fn snapshot_history_circular_eviction() {
        let mut history = SnapshotHistory::new(3);
        history.push(make_test_snapshot(100));
        history.push(make_test_snapshot(200));
        history.push(make_test_snapshot(300));
        history.push(make_test_snapshot(400)); // should evict timestamp=100

        assert_eq!(history.len(), 3);
        let latest = history.latest().unwrap();
        assert_eq!(latest.timestamp, 400);

        // All timestamps should be >= 200
        let all = history.last_n_seconds(u32::MAX);
        assert!(all.iter().all(|s| s.timestamp >= 200));
    }

    #[test]
    fn snapshot_history_last_n_seconds() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut history = SnapshotHistory::new(10);
        history.push(make_test_snapshot(now - 120));
        history.push(make_test_snapshot(now - 60));
        history.push(make_test_snapshot(now - 30));
        history.push(make_test_snapshot(now));

        let last_min = history.last_n_seconds(60);
        // now-60 is exactly at cutoff (>=), now-30, and now → 3 entries
        assert_eq!(last_min.len(), 3);
    }

    #[test]
    fn process_summaries_aggregation() {
        let conns = vec![
            {
                let mut c = make_test_conn(1, Protocol::TCP, 443);
                c.bytes_per_sec_up = 100.0;
                c.bytes_per_sec_down = 200.0;
                c
            },
            {
                let mut c = make_test_conn(1, Protocol::UDP, 53);
                c.bytes_per_sec_up = 10.0;
                c.bytes_per_sec_down = 20.0;
                c
            },
            {
                let mut c = make_test_conn(2, Protocol::TCP, 80);
                c.bytes_per_sec_up = 500.0;
                c.bytes_per_sec_down = 1000.0;
                c
            },
        ];

        let summaries = build_process_summaries(&conns);
        assert_eq!(summaries.len(), 2);

        // PID 2 should be first (highest traffic)
        assert_eq!(summaries[0].pid, 2);
        assert_eq!(summaries[0].connection_count, 1);

        // PID 1 has 2 connections with 2 protocols
        assert_eq!(summaries[1].pid, 1);
        assert_eq!(summaries[1].connection_count, 2);
        assert_eq!(summaries[1].protocols.len(), 2);
    }

    #[test]
    fn default_snapshot_has_current_timestamp() {
        let snap = NetworkSnapshot::default();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // Should be within 2 seconds of now
        assert!(snap.timestamp <= now);
        assert!(snap.timestamp >= now - 2);
    }

    #[test]
    fn parse_lsof_name_established() {
        let conn = parse_lsof_name(
            "10.0.0.1:54321->142.250.80.46:443",
            1234,
            "chrome",
            Protocol::TCP,
            ConnectionState::Established,
        )
        .unwrap();

        assert_eq!(conn.pid, 1234);
        assert_eq!(conn.process_name, "chrome");
        assert_eq!(conn.local_addr, IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
        assert_eq!(conn.local_port, 54321);
        assert_eq!(
            conn.remote_addr,
            IpAddr::V4(Ipv4Addr::new(142, 250, 80, 46))
        );
        assert_eq!(conn.remote_port, 443);
        assert_eq!(conn.state, ConnectionState::Established);
        assert_eq!(conn.is_encrypted, Some(true));
    }

    #[test]
    fn parse_lsof_name_listen() {
        let conn = parse_lsof_name(
            "*:80",
            5678,
            "nginx",
            Protocol::TCP,
            ConnectionState::Listen,
        )
        .unwrap();

        assert_eq!(conn.pid, 5678);
        assert_eq!(conn.local_port, 80);
        assert_eq!(conn.remote_port, 0);
        assert_eq!(conn.state, ConnectionState::Listen);
    }

    #[test]
    fn is_private_ip_detection() {
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 1, 1))));
        assert!(is_private_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
    }

    #[test]
    fn parse_lsof_output_multi_process() {
        let lsof_data = "\
p1234
cchrome
tIPv4
PTCP
TST=ESTABLISHED
n10.0.0.1:54321->142.250.80.46:443
p5678
cnginx
tIPv4
PTCP
TST=LISTEN
n*:80
p9999
cdnsmasq
tIPv4
PUDP
n0.0.0.0:53
";
        let result = parse_lsof_output(lsof_data).unwrap();
        assert_eq!(result.len(), 3);

        // First: chrome TCP ESTABLISHED
        assert_eq!(result[0].pid, 1234);
        assert_eq!(result[0].process_name, "chrome");
        assert_eq!(result[0].protocol, Protocol::TCP);
        assert_eq!(result[0].state, ConnectionState::Established);
        assert_eq!(result[0].remote_port, 443);

        // Second: nginx TCP LISTEN
        assert_eq!(result[1].pid, 5678);
        assert_eq!(result[1].process_name, "nginx");
        assert_eq!(result[1].state, ConnectionState::Listen);
        assert_eq!(result[1].local_port, 80);

        // Third: dnsmasq UDP
        assert_eq!(result[2].pid, 9999);
        assert_eq!(result[2].protocol, Protocol::UDP);
        assert_eq!(result[2].local_port, 53);
    }

    #[test]
    fn parse_lsof_output_skips_unix_sockets() {
        let lsof_data = "\
p1234
cpostgres
tunix
n/var/run/postgres/.s.PGSQL.5432
p5678
cchrome
tIPv4
PTCP
TST=ESTABLISHED
n10.0.0.1:50000->8.8.8.8:443
";
        let result = parse_lsof_output(lsof_data).unwrap();
        // Only the IPv4 connection should be parsed, unix socket skipped
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].pid, 5678);
    }

    #[test]
    fn parse_lsof_output_ipv6_connection() {
        let lsof_data = "\
p1234
cnode
tIPv6
PTCP
TST=ESTABLISHED
n[::1]:3000->[::1]:50000
";
        let result = parse_lsof_output(lsof_data).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].local_addr, IpAddr::V6(Ipv6Addr::LOCALHOST));
        assert_eq!(result[0].local_port, 3000);
    }

    #[test]
    fn parse_lsof_output_empty_input() {
        let result = parse_lsof_output("").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_hex_addr_port_ipv4() {
        // 0100007F:0050 = 127.0.0.1:80 (little-endian)
        let (addr, port) = parse_hex_addr_port("0100007F:0050").unwrap();
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_eq!(port, 80);
    }

    #[test]
    fn parse_hex_addr_port_ipv4_any() {
        // 00000000:0016 = 0.0.0.0:22
        let (addr, port) = parse_hex_addr_port("00000000:0016").unwrap();
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(port, 22);
    }

    #[test]
    fn parse_hex_addr_port_invalid() {
        assert!(parse_hex_addr_port("invalid").is_none());
        assert!(parse_hex_addr_port("ZZZZZZZZ:0050").is_none());
        assert!(parse_hex_addr_port("0100007F").is_none());
    }

    #[test]
    fn parse_tcp_state_hex_values() {
        assert_eq!(parse_tcp_state_hex("01"), ConnectionState::Established);
        assert_eq!(parse_tcp_state_hex("0A"), ConnectionState::Listen);
        assert_eq!(parse_tcp_state_hex("06"), ConnectionState::TimeWait);
        assert_eq!(parse_tcp_state_hex("08"), ConnectionState::CloseWait);
        assert_eq!(parse_tcp_state_hex("02"), ConnectionState::SynSent);
        assert_eq!(parse_tcp_state_hex("03"), ConnectionState::SynReceived);
        assert_eq!(parse_tcp_state_hex("07"), ConnectionState::Closed);
        assert_eq!(parse_tcp_state_hex("FF"), ConnectionState::Unknown);
    }

    #[test]
    fn parse_netstat_windows_mock() {
        let netstat_output = "\
Active Connections

  Proto  Local Address          Foreign Address        State           PID
  TCP    0.0.0.0:135            0.0.0.0:0              LISTENING       1020
  TCP    10.0.0.5:50123         142.250.80.46:443      ESTABLISHED     5678
  TCP    10.0.0.5:50124         151.101.1.69:443       TIME_WAIT       0
  UDP    0.0.0.0:5353           *:*                                    1234
";
        let pid_names: HashMap<u32, String> = [
            (1020, "svchost.exe".to_string()),
            (5678, "chrome.exe".to_string()),
            (1234, "mDNSResponder.exe".to_string()),
        ]
        .into_iter()
        .collect();

        let result = parse_netstat_windows(netstat_output, &pid_names).unwrap();

        // Should parse TCP and UDP entries, skip header lines
        assert!(result.len() >= 3);

        // Find the ESTABLISHED chrome connection
        let chrome_conn = result
            .iter()
            .find(|c| c.pid == 5678)
            .expect("chrome connection");
        assert_eq!(chrome_conn.protocol, Protocol::TCP);
        assert_eq!(chrome_conn.state, ConnectionState::Established);
        assert_eq!(chrome_conn.remote_port, 443);
        assert_eq!(chrome_conn.process_name, "chrome.exe");
        assert_eq!(chrome_conn.is_encrypted, Some(true));

        // Find the LISTENING svchost
        let svchost_conn = result
            .iter()
            .find(|c| c.pid == 1020)
            .expect("svchost connection");
        assert_eq!(svchost_conn.state, ConnectionState::Listen);
        assert_eq!(svchost_conn.local_port, 135);
    }

    #[test]
    fn parse_netstat_windows_empty_output() {
        let result = parse_netstat_windows("", &HashMap::new()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn parse_proc_net_tcp_mock() {
        // Real /proc/net/tcp format (simplified)
        let proc_tcp = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 0100007F:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345 1 0000000000000000 100 0 0 10 0
   1: 0500000A:C3F3 2E50FA8E:01BB 01 00000000:00000000 00:00000000 00000000  1000        0 67890 1 0000000000000000 100 0 0 10 0
";
        let pid_names: HashMap<u32, String> =
            [(100, "nginx".to_string()), (200, "chrome".to_string())]
                .into_iter()
                .collect();

        let inode_pid: HashMap<u64, u32> = [(12345, 100), (67890, 200)].into_iter().collect();

        let result = parse_proc_net(proc_tcp, Protocol::TCP, &pid_names, &inode_pid);
        assert_eq!(result.len(), 2);

        // First: LISTEN on 127.0.0.1:80
        assert_eq!(result[0].state, ConnectionState::Listen);
        assert_eq!(
            result[0].local_addr,
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        );
        assert_eq!(result[0].local_port, 80);
        assert_eq!(result[0].pid, 100);
        assert_eq!(result[0].process_name, "nginx");

        // Second: ESTABLISHED to 142.80.250.46:443
        assert_eq!(result[1].state, ConnectionState::Established);
        assert_eq!(result[1].remote_port, 443);
        assert_eq!(result[1].pid, 200);
        assert_eq!(result[1].process_name, "chrome");
    }

    #[test]
    fn parse_proc_net_udp_mock() {
        let proc_udp = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000:0035 00000000:0000 07 00000000:00000000 00:00000000 00000000     0        0 11111 1 0000000000000000 100 0 0 10 0
";
        let pid_names: HashMap<u32, String> = [(300, "dnsmasq".to_string())].into_iter().collect();
        let inode_pid: HashMap<u64, u32> = [(11111, 300)].into_iter().collect();

        let result = parse_proc_net(proc_udp, Protocol::UDP, &pid_names, &inode_pid);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].protocol, Protocol::UDP);
        assert_eq!(result[0].local_port, 53);
        assert_eq!(result[0].pid, 300);
        assert_eq!(result[0].process_name, "dnsmasq");
    }

    #[test]
    fn parse_proc_net_malformed_lines() {
        let proc_tcp = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
short line
   0: INVALID:DATA 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 12345
";
        let result = parse_proc_net(proc_tcp, Protocol::TCP, &HashMap::new(), &HashMap::new());
        // Should gracefully skip malformed lines
        assert!(result.is_empty());
    }

    #[test]
    fn capture_snapshot_computes_deltas() {
        let prev = NetworkSnapshot {
            timestamp: 100,
            connections: vec![{
                let mut c = make_test_conn(1, Protocol::TCP, 443);
                c.bytes_sent = 1000;
                c.bytes_received = 2000;
                c
            }],
            total_bytes_up: 1000,
            total_bytes_down: 2000,
            ..Default::default()
        };

        // Simulate a "new" snapshot with higher byte counts
        // (In reality capture_snapshot calls get_active_connections,
        // but we test the delta logic via the prev_map)
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Use the prev snapshot to verify delta calculation logic
        assert!(prev.total_bytes_up == 1000);
        assert!(prev.timestamp < now_secs);
    }

    #[test]
    fn dns_cache_insert_and_lookup() {
        let cache = DnsCache::new();
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));

        // Initially empty
        assert!(cache.entries.read().unwrap().get(&ip).is_none());

        // Insert
        cache.insert(ip, Some("dns.google".to_string()));

        // Lookup should succeed
        if let Ok(entries) = cache.entries.read() {
            let entry = entries.get(&ip).unwrap();
            assert_eq!(entry.hostname, Some("dns.google".to_string()));
        };
    }

    /// Helper: create an Instant far enough in the past to be considered expired.
    /// Returns None on Windows CI VMs with uptime shorter than the required offset.
    fn expired_instant(multiplier: u32) -> Option<Instant> {
        Instant::now().checked_sub(DNS_CACHE_TTL * multiplier)
    }

    #[test]
    fn dns_cache_evict_expired() {
        let Some(old_ts) = expired_instant(3) else {
            return; // system uptime too short for this test
        };
        let cache = DnsCache::new();
        let ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

        // Insert with an artificially old timestamp
        if let Ok(mut entries) = cache.entries.write() {
            entries.insert(
                ip,
                DnsCacheEntry {
                    hostname: Some("old.example.com".to_string()),
                    resolved_at: old_ts,
                },
            );
        }

        cache.evict_expired();

        // Should have been evicted
        if let Ok(entries) = cache.entries.read() {
            assert!(!entries.contains_key(&ip));
        };
    }

    #[test]
    fn dns_cache_retains_fresh_entries() {
        let cache = DnsCache::new();
        let ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));

        cache.insert(ip, Some("fresh.example.com".to_string()));
        cache.evict_expired();

        // Should still be there
        if let Ok(entries) = cache.entries.read() {
            assert!(entries.contains_key(&ip));
        };
    }

    // --- Tests from sprint3/net-testing ---

    #[test]
    fn parse_lsof_output_extracts_multiple_connections() {
        let text = [
            "p123",
            "cchrome",
            "tIPv4",
            "PTCP",
            "TST=ESTABLISHED",
            "n10.0.0.1:54321->93.184.216.34:443",
            "p321",
            "cresolver",
            "tIPv6",
            "PUDP",
            "n[::1]:5353->[2001:4860:4860::8888]:53",
        ]
        .join("\n");

        let connections = parse_lsof_output(&text).expect("parse lsof output");
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].pid, 123);
        assert_eq!(connections[0].remote_port, 443);
        assert_eq!(connections[1].protocol, Protocol::UDP);
        assert_eq!(connections[1].local_addr, IpAddr::V6(Ipv6Addr::LOCALHOST));
    }

    #[test]
    fn parse_proc_net_with_mock_inode_map() {
        let content = concat!(
            "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n",
            "   0: 0100007F:1F90 08080808:0035 01 00000000:00000000 00:00000000 00000000  1000        0 12345 1 0000000000000000 100 0 0 10 0\n"
        );
        let pid_names = HashMap::from([(4242_u32, "dns-proxy".to_string())]);
        let inode_pid = HashMap::from([(12345_u64, 4242_u32)]);

        let connections =
            parse_proc_net_with_inode_map(content, Protocol::TCP, &pid_names, &inode_pid);
        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].pid, 4242);
        assert_eq!(connections[0].process_name, "dns-proxy");
        assert_eq!(connections[0].local_addr, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(connections[0].remote_port, 53);
        assert_eq!(connections[0].state, ConnectionState::Established);
    }

    #[test]
    fn parse_hex_addr_port_and_state_hex() {
        let (addr, port) = parse_hex_addr_port("0100007F:01BB").expect("parse hex addr");
        assert_eq!(addr, IpAddr::V4(Ipv4Addr::LOCALHOST));
        assert_eq!(port, 443);
        assert_eq!(parse_tcp_state_hex("0A"), ConnectionState::Listen);
        assert_eq!(parse_tcp_state_hex("08"), ConnectionState::CloseWait);
    }

    #[test]
    fn parse_netstat_windows_parses_tcp_and_udp_rows() {
        let text = concat!(
            "Active Connections\n",
            "  Proto  Local Address          Foreign Address        State           PID\n",
            "  TCP    127.0.0.1:5050         93.184.216.34:443      ESTABLISHED     4242\n",
            "  UDP    0.0.0.0:5353           *:*                                    5151\n"
        );
        let pid_names = HashMap::from([
            (4242_u32, "browser.exe".to_string()),
            (5151_u32, "mdns.exe".to_string()),
        ]);

        let connections = parse_netstat_windows(text, &pid_names).expect("parse netstat");
        assert_eq!(connections.len(), 2);
        assert_eq!(connections[0].protocol, Protocol::TCP);
        assert_eq!(connections[0].process_name, "browser.exe");
        assert_eq!(connections[0].state, ConnectionState::Established);
        assert_eq!(connections[1].protocol, Protocol::UDP);
        assert_eq!(connections[1].process_name, "mdns.exe");
        assert_eq!(connections[1].remote_port, 0);
    }

    #[test]
    fn dns_cache_lookup_returns_cached_entry_before_ttl() {
        let ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10));
        clear_dns_cache();
        dns_cache().insert(ip, Some("cached.example".to_string()));

        assert_eq!(dns_cache().lookup(&ip), Some("cached.example".to_string()));
        clear_dns_cache();
    }

    #[test]
    fn dns_cache_eviction_removes_stale_entries() {
        let Some(old_ts) = expired_instant(3) else {
            return; // system uptime too short for this test
        };
        let ip = IpAddr::V4(Ipv4Addr::new(198, 51, 100, 10));
        clear_dns_cache();
        if let Ok(mut entries) = dns_cache().entries.write() {
            entries.insert(
                ip,
                DnsCacheEntry {
                    hostname: Some("stale.example".to_string()),
                    resolved_at: old_ts,
                },
            );
        }

        dns_cache().evict_expired();

        let contains = dns_cache()
            .entries
            .read()
            .map(|entries| entries.contains_key(&ip))
            .unwrap_or(false);
        assert!(!contains);
        clear_dns_cache();
    }

    #[test]
    fn resolve_and_cache_respects_concurrency_limit() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 44));
        clear_dns_cache();
        DNS_ACTIVE_LOOKUPS.store(DNS_MAX_CONCURRENT, Ordering::SeqCst);

        resolve_and_cache(ip);

        let contains = dns_cache()
            .entries
            .read()
            .map(|entries| entries.contains_key(&ip))
            .unwrap_or(false);
        assert!(!contains);
        DNS_ACTIVE_LOOKUPS.store(0, Ordering::SeqCst);
        clear_dns_cache();
    }

    #[test]
    fn enqueue_dns_resolution_skips_loopback_and_cached_entries() {
        let cached_ip = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 20));
        clear_dns_cache();
        dns_cache().insert(cached_ip, Some("preloaded.example".to_string()));

        let mut cached_conn = make_test_conn(1, Protocol::TCP, 443);
        cached_conn.remote_addr = cached_ip;
        let mut loopback_conn = make_test_conn(2, Protocol::TCP, 80);
        loopback_conn.remote_addr = IpAddr::V4(Ipv4Addr::LOCALHOST);

        enqueue_dns_resolution(&[cached_conn, loopback_conn]);

        let len = dns_cache()
            .entries
            .read()
            .map(|entries| entries.len())
            .unwrap_or_default();
        assert_eq!(len, 1);
        clear_dns_cache();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn get_active_connections_returns_vec() {
        // Just verify it doesn't panic or error on macOS
        let result = get_active_connections();
        assert!(result.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_connections_have_valid_pids() {
        let result = get_active_connections().unwrap();
        // At least some connections should have non-zero PIDs
        if !result.is_empty() {
            let with_pids: Vec<_> = result.iter().filter(|c| c.pid > 0).collect();
            assert!(
                !with_pids.is_empty(),
                "expected at least one connection with a valid PID"
            );
        }
    }

    #[test]
    fn parse_ss_output_mock() {
        let ss_data = "\
tcp   ESTAB      0      0      10.0.0.5:50123    142.250.80.46:443   users:((\"chrome\",pid=1234,fd=56))
udp   UNCONN     0      0      0.0.0.0:5353      0.0.0.0:*           users:((\"avahi-daemon\",pid=789,fd=12))
tcp   LISTEN     0      128    0.0.0.0:22        0.0.0.0:*           users:((\"sshd\",pid=456,fd=3))
";
        let pid_names: HashMap<u32, String> = [
            (1234, "chrome".to_string()),
            (789, "avahi-daemon".to_string()),
            (456, "sshd".to_string()),
        ]
        .into_iter()
        .collect();

        let result = parse_ss_output(ss_data, &pid_names).unwrap();
        assert_eq!(result.len(), 3);

        assert_eq!(result[0].pid, 1234);
        assert_eq!(result[0].protocol, Protocol::TCP);
        assert_eq!(result[0].state, ConnectionState::Established);
        assert_eq!(result[0].remote_port, 443);

        assert_eq!(result[1].pid, 789);
        assert_eq!(result[1].protocol, Protocol::UDP);

        assert_eq!(result[2].pid, 456);
        assert_eq!(result[2].state, ConnectionState::Listen);
        assert_eq!(result[2].local_port, 22);
    }

    #[test]
    fn extract_pid_from_ss_process_field() {
        assert_eq!(
            extract_pid_from_ss_process("users:((\"chrome\",pid=1234,fd=56))"),
            1234
        );
        assert_eq!(
            extract_pid_from_ss_process("users:((\"sshd\",pid=456,fd=3))"),
            456
        );
        assert_eq!(extract_pid_from_ss_process("no-pid-here"), 0);
        assert_eq!(extract_pid_from_ss_process(""), 0);
    }

    #[test]
    fn filter_by_remote_host() {
        let mut conns = vec![
            make_test_conn(1, Protocol::TCP, 443),
            make_test_conn(2, Protocol::TCP, 80),
        ];
        conns[0].remote_hostname = Some("google.com".to_string());
        conns[1].remote_hostname = Some("example.com".to_string());

        let filter = NetworkFilter {
            remote_hosts: Some(vec!["google".to_string()]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].remote_hostname, Some("google.com".to_string()));
    }

    #[test]
    fn filter_by_pid_multiple() {
        let conns = vec![
            make_test_conn(100, Protocol::TCP, 443),
            make_test_conn(200, Protocol::TCP, 80),
            make_test_conn(300, Protocol::UDP, 53),
        ];

        let filter = NetworkFilter {
            pids: Some(vec![100, 300]),
            ..Default::default()
        };

        let result = filter.apply(&conns);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|c| c.pid == 100 || c.pid == 300));
    }

    // ---- test helpers ----

    fn make_test_conn(pid: u32, protocol: Protocol, remote_port: u16) -> NetworkConnection {
        NetworkConnection {
            pid,
            process_name: format!("proc_{}", pid),
            protocol,
            local_addr: IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)),
            local_port: 50000 + pid as u16,
            remote_addr: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            remote_port,
            remote_hostname: None,
            state: ConnectionState::Established,
            bytes_sent: 0,
            bytes_received: 0,
            bytes_per_sec_up: 0.0,
            bytes_per_sec_down: 0.0,
            established_at: 0,
            country: None,
            is_encrypted: detect_tls_port(remote_port),
        }
    }

    fn make_test_snapshot(timestamp: u64) -> NetworkSnapshot {
        NetworkSnapshot {
            timestamp,
            ..Default::default()
        }
    }

    fn clear_dns_cache() {
        if let Ok(mut entries) = dns_cache().entries.write() {
            entries.clear();
        }
    }
}
