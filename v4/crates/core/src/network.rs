use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::Networks;
use tokio::sync::mpsc::{self, Receiver};

const CAPTURE_WINDOW_MS: u64 = 250;
const MAX_CONNECTION_EVENTS: usize = 256;
const CAPTURE_QUEUE_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkCaptureBackend {
    Ebpf,
    PacketFilter,
    WinDivert,
    Unsupported,
}

impl NetworkCaptureBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ebpf => "eBPF",
            Self::PacketFilter => "Packet Filter (libpcap)",
            Self::WinDivert => "WinDivert",
            Self::Unsupported => "Unsupported",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransportProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrafficDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConnectionEvent {
    pub pid: u32,
    pub protocol: TransportProtocol,
    pub direction: TrafficDirection,
    pub src_ip: String,
    pub dst_ip: String,
    pub src_port: u16,
    pub dst_port: u16,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessNetworkThroughput {
    pub pid: u32,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub tcp_packets_per_sec: u64,
    pub udp_packets_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowSample {
    pub backend: NetworkCaptureBackend,
    pub backend_label: String,
    pub privileged_path_available: bool,
    pub deep_packet_inspection_active: bool,
    pub net_rx_bytes_per_sec: u64,
    pub net_tx_bytes_per_sec: u64,
    pub observed_interval_ms: u128,
    pub process_throughput: Vec<ProcessNetworkThroughput>,
    pub recent_connections: Vec<ProcessConnectionEvent>,
    pub capture_windows_dropped: u64,
    pub captured_at_unix_ms: u128,
}

#[derive(Debug, Default, Clone)]
struct ProcessTrafficAcc {
    rx_bytes: u64,
    tx_bytes: u64,
    tcp_packets: u64,
    udp_packets: u64,
}

#[derive(Debug, Default, Clone)]
struct CollectorWindow {
    process_traffic: HashMap<u32, ProcessTrafficAcc>,
    recent_connections: Vec<ProcessConnectionEvent>,
}

impl CollectorWindow {
    fn merge_from(&mut self, incoming: CollectorWindow) {
        for (pid, acc) in incoming.process_traffic {
            let target = self.process_traffic.entry(pid).or_default();
            target.rx_bytes = target.rx_bytes.saturating_add(acc.rx_bytes);
            target.tx_bytes = target.tx_bytes.saturating_add(acc.tx_bytes);
            target.tcp_packets = target.tcp_packets.saturating_add(acc.tcp_packets);
            target.udp_packets = target.udp_packets.saturating_add(acc.udp_packets);
        }

        self.recent_connections.extend(
            incoming
                .recent_connections
                .into_iter()
                .take(MAX_CONNECTION_EVENTS),
        );
        if self.recent_connections.len() > MAX_CONNECTION_EVENTS {
            let cut = self.recent_connections.len() - MAX_CONNECTION_EVENTS;
            self.recent_connections.drain(0..cut);
        }
    }

    fn into_rates(
        self,
        interval_ms: u128,
    ) -> (Vec<ProcessNetworkThroughput>, Vec<ProcessConnectionEvent>) {
        let mut rates = self
            .process_traffic
            .into_iter()
            .map(|(pid, acc)| ProcessNetworkThroughput {
                pid,
                rx_bytes_per_sec: scale_to_per_sec(acc.rx_bytes, interval_ms),
                tx_bytes_per_sec: scale_to_per_sec(acc.tx_bytes, interval_ms),
                tcp_packets_per_sec: scale_to_per_sec(acc.tcp_packets, interval_ms),
                udp_packets_per_sec: scale_to_per_sec(acc.udp_packets, interval_ms),
            })
            .collect::<Vec<_>>();
        rates.sort_by(|a, b| {
            (b.rx_bytes_per_sec + b.tx_bytes_per_sec)
                .cmp(&(a.rx_bytes_per_sec + a.tx_bytes_per_sec))
        });
        rates.truncate(128);

        (rates, self.recent_connections)
    }
}

trait NativeCollector: Send {
    fn is_active(&self) -> bool;
    fn capture_window(&mut self, interval: Duration) -> CollectorWindow;
}

struct NoopCollector;

impl NativeCollector for NoopCollector {
    fn is_active(&self) -> bool {
        false
    }

    fn capture_window(&mut self, _interval: Duration) -> CollectorWindow {
        CollectorWindow::default()
    }
}

pub struct NetworkTelemetryEngine {
    backend: NetworkCaptureBackend,
    privileged_path_available: bool,
    deep_packet_inspection_active: bool,
    capture_rx: Receiver<CollectorWindow>,
    dropped_windows: Arc<AtomicU64>,
    pending_capture: CollectorWindow,
    networks: Networks,
    prev_rx: u64,
    prev_tx: u64,
    last_tick: Instant,
}

impl Default for NetworkTelemetryEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkTelemetryEngine {
    pub fn new() -> Self {
        let networks = Networks::new_with_refreshed_list();
        let prev_rx = networks
            .values()
            .map(sysinfo::NetworkData::total_received)
            .sum();
        let prev_tx = networks
            .values()
            .map(sysinfo::NetworkData::total_transmitted)
            .sum();

        let choice = make_collector();
        let deep_packet_inspection_active = choice.collector.is_active();
        let (capture_tx, capture_rx) = mpsc::channel(CAPTURE_QUEUE_DEPTH);
        let dropped_windows = Arc::new(AtomicU64::new(0));
        let dropped_windows_thread = Arc::clone(&dropped_windows);

        std::thread::spawn(move || {
            let mut collector = choice.collector;
            let interval = Duration::from_millis(CAPTURE_WINDOW_MS);
            loop {
                let window = collector.capture_window(interval);
                match capture_tx.try_send(window) {
                    Ok(_) => {}
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => break,
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        dropped_windows_thread.fetch_add(1, Ordering::Relaxed);
                    }
                }
                if dropped_windows_thread.load(Ordering::Relaxed) > 10_000 {
                    break;
                }
                std::thread::sleep(Duration::from_millis(15));
            }
        });

        Self {
            backend: choice.backend,
            privileged_path_available: choice.privileged_path_available,
            deep_packet_inspection_active,
            capture_rx,
            dropped_windows,
            pending_capture: CollectorWindow::default(),
            networks,
            prev_rx,
            prev_tx,
            last_tick: Instant::now(),
        }
    }

    pub fn sample(&mut self) -> NetworkFlowSample {
        while let Ok(window) = self.capture_rx.try_recv() {
            self.pending_capture.merge_from(window);
        }

        self.networks.refresh();
        let total_rx: u64 = self
            .networks
            .values()
            .map(sysinfo::NetworkData::total_received)
            .sum();
        let total_tx: u64 = self
            .networks
            .values()
            .map(sysinfo::NetworkData::total_transmitted)
            .sum();

        let interval_ms = self.last_tick.elapsed().as_millis().max(1);
        self.last_tick = Instant::now();

        let rx_delta = total_rx.saturating_sub(self.prev_rx);
        let tx_delta = total_tx.saturating_sub(self.prev_tx);
        self.prev_rx = total_rx;
        self.prev_tx = total_tx;

        let capture = std::mem::take(&mut self.pending_capture);
        let (process_throughput, recent_connections) = capture.into_rates(interval_ms);

        NetworkFlowSample {
            backend: self.backend,
            backend_label: self.backend.as_str().to_string(),
            privileged_path_available: self.privileged_path_available,
            deep_packet_inspection_active: self.deep_packet_inspection_active,
            net_rx_bytes_per_sec: scale_to_per_sec(rx_delta, interval_ms),
            net_tx_bytes_per_sec: scale_to_per_sec(tx_delta, interval_ms),
            observed_interval_ms: interval_ms,
            process_throughput,
            recent_connections,
            capture_windows_dropped: self.dropped_windows.load(Ordering::Relaxed),
            captured_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
        }
    }
}

fn scale_to_per_sec(value: u64, interval_ms: u128) -> u64 {
    let scaled = u128::from(value).saturating_mul(1000) / interval_ms.max(1);
    u64::try_from(scaled).unwrap_or(u64::MAX)
}

struct CollectorChoice {
    backend: NetworkCaptureBackend,
    privileged_path_available: bool,
    collector: Box<dyn NativeCollector>,
}

#[cfg(target_os = "macos")]
fn make_collector() -> CollectorChoice {
    match macos::MacPcapCollector::new() {
        Ok(c) => CollectorChoice {
            backend: NetworkCaptureBackend::PacketFilter,
            privileged_path_available: c.is_active(),
            collector: Box::new(c),
        },
        Err(_) => CollectorChoice {
            backend: NetworkCaptureBackend::PacketFilter,
            privileged_path_available: false,
            collector: Box::new(NoopCollector),
        },
    }
}

#[cfg(target_os = "windows")]
fn make_collector() -> CollectorChoice {
    match windows_collector::WinDivertCollector::new() {
        Ok(c) => CollectorChoice {
            backend: NetworkCaptureBackend::WinDivert,
            privileged_path_available: c.is_active(),
            collector: Box::new(c),
        },
        Err(_) => CollectorChoice {
            backend: NetworkCaptureBackend::WinDivert,
            privileged_path_available: false,
            collector: Box::new(NoopCollector),
        },
    }
}

#[cfg(target_os = "linux")]
fn make_collector() -> CollectorChoice {
    match linux_ebpf::LinuxEbpfCollector::new() {
        Ok(c) => CollectorChoice {
            backend: NetworkCaptureBackend::Ebpf,
            privileged_path_available: c.is_active(),
            collector: Box::new(c),
        },
        Err(_) => CollectorChoice {
            backend: NetworkCaptureBackend::Ebpf,
            privileged_path_available: false,
            collector: Box::new(NoopCollector),
        },
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn make_collector() -> CollectorChoice {
    CollectorChoice {
        backend: NetworkCaptureBackend::Unsupported,
        privileged_path_available: false,
        collector: Box::new(NoopCollector),
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{
        CollectorWindow, NativeCollector, ProcessConnectionEvent, ProcessTrafficAcc,
        TrafficDirection, TransportProtocol,
    };
    use pcap::{Active, Capture, Device};
    use std::collections::HashMap;
    use std::process::Command;
    use std::time::{Duration, Instant};

    pub struct MacPcapCollector {
        cap: Option<Capture<Active>>,
        active: bool,
        port_pid_map: HashMap<u16, u32>,
        map_refreshed_at: Instant,
    }

    impl MacPcapCollector {
        pub fn new() -> Result<Self, String> {
            let device = Device::lookup()
                .map_err(|e| format!("pcap device lookup failed: {e}"))?
                .or_else(|| Device::list().ok().and_then(|mut d| d.pop()))
                .ok_or_else(|| "no network capture device available".to_string())?;

            let cap = Capture::from_device(device)
                .map_err(|e| format!("pcap from_device failed: {e}"))?
                .promisc(false)
                .immediate_mode(true)
                .timeout(100)
                .open()
                .map_err(|e| format!("pcap open failed: {e}"))?;

            Ok(Self {
                cap: Some(cap),
                active: true,
                port_pid_map: HashMap::new(),
                map_refreshed_at: Instant::now() - Duration::from_secs(10),
            })
        }

        fn maybe_refresh_socket_map(&mut self) {
            if self.map_refreshed_at.elapsed() < Duration::from_secs(2) {
                return;
            }
            self.port_pid_map = build_port_pid_map();
            self.map_refreshed_at = Instant::now();
        }
    }

    impl NativeCollector for MacPcapCollector {
        fn is_active(&self) -> bool {
            self.active
        }

        fn capture_window(&mut self, interval: Duration) -> CollectorWindow {
            self.maybe_refresh_socket_map();
            let Some(cap) = self.cap.as_mut() else {
                return CollectorWindow::default();
            };
            let started = Instant::now();
            let mut process_traffic: HashMap<u32, ProcessTrafficAcc> = HashMap::new();
            let mut recent_connections = Vec::new();

            while started.elapsed() < interval {
                let Ok(packet) = cap.next_packet() else {
                    break;
                };

                let Some(meta) = super::parse_ipv4_transport(packet.data) else {
                    continue;
                };

                if let Some(pid) = self.port_pid_map.get(&meta.src_port) {
                    let entry = process_traffic.entry(*pid).or_default();
                    entry.tx_bytes = entry.tx_bytes.saturating_add(meta.total_len);
                    if meta.protocol == TransportProtocol::Tcp {
                        entry.tcp_packets = entry.tcp_packets.saturating_add(1);
                    } else {
                        entry.udp_packets = entry.udp_packets.saturating_add(1);
                    }

                    recent_connections.push(ProcessConnectionEvent {
                        pid: *pid,
                        protocol: meta.protocol,
                        direction: TrafficDirection::Outbound,
                        src_ip: meta.src_ip.clone(),
                        dst_ip: meta.dst_ip.clone(),
                        src_port: meta.src_port,
                        dst_port: meta.dst_port,
                        bytes: meta.total_len,
                    });
                }

                if let Some(pid) = self.port_pid_map.get(&meta.dst_port) {
                    let entry = process_traffic.entry(*pid).or_default();
                    entry.rx_bytes = entry.rx_bytes.saturating_add(meta.total_len);
                    if meta.protocol == TransportProtocol::Tcp {
                        entry.tcp_packets = entry.tcp_packets.saturating_add(1);
                    } else {
                        entry.udp_packets = entry.udp_packets.saturating_add(1);
                    }

                    recent_connections.push(ProcessConnectionEvent {
                        pid: *pid,
                        protocol: meta.protocol,
                        direction: TrafficDirection::Inbound,
                        src_ip: meta.src_ip,
                        dst_ip: meta.dst_ip,
                        src_port: meta.src_port,
                        dst_port: meta.dst_port,
                        bytes: meta.total_len,
                    });
                }

                if recent_connections.len() >= 128 {
                    break;
                }
            }

            CollectorWindow {
                process_traffic,
                recent_connections,
            }
        }
    }

    fn build_port_pid_map() -> HashMap<u16, u32> {
        let mut map = HashMap::new();
        let output = Command::new("lsof")
            .args(["-nP", "-iTCP", "-iUDP", "-Fpn"])
            .output();

        let Ok(output) = output else {
            return map;
        };
        if !output.status.success() {
            return map;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut current_pid: Option<u32> = None;

        for line in text.lines() {
            if let Some(pid_raw) = line.strip_prefix('p') {
                current_pid = pid_raw.parse::<u32>().ok();
                continue;
            }
            if let Some(net_raw) = line.strip_prefix('n') {
                let Some(pid) = current_pid else {
                    continue;
                };
                if let Some(port) = extract_port(net_raw) {
                    map.insert(port, pid);
                }
            }
        }

        map
    }

    fn extract_port(socket_repr: &str) -> Option<u16> {
        let normalized = socket_repr.split("->").next().unwrap_or(socket_repr).trim();
        let port_part = normalized.rsplit(':').next()?;
        port_part.parse::<u16>().ok()
    }
}

#[cfg(target_os = "linux")]
mod linux_ebpf {
    use super::{
        CollectorWindow, NativeCollector, ProcessConnectionEvent, ProcessTrafficAcc,
        TrafficDirection, TransportProtocol,
    };
    use aya::maps::HashMap as BpfHashMap;
    use aya::programs::KProbe;
    use aya::Ebpf;
    use std::collections::HashMap;
    use std::net::Ipv4Addr;
    use std::time::Duration;

    pub struct LinuxEbpfCollector {
        bpf: Option<Ebpf>,
        active: bool,
        prev_tcp: HashMap<u32, u64>,
        prev_udp: HashMap<u32, u64>,
    }

    impl LinuxEbpfCollector {
        pub fn new() -> Result<Self, String> {
            let object_path = std::env::var("OMNIMON_EBPF_OBJECT")
                .unwrap_or_else(|_| "/opt/omnimon/tcp_udp_sendmsg.bpf.o".to_string());

            let mut bpf = Ebpf::load_file(&object_path)
                .map_err(|e| format!("failed to load eBPF object {object_path}: {e}"))?;

            if let Some(program) = bpf.program_mut("trace_tcp_sendmsg") {
                let probe: &mut KProbe = program
                    .try_into()
                    .map_err(|e| format!("tcp kprobe cast failed: {e}"))?;
                probe
                    .load()
                    .map_err(|e| format!("tcp kprobe load failed: {e}"))?;
                probe
                    .attach("tcp_sendmsg", 0)
                    .map_err(|e| format!("tcp kprobe attach failed: {e}"))?;
            }

            if let Some(program) = bpf.program_mut("trace_udp_sendmsg") {
                let probe: &mut KProbe = program
                    .try_into()
                    .map_err(|e| format!("udp kprobe cast failed: {e}"))?;
                probe
                    .load()
                    .map_err(|e| format!("udp kprobe load failed: {e}"))?;
                probe
                    .attach("udp_sendmsg", 0)
                    .map_err(|e| format!("udp kprobe attach failed: {e}"))?;
            }

            Ok(Self {
                bpf: Some(bpf),
                active: true,
                prev_tcp: HashMap::new(),
                prev_udp: HashMap::new(),
            })
        }
    }

    impl NativeCollector for LinuxEbpfCollector {
        fn is_active(&self) -> bool {
            self.active
        }

        fn capture_window(&mut self, _interval: Duration) -> CollectorWindow {
            let Some(bpf) = self.bpf.as_mut() else {
                return CollectorWindow::default();
            };

            let mut process_traffic = HashMap::new();
            let mut recent_connections = Vec::new();

            // Collect TCP traffic data first, then do destination lookups
            // (avoids double mutable borrow of bpf)
            let mut tcp_deltas: Vec<(u32, u64)> = Vec::new();
            if let Some(map) = bpf.map_mut("PROCESS_NET_TCP_BYTES") {
                if let Ok(tcp_map) = BpfHashMap::<_, u32, u64>::try_from(map) {
                    for pid in tcp_map.keys().flatten() {
                        if let Ok(current) = tcp_map.get(&pid, 0) {
                            let prev = self.prev_tcp.insert(pid, current).unwrap_or(0);
                            let delta = current.saturating_sub(prev);
                            let entry = process_traffic
                                .entry(pid)
                                .or_insert_with(ProcessTrafficAcc::default);
                            entry.tx_bytes = entry.tx_bytes.saturating_add(delta);
                            entry.tcp_packets = entry.tcp_packets.saturating_add(1);
                            tcp_deltas.push((pid, delta));
                        }
                    }
                }
            }
            for (pid, delta) in tcp_deltas {
                if let Some((dst_ip, dst_port)) = tcp_destination_for_pid(bpf, pid) {
                    recent_connections.push(ProcessConnectionEvent {
                        pid,
                        protocol: TransportProtocol::Tcp,
                        direction: TrafficDirection::Outbound,
                        src_ip: "0.0.0.0".to_string(),
                        dst_ip,
                        src_port: 0,
                        dst_port,
                        bytes: delta,
                    });
                }
            }

            // Collect UDP traffic data first, then do destination lookups
            let mut udp_deltas: Vec<(u32, u64)> = Vec::new();
            if let Some(map) = bpf.map_mut("PROCESS_NET_UDP_BYTES") {
                if let Ok(udp_map) = BpfHashMap::<_, u32, u64>::try_from(map) {
                    for pid in udp_map.keys().flatten() {
                        if let Ok(current) = udp_map.get(&pid, 0) {
                            let prev = self.prev_udp.insert(pid, current).unwrap_or(0);
                            let delta = current.saturating_sub(prev);
                            let entry = process_traffic
                                .entry(pid)
                                .or_insert_with(ProcessTrafficAcc::default);
                            entry.tx_bytes = entry.tx_bytes.saturating_add(delta);
                            entry.udp_packets = entry.udp_packets.saturating_add(1);
                            udp_deltas.push((pid, delta));
                        }
                    }
                }
            }
            for (pid, delta) in udp_deltas {
                if let Some((dst_ip, dst_port)) = udp_destination_for_pid(bpf, pid) {
                    recent_connections.push(ProcessConnectionEvent {
                        pid,
                        protocol: TransportProtocol::Udp,
                        direction: TrafficDirection::Outbound,
                        src_ip: "0.0.0.0".to_string(),
                        dst_ip,
                        src_port: 0,
                        dst_port,
                        bytes: delta,
                    });
                }
            }

            CollectorWindow {
                process_traffic,
                recent_connections,
            }
        }
    }

    fn tcp_destination_for_pid(bpf: &mut Ebpf, pid: u32) -> Option<(String, u16)> {
        destination_for_pid(
            bpf,
            pid,
            "PROCESS_NET_TCP_DST_IP",
            "PROCESS_NET_TCP_DST_PORT",
        )
    }

    fn udp_destination_for_pid(bpf: &mut Ebpf, pid: u32) -> Option<(String, u16)> {
        destination_for_pid(
            bpf,
            pid,
            "PROCESS_NET_UDP_DST_IP",
            "PROCESS_NET_UDP_DST_PORT",
        )
    }

    fn destination_for_pid(
        bpf: &mut Ebpf,
        pid: u32,
        ip_map_name: &str,
        port_map_name: &str,
    ) -> Option<(String, u16)> {
        // Look up IP first, drop the borrow, then look up port
        let ip = {
            let map = bpf.map_mut(ip_map_name)?;
            let ip_map = BpfHashMap::<_, u32, u32>::try_from(map).ok()?;
            ip_map.get(&pid, 0).ok()?
        };
        let port = {
            let map = bpf.map_mut(port_map_name)?;
            let port_map = BpfHashMap::<_, u32, u16>::try_from(map).ok()?;
            port_map.get(&pid, 0).ok()?
        };
        Some((Ipv4Addr::from(ip).to_string(), port))
    }
}

#[cfg(target_os = "windows")]
mod windows_collector {
    use super::{
        CollectorWindow, NativeCollector, ProcessConnectionEvent, ProcessTrafficAcc,
        TrafficDirection, TransportProtocol,
    };
    use std::collections::HashMap;
    use std::process::Command;
    use std::time::{Duration, Instant};

    type Handle = isize;

    #[repr(C)]
    struct WinDivertAddress {
        _raw: [u8; 64],
    }

    #[link(name = "WinDivert")]
    extern "system" {
        fn WinDivertOpen(filter: *const i8, layer: u32, priority: i16, flags: u64) -> Handle;
        fn WinDivertRecv(
            handle: Handle,
            packet: *mut core::ffi::c_void,
            packet_len: u32,
            addr: *mut WinDivertAddress,
            read_len: *mut u32,
        ) -> i32;
        fn WinDivertClose(handle: Handle) -> i32;
    }

    pub struct WinDivertCollector {
        handle: Option<Handle>,
        port_pid_map: HashMap<u16, u32>,
        map_refreshed_at: Instant,
    }

    impl WinDivertCollector {
        pub fn new() -> Result<Self, String> {
            let filter = std::ffi::CString::new("ip and (tcp or udp)")
                .map_err(|e| format!("invalid WinDivert filter: {e}"))?;
            // SAFETY: FFI call with stable parameters and managed lifecycle via Drop.
            let handle = unsafe { WinDivertOpen(filter.as_ptr(), 0, 0, 1) };
            if handle == -1 {
                return Err("WinDivertOpen failed".to_string());
            }
            Ok(Self {
                handle: Some(handle),
                port_pid_map: HashMap::new(),
                map_refreshed_at: Instant::now() - Duration::from_secs(10),
            })
        }

        fn maybe_refresh_socket_map(&mut self) {
            if self.map_refreshed_at.elapsed() < Duration::from_secs(2) {
                return;
            }
            self.port_pid_map = build_port_pid_map();
            self.map_refreshed_at = Instant::now();
        }
    }

    impl NativeCollector for WinDivertCollector {
        fn is_active(&self) -> bool {
            self.handle.is_some()
        }

        fn capture_window(&mut self, interval: Duration) -> CollectorWindow {
            let Some(handle) = self.handle else {
                return CollectorWindow::default();
            };

            self.maybe_refresh_socket_map();
            let started = Instant::now();
            let mut buf = [0u8; 2000];
            let mut addr = WinDivertAddress { _raw: [0; 64] };
            let mut read_len: u32 = 0;
            let mut process_traffic: HashMap<u32, ProcessTrafficAcc> = HashMap::new();
            let mut recent_connections = Vec::new();

            while started.elapsed() < interval {
                // SAFETY: Buffer and output pointers are valid for the call duration.
                let ok = unsafe {
                    WinDivertRecv(
                        handle,
                        buf.as_mut_ptr().cast(),
                        buf.len() as u32,
                        &mut addr,
                        &mut read_len,
                    )
                };
                if ok == 0 || read_len == 0 {
                    continue;
                }

                let data = &buf[..usize::try_from(read_len).unwrap_or(0)];
                let Some(meta) = super::parse_ipv4_transport_no_eth(data) else {
                    continue;
                };

                if let Some(pid) = self.port_pid_map.get(&meta.src_port) {
                    let entry = process_traffic.entry(*pid).or_default();
                    entry.tx_bytes = entry.tx_bytes.saturating_add(meta.total_len);
                    if meta.protocol == TransportProtocol::Tcp {
                        entry.tcp_packets = entry.tcp_packets.saturating_add(1);
                    } else {
                        entry.udp_packets = entry.udp_packets.saturating_add(1);
                    }

                    recent_connections.push(ProcessConnectionEvent {
                        pid: *pid,
                        protocol: meta.protocol,
                        direction: TrafficDirection::Outbound,
                        src_ip: meta.src_ip,
                        dst_ip: meta.dst_ip,
                        src_port: meta.src_port,
                        dst_port: meta.dst_port,
                        bytes: meta.total_len,
                    });
                }

                if recent_connections.len() >= 128 {
                    break;
                }
            }

            CollectorWindow {
                process_traffic,
                recent_connections,
            }
        }
    }

    impl Drop for WinDivertCollector {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                // SAFETY: Handle was obtained from WinDivertOpen and closed exactly once.
                let _ = unsafe { WinDivertClose(handle) };
            }
        }
    }

    fn build_port_pid_map() -> HashMap<u16, u32> {
        let mut map = HashMap::new();
        for command in [["-ano", "-p", "tcp"], ["-ano", "-p", "udp"]] {
            let output = Command::new("netstat").args(command).output();
            let Ok(output) = output else {
                continue;
            };
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                let cols = line.split_whitespace().collect::<Vec<_>>();
                if cols.len() < 4 {
                    continue;
                }
                let local = cols[1];
                let pid_col = cols.last().copied().unwrap_or_default();
                let Some(port) = local.rsplit(':').next().and_then(|p| p.parse::<u16>().ok())
                else {
                    continue;
                };
                let Some(pid) = pid_col.parse::<u32>().ok() else {
                    continue;
                };
                map.insert(port, pid);
            }
        }
        map
    }
}

#[derive(Debug, Clone)]
struct ParsedTransport {
    protocol: TransportProtocol,
    src_ip: String,
    dst_ip: String,
    src_port: u16,
    dst_port: u16,
    total_len: u64,
}

fn parse_ipv4_transport(frame: &[u8]) -> Option<ParsedTransport> {
    if frame.len() < 34 {
        return None;
    }
    let ether_type = u16::from_be_bytes([frame[12], frame[13]]);
    if ether_type != 0x0800 {
        return None;
    }
    parse_ipv4_transport_inner(frame, 14)
}

#[cfg(target_os = "windows")]
fn parse_ipv4_transport_no_eth(frame: &[u8]) -> Option<ParsedTransport> {
    parse_ipv4_transport_inner(frame, 0)
}

fn parse_ipv4_transport_inner(frame: &[u8], ip_offset: usize) -> Option<ParsedTransport> {
    if frame.len() < ip_offset + 20 {
        return None;
    }

    let version = frame[ip_offset] >> 4;
    if version != 4 {
        return None;
    }

    let ihl = usize::from(frame[ip_offset] & 0x0F) * 4;
    let proto = frame[ip_offset + 9];
    let total_len = u16::from_be_bytes([frame[ip_offset + 2], frame[ip_offset + 3]]) as u64;
    let transport_offset = ip_offset + ihl;
    if frame.len() < transport_offset + 4 {
        return None;
    }

    let protocol = match proto {
        6 => TransportProtocol::Tcp,
        17 => TransportProtocol::Udp,
        _ => return None,
    };

    let src_ip = format!(
        "{}.{}.{}.{}",
        frame[ip_offset + 12],
        frame[ip_offset + 13],
        frame[ip_offset + 14],
        frame[ip_offset + 15]
    );
    let dst_ip = format!(
        "{}.{}.{}.{}",
        frame[ip_offset + 16],
        frame[ip_offset + 17],
        frame[ip_offset + 18],
        frame[ip_offset + 19]
    );
    let src_port = u16::from_be_bytes([frame[transport_offset], frame[transport_offset + 1]]);
    let dst_port = u16::from_be_bytes([frame[transport_offset + 2], frame[transport_offset + 3]]);

    Some(ParsedTransport {
        protocol,
        src_ip,
        dst_ip,
        src_port,
        dst_port,
        total_len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_labels_are_stable() {
        assert_eq!(NetworkCaptureBackend::Ebpf.as_str(), "eBPF");
        assert_eq!(
            NetworkCaptureBackend::PacketFilter.as_str(),
            "Packet Filter (libpcap)"
        );
    }

    #[test]
    fn parse_ipv4_tcp_packet() {
        let mut frame = vec![0u8; 54];
        frame[12] = 0x08;
        frame[13] = 0x00;
        frame[14] = 0x45;
        frame[16] = 0;
        frame[17] = 40;
        frame[23] = 6;
        frame[26..30].copy_from_slice(&[10, 0, 0, 1]);
        frame[30..34].copy_from_slice(&[8, 8, 8, 8]);
        frame[34..36].copy_from_slice(&443u16.to_be_bytes());
        frame[36..38].copy_from_slice(&50500u16.to_be_bytes());

        let parsed = parse_ipv4_transport(&frame).expect("parsed ipv4 tcp frame");
        assert_eq!(parsed.protocol, TransportProtocol::Tcp);
        assert_eq!(parsed.src_ip, "10.0.0.1");
        assert_eq!(parsed.dst_ip, "8.8.8.8");
        assert_eq!(parsed.src_port, 443);
    }

    #[test]
    fn engine_produces_sample() {
        let mut engine = NetworkTelemetryEngine::new();
        std::thread::sleep(Duration::from_millis(300));
        let sample = engine.sample();
        assert!(sample.observed_interval_ms >= 1);
        assert!(!sample.backend_label.is_empty());
    }
}
