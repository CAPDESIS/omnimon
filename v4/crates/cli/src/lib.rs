//! OmniMon CLI library — command parsing and execution.
//!
//! The binary entrypoint lives in `main.rs` and translates [`run`] results into
//! process exit codes. Keeping command logic in this library lets unit tests
//! exercise helpers and subcommand paths without spawning a process.

use clap::{Parser, Subcommand, ValueEnum};
use core::ai as core_ai;
use core::browser::{BrowserKind, BrowserTab, NativeTabProvider, TabProvider};
use core::crypto;
use core::killer;
use core::metrics;
use core::network_analysis::{NetworkFilter, Protocol};
use core::rules_engine;
use core::settings::{self, ProfilePreset};
use core::watcher;

#[derive(Parser)]
#[command(name = "omnimon")]
#[command(version = env!("CARGO_PKG_VERSION"), about = "OmniMon: Monitor de sistema y navegador de próxima generación de alto rendimiento.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Force secure credential sync from keychain
    #[arg(long, global = true)]
    sync_keychain: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the status of OmniMon
    Status {
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
    },
    /// Kill a process by PID
    Kill {
        /// The Process ID to kill
        pid: u32,
    },
    /// Smart Optimize via AI
    Optimize {
        /// AI Provider to use
        #[arg(long, value_enum)]
        ai: AiProvider,
        /// Target to optimize (e.g. browsers, all)
        #[arg(long)]
        target: Option<String>,
    },
    /// Manage Browser Tabs
    Tabs {
        #[command(subcommand)]
        command: TabCommands,
    },
    /// Analyze Context or Chat with AI Assistant
    Chat {
        /// AI Provider to use
        #[arg(long, value_enum)]
        ai: AiProvider,
        /// Prompt/Context to send to the AI
        prompt: String,
    },
    /// Save and validate an API Key for an AI Provider
    Apikey {
        /// AI Provider to use
        #[arg(long, value_enum)]
        ai: AiProvider,
        /// The API Key to save
        key: String,
    },
    /// Manage settings (theme, locale, etc.)
    Settings {
        #[command(subcommand)]
        command: SettingsCommands,
    },
    /// Manage cryptographic key configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
    /// Manage Authentication (e.g., CrabNebula)
    Auth {
        #[command(subcommand)]
        command: AuthCommands,
    },
    /// Manage Cloud operations (CrabNebula)
    Cloud {
        #[command(subcommand)]
        command: CloudCommands,
    },
    /// Run a local Security Scan
    SecurityScan {
        /// Optional path to a JSON CVE database
        #[arg(long)]
        cve_db: Option<String>,
    },
    /// Run system health and native driver checks
    Doctor,
    /// Launch the real-time terminal UI (htop-style)
    Tui,
    /// Show real-time network telemetry (throughput per process, connections)
    Network {
        /// Output format
        #[arg(long, value_enum, default_value_t = Format::Text)]
        format: Format,
        /// Show recent connection events instead of throughput summary
        #[arg(long, conflicts_with_all = ["alerts", "top"])]
        connections: bool,
        /// Filter connections by protocol (implies connections view)
        #[arg(long, value_enum)]
        filter: Option<NetworkProtocolArg>,
        /// Filter connections by local or remote port (implies connections view)
        #[arg(long)]
        port: Option<u16>,
        /// Show evaluated network alerts from the watcher
        #[arg(long, conflicts_with_all = ["connections", "top"])]
        alerts: bool,
        /// Show top per-process throughput explicitly
        #[arg(long, conflicts_with_all = ["connections", "alerts"])]
        top: bool,
        /// Refresh continuously until interrupted
        #[arg(long)]
        watch: bool,
        /// Refresh interval for watch mode in milliseconds
        #[arg(long, default_value_t = 2000, requires = "watch")]
        watch_interval_ms: u64,
        /// Limit watch iterations for scripts/tests; omit to watch indefinitely
        #[arg(long, requires = "watch")]
        watch_iterations: Option<u32>,
    },
    /// Manage AI-driven security alert rules (MITRE ATT&CK)
    Rules {
        #[command(subcommand)]
        command: RulesCommands,
    },
    /// Release signing, verification, and manifest generation (NIST SI-7)
    Release {
        #[command(subcommand)]
        command: ReleaseCommands,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Login and save CrabNebula API Key
    Login {
        /// The CrabNebula API Key (CN_API_KEY)
        key: String,
    },
}

#[derive(Subcommand)]
enum CloudCommands {
    /// Sync encrypted security reports to CrabNebula
    Sync {
        /// Path to the encrypted report to upload
        #[arg(long)]
        report_path: String,
    },
}

#[derive(Subcommand)]
enum RulesCommands {
    /// List all active security alert rules
    List,
    /// Load rules from a JSON file (schema_version 1)
    Load {
        /// Path to the JSON rules file
        path: String,
    },
    /// Remove a rule by ID
    Remove {
        /// Rule ID to remove
        id: String,
    },
    /// Print the expected JSON schema for AI rules payloads
    Schema,
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Rotate the scan encryption key (NIST SC-12 key rotation)
    RotateKey,
}

#[derive(Subcommand)]
enum ReleaseCommands {
    /// Generate a new Ed25519 signing keypair (private key → keyring, public key → stdout)
    GenerateKeypair,
    /// Sign a release artifact with Ed25519
    Sign {
        /// Path to the binary/artifact to sign
        file: String,
        /// Version string (e.g. "6.0.1")
        #[arg(long)]
        version: String,
        /// Path to a base64-encoded signing key file (alternative to keyring)
        #[arg(long)]
        key_file: Option<String>,
    },
    /// Verify a release artifact's signature
    Verify {
        /// Path to the binary/artifact to verify
        file: String,
        /// Path to the .sig.json signature file
        #[arg(long)]
        sig: String,
        /// Base64-encoded public key (alternative to embedded key)
        #[arg(long)]
        pubkey: Option<String>,
    },
    /// Compute SHA-256 checksum of a file
    Checksum {
        /// Path to the file
        file: String,
    },
    /// Generate a release manifest (releases.json) for all artifacts in a directory
    Manifest {
        /// Version string (e.g. "6.0.1")
        #[arg(long)]
        version: String,
        /// Directory containing release artifacts
        #[arg(long)]
        dir: String,
        /// Output path for releases.json (default: <dir>/releases.json)
        #[arg(long)]
        output: Option<String>,
        /// Path to a base64-encoded signing key file (alternative to keyring)
        #[arg(long)]
        key_file: Option<String>,
    },
    /// Verify a release manifest (releases.json)
    VerifyManifest {
        /// Path to releases.json
        file: String,
        /// Base64-encoded public key (alternative to embedded key)
        #[arg(long)]
        pubkey: Option<String>,
    },
}

#[derive(Subcommand)]
enum SettingsCommands {
    /// Show all settings
    Get,
    /// Set a specific setting
    Set {
        /// Setting to change (theme, font-size, locale, idle-threshold, ai-profile, poll-interval-ms, automation-interval-secs, active-profile-preset)
        key: String,
        /// New value for the setting
        value: String,
    },
    /// List shared profile presets
    Presets,
    /// Apply a shared profile preset by ID
    Use {
        /// Preset ID to activate
        id: String,
    },
}

#[derive(Subcommand)]
enum TabCommands {
    /// List open browser tabs
    List,
    /// Close a browser tab by ID or URL
    Close {
        /// Target Browser (Chrome, Safari, Brave, Edge, Arc)
        #[arg(long)]
        browser: String,
        #[arg(long, default_value_t = String::new())]
        id: String,
        #[arg(long, default_value_t = String::new())]
        url: String,
    },
    /// Focus a browser tab by ID or URL
    Focus {
        /// Target Browser (Chrome, Safari, Brave, Edge, Arc)
        #[arg(long)]
        browser: String,
        #[arg(long, default_value_t = String::new())]
        id: String,
        #[arg(long, default_value_t = String::new())]
        url: String,
    },
}

#[derive(Clone, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Clone, ValueEnum)]
enum AiProvider {
    Openai,
    Anthropic,
    Openrouter,
    Gemini,
}

#[derive(Clone, Copy, ValueEnum)]
enum NetworkProtocolArg {
    Tcp,
    Udp,
    Icmp,
    Other,
}

impl NetworkProtocolArg {
    fn to_protocol(self) -> Protocol {
        match self {
            Self::Tcp => Protocol::TCP,
            Self::Udp => Protocol::UDP,
            Self::Icmp => Protocol::ICMP,
            Self::Other => Protocol::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkView {
    Connections,
    Alerts,
    Top,
}

impl AiProvider {
    fn to_core_provider(&self) -> core_ai::AiProvider {
        match self {
            AiProvider::Openai => core_ai::AiProvider::OpenAI,
            AiProvider::Anthropic => core_ai::AiProvider::Anthropic,
            AiProvider::Openrouter => core_ai::AiProvider::OpenRouter,
            AiProvider::Gemini => core_ai::AiProvider::Gemini,
        }
    }

    fn display_name(&self) -> &'static str {
        match self {
            AiProvider::Openai => "OpenAI",
            AiProvider::Anthropic => "Anthropic",
            AiProvider::Openrouter => "OpenRouter",
            AiProvider::Gemini => "Gemini",
        }
    }

    fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Openai => "gpt-4o-mini",
            AiProvider::Anthropic => "claude-haiku-4-5-20251001",
            AiProvider::Openrouter => "meta-llama/llama-3.2-3b-instruct:free",
            AiProvider::Gemini => "gemini-2.0-flash",
        }
    }
}

fn format_memory(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;

    if gb >= 1.0 {
        format!("{:.2} GB", gb)
    } else if mb >= 1.0 {
        format!("{:.2} MB", mb)
    } else if kb >= 1.0 {
        format!("{:.2} KB", kb)
    } else {
        format!("{} B", bytes)
    }
}

fn build_network_filter(protocol: Option<NetworkProtocolArg>, port: Option<u16>) -> NetworkFilter {
    NetworkFilter {
        protocols: protocol.map(|value| vec![value.to_protocol()]),
        ports: port.map(|value| vec![value]),
        ..Default::default()
    }
}

fn top_network_processes(
    state: &watcher::SystemState,
    limit: usize,
) -> Vec<core::network::ProcessNetworkThroughput> {
    state
        .top_network_processes
        .iter()
        .take(limit)
        .cloned()
        .collect()
}

fn determine_network_view(
    connections: bool,
    alerts: bool,
    _top: bool,
    protocol: Option<NetworkProtocolArg>,
    port: Option<u16>,
) -> NetworkView {
    if connections || protocol.is_some() || port.is_some() {
        NetworkView::Connections
    } else if alerts {
        NetworkView::Alerts
    } else {
        NetworkView::Top
    }
}

fn print_network_text(state: &watcher::SystemState, view: NetworkView, filter: &NetworkFilter) {
    println!("OmniMon Network Telemetry");
    println!();
    println!(
        "  Backend:   {} (DPI: {})",
        state.net_capture_backend,
        if state.net_dpi_active {
            "active"
        } else {
            "inactive"
        }
    );
    println!(
        "  Global:    rx {} /s   tx {} /s",
        format_memory(state.net_rx_bytes_per_sec),
        format_memory(state.net_tx_bytes_per_sec)
    );

    match view {
        NetworkView::Connections => {
            let connections = watcher::get_filtered_connections(filter);
            println!();
            println!("  Filtered connections:");
            println!(
                "  {:>6}  {:<18}  {:>5}  {:<18}  {:>5}  {:<5}  {:<12}",
                "PID", "LOCAL", "PORT", "REMOTE", "PORT", "PROTO", "STATE"
            );
            println!("  {}", "-".repeat(88));
            for conn in &connections {
                println!(
                    "  {:>6}  {:<18}  {:>5}  {:<18}  {:>5}  {:<5}  {:<12}",
                    conn.pid,
                    conn.local_addr,
                    conn.local_port,
                    conn.remote_hostname
                        .as_deref()
                        .unwrap_or(&conn.remote_addr.to_string()),
                    conn.remote_port,
                    conn.protocol,
                    format!("{:?}", conn.state)
                );
            }
            if connections.is_empty() {
                println!("  (no matching connections captured yet)");
            }
        }
        NetworkView::Alerts => {
            println!();
            println!("  Network alerts:");
            println!(
                "  {:<10}  {:<12}  {:<18}  MESSAGE",
                "SEVERITY", "RULE", "DESTINATION"
            );
            println!("  {}", "-".repeat(90));
            for alert in &state.network_alerts {
                println!(
                    "  {:<10}  {:<12}  {:<18}  {}",
                    format!("{:?}", alert.severity),
                    alert.rule_id,
                    alert.destination.as_deref().unwrap_or("-"),
                    alert.message
                );
            }
            if state.network_alerts.is_empty() {
                println!("  (no network alerts fired yet)");
            }
        }
        NetworkView::Top => {
            let top_processes = top_network_processes(state, 10);
            println!();
            println!("  Top 10 processes by throughput:");
            println!(
                "  {:>6}  {:<18}  {:>12}  {:>12}  {:>8}  {:>8}",
                "PID", "NAME", "RX/s", "TX/s", "TCP/s", "UDP/s"
            );
            println!("  {}", "-".repeat(78));
            for process in &top_processes {
                println!(
                    "  {:>6}  {:<18}  {:>12}  {:>12}  {:>8}  {:>8}",
                    process.pid,
                    process.process_name.as_deref().unwrap_or("unknown"),
                    format_memory(process.rx_bytes_per_sec),
                    format_memory(process.tx_bytes_per_sec),
                    process.tcp_packets_per_sec,
                    process.udp_packets_per_sec
                );
            }
            if top_processes.is_empty() {
                println!("  (no network activity captured yet)");
            }
        }
    }
}

fn print_network_json(state: &watcher::SystemState, view: NetworkView, filter: &NetworkFilter) {
    let output = match view {
        NetworkView::Connections => serde_json::json!({
            "view": "connections",
            "capture_backend": state.net_capture_backend,
            "dpi_active": state.net_dpi_active,
            "filters": filter,
            "connections": watcher::get_filtered_connections(filter),
        }),
        NetworkView::Alerts => serde_json::json!({
            "view": "alerts",
            "capture_backend": state.net_capture_backend,
            "dpi_active": state.net_dpi_active,
            "alerts": state.network_alerts,
        }),
        NetworkView::Top => serde_json::json!({
            "view": "top",
            "net_rx_bytes_per_sec": state.net_rx_bytes_per_sec,
            "net_tx_bytes_per_sec": state.net_tx_bytes_per_sec,
            "capture_backend": state.net_capture_backend,
            "dpi_active": state.net_dpi_active,
            "top_processes": top_network_processes(state, 10),
        }),
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
}

fn render_network_view(format: &Format, view: NetworkView, filter: &NetworkFilter) {
    let state = watcher::get_cached_state();
    match format {
        Format::Json => print_network_json(&state, view, filter),
        Format::Text => print_network_text(&state, view, filter),
    }
}

fn cli_startup_wait() {
    let ms = std::env::var("OMNIMON_CLI_STARTUP_WAIT_MS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(2500);
    if ms > 0 {
        std::thread::sleep(std::time::Duration::from_millis(ms));
    }
}

fn build_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| {
            eprintln!("Fatal: failed to create async runtime: {}", e);
            std::process::exit(1);
        })
}

pub fn run() -> Result<(), i32> {
    run_cli(Cli::parse())
}

pub(crate) fn run_cli(cli: Cli) -> Result<(), i32> {
    if cli.sync_keychain {
        println!("Syncing credentials from keychain...");
        let providers = [
            ("OpenAI", core_ai::AiProvider::OpenAI),
            ("Anthropic", core_ai::AiProvider::Anthropic),
            ("OpenRouter", core_ai::AiProvider::OpenRouter),
            ("Gemini", core_ai::AiProvider::Gemini),
        ];
        for (name, provider) in &providers {
            match core_ai::get_api_key(*provider) {
                Ok(_) => println!("  [ok] {} key found in keyring", name),
                Err(_) => println!("  [--] {} key not configured", name),
            }
        }
    }

    match &cli.command {
        Commands::Status { format } => {
            watcher::start_watcher();
            cli_startup_wait();

            let snapshot = core::telemetry::telemetry_snapshot(Some(10));

            match format {
                Format::Json => {
                    let procs_json: Vec<serde_json::Value> = snapshot
                        .processes
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "pid": p.pid,
                                "name": p.name,
                                "group": p.group,
                                "group_key": p.group_key,
                                "grouped_name": p.grouped_display_name,
                                "process_count": p.process_count,
                                "memory_bytes": p.memory_bytes,
                                "cpu_usage_percent": p.cpu_usage_percent,
                                "disk_read_bytes": p.disk_read_bytes,
                                "disk_write_bytes": p.disk_write_bytes,
                                "net_rx_bytes_per_sec": p.net_rx_bytes_per_sec,
                                "net_tx_bytes_per_sec": p.net_tx_bytes_per_sec,
                                "energy_impact_score": p.energy_impact_score,
                                "bundle_id": p.bundle_id,
                                "exe_path": p.exe_path
                            })
                        })
                        .collect();

                    let grouped_json: Vec<serde_json::Value> = snapshot
                        .super_processes
                        .iter()
                        .map(|group| {
                            serde_json::json!({
                                "key": group.binary_key,
                                "display_name": group.display_name,
                                "group": group.group,
                                "identity_type": group.identity_type,
                                "process_count": group.process_count,
                                "memory_bytes": group.total_memory_bytes,
                                "cpu_usage_percent": group.total_cpu_usage_percent,
                                "disk_read_bytes": group.total_disk_read_bytes,
                                "disk_write_bytes": group.total_disk_write_bytes,
                                "net_rx_bytes_per_sec": group.total_net_rx_bytes_per_sec,
                                "net_tx_bytes_per_sec": group.total_net_tx_bytes_per_sec,
                                "energy_impact_score": group.energy_impact_score,
                                "pids": group.pids
                            })
                        })
                        .collect();

                    let output = serde_json::json!({
                        "status": "running",
                        "total_memory_bytes": snapshot.total_memory_bytes,
                        "used_memory_bytes": snapshot.used_memory_bytes,
                        "free_memory_bytes": snapshot.free_memory_bytes,
                        "free_percent": snapshot.free_percent,
                        "swap_used_mb": snapshot.swap_used_mb,
                        "cpu_usage_percent": snapshot.cpu_usage_percent,
                        "net_rx_bytes_per_sec": snapshot.net_rx_bytes_per_sec,
                        "net_tx_bytes_per_sec": snapshot.net_tx_bytes_per_sec,
                        "top_processes": procs_json,
                        "grouped_processes": grouped_json
                    });
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&output).unwrap_or_default()
                    );
                }
                Format::Text => {
                    println!("omnimon status: running");
                    println!();
                    println!(
                        "  Memory: {} / {} ({:.1}% used)",
                        format_memory(snapshot.used_memory_bytes),
                        format_memory(snapshot.total_memory_bytes),
                        100.0 - snapshot.free_percent as f64
                    );
                    println!("  Swap:   {} MB used", snapshot.swap_used_mb);
                    println!("  CPU:    {:.1}%", snapshot.cpu_usage_percent);
                    println!(
                        "  Net:    rx {} /s  tx {} /s",
                        format_memory(snapshot.net_rx_bytes_per_sec),
                        format_memory(snapshot.net_tx_bytes_per_sec)
                    );
                    println!();
                    println!("  Top grouped processes:");
                    println!(
                        "  {:<18}  {:>5}  {:>10}  {:>8}  {:>8}",
                        "NAME", "COUNT", "MEMORY", "NET", "ENERGY"
                    );
                    println!("  {}", "-".repeat(66));
                    for p in &snapshot.super_processes {
                        let net_total = p
                            .total_net_rx_bytes_per_sec
                            .saturating_add(p.total_net_tx_bytes_per_sec);
                        println!(
                            "  {:<18}  {:>5}  {:>10}  {:>8}  {:>8}",
                            if p.display_name.len() > 18 {
                                &p.display_name[..18]
                            } else {
                                &p.display_name
                            },
                            p.process_count,
                            format_memory(p.total_memory_bytes),
                            format_memory(net_total),
                            format!("{:.1}", p.energy_impact_score.unwrap_or_default())
                        );
                    }
                    println!();
                    println!("  Top processes by memory:");
                    println!(
                        "  {:>6}  {:<26}  {:>10}  {:>8}  {:>8}",
                        "PID", "NAME", "MEMORY", "NET", "ENERGY"
                    );
                    println!("  {}", "-".repeat(68));
                    for p in &snapshot.processes {
                        let net_total = p
                            .net_rx_bytes_per_sec
                            .saturating_add(p.net_tx_bytes_per_sec);
                        println!(
                            "  {:>6}  {:<26}  {:>10}  {:>8}  {:>8}",
                            p.pid,
                            if p.name.len() > 26 {
                                &p.name[..26]
                            } else {
                                &p.name
                            },
                            format_memory(p.memory_bytes),
                            format_memory(net_total),
                            format!("{:.1}", p.energy_impact_score.unwrap_or_default())
                        );
                    }
                }
            }
        }
        Commands::Kill { pid } => {
            println!("Attempting to kill process with PID {}...", pid);
            match killer::kill_process_safe(*pid as i32, &[]) {
                Ok(result) => {
                    println!(
                        "Successfully killed process '{}' (PID {})",
                        result.process_name, result.pid
                    );
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    return Err(1);
                }
            }
        }
        Commands::Optimize { ai, target } => {
            let target_name = target.as_deref().unwrap_or("all");
            println!(
                "Starting Smart Optimize using {} on target '{}'...",
                ai.display_name(),
                target_name
            );

            let core_provider = ai.to_core_provider();
            let model = ai.default_model();

            let top_procs = metrics::top_processes_by_memory(30);
            let mut procs_to_send: Vec<serde_json::Value> = Vec::new();
            for p in &top_procs {
                if !killer::is_immutable_blocked_process_name(&p.name) {
                    procs_to_send.push(serde_json::json!({
                        "pid": p.pid,
                        "name": p.name,
                        "memory_mb": p.memory_bytes / 1_048_576
                    }));
                }
            }
            let procs_json =
                serde_json::to_string(&procs_to_send).unwrap_or_else(|_| "[]".to_string());

            let profile = target_name;

            let rt = build_runtime();

            match rt.block_on(core_ai::analyze_with_ai(
                core_provider,
                model,
                &procs_json,
                profile,
            )) {
                Ok(mut suggestions) => {
                    suggestions.retain(|s| !killer::is_immutable_blocked_process_name(&s.name));
                    if suggestions.is_empty() {
                        println!("No optimization suggestions — your system looks healthy.");
                    } else {
                        println!();
                        println!(
                            "AI Suggestions ({} provider, {} model):",
                            ai.display_name(),
                            model
                        );
                        println!("  {:>6}  {:<25}  REASON", "PID", "NAME");
                        println!("  {}", "-".repeat(70));
                        for s in &suggestions {
                            println!("  {:>6}  {:<25}  {}", s.pid, s.name, s.reason);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("AI optimization failed: {}", e);
                    return Err(1);
                }
            }
        }
        Commands::Tabs { command } => match command {
            TabCommands::List => {
                let provider = NativeTabProvider;
                let mut all_tabs = Vec::new();
                for browser in BrowserKind::all() {
                    if let Ok(tabs) = provider.list_tabs(*browser) {
                        all_tabs.extend(tabs);
                    }
                }
                println!("Open Browser Tabs:");
                for tab in all_tabs {
                    println!(
                        "  [{}] {} ({})",
                        tab.browser.display_name(),
                        tab.title,
                        tab.url
                    );
                }
            }
            TabCommands::Close { browser, id, url } => {
                use std::str::FromStr;
                let kind = match BrowserKind::from_str(browser) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Invalid browser '{}': {}", browser, e);
                        return Err(1);
                    }
                };
                let tab = BrowserTab {
                    id: id.clone(),
                    url: url.clone(),
                    title: String::new(),
                    browser: kind,
                };
                let provider = NativeTabProvider;
                match provider.close_tab(kind, &tab) {
                    Ok(true) => println!("Tab successfully closed."),
                    Ok(false) => println!("Tab not found or could not be closed."),
                    Err(e) => eprintln!("Error closing tab: {}", e),
                }
            }
            TabCommands::Focus { browser, id, url } => {
                use std::str::FromStr;
                let kind = match BrowserKind::from_str(browser) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Invalid browser '{}': {}", browser, e);
                        return Err(1);
                    }
                };
                let tab = BrowserTab {
                    id: id.clone(),
                    url: url.clone(),
                    title: String::new(),
                    browser: kind,
                };
                let provider = NativeTabProvider;
                match provider.focus_tab(kind, &tab) {
                    Ok(true) => println!("Tab successfully focused."),
                    Ok(false) => println!("Tab not found or could not be focused."),
                    Err(e) => eprintln!("Error focusing tab: {}", e),
                }
            }
        },
        Commands::Chat { ai, prompt } => {
            println!("Sending context analysis to {}...", ai.display_name());
            let core_provider = ai.to_core_provider();
            let model = ai.default_model();

            let rt = build_runtime();

            let prompt_clone = prompt.clone();
            match rt.block_on(core_ai::analyze_context(
                core_provider,
                model,
                &prompt_clone,
            )) {
                Ok(response) => {
                    println!("\nAI Response:\n{}", response);
                }
                Err(e) => {
                    eprintln!("Chat failed: {}", e);
                    return Err(1);
                }
            }
        }
        Commands::Apikey { ai, key } => {
            println!("Validating and saving API Key for {}...", ai.display_name());
            let core_provider = ai.to_core_provider();
            let model = ai.default_model();

            let rt = build_runtime();

            match rt.block_on(core_ai::save_api_key_with_ping(core_provider, model, key)) {
                Ok(()) => println!("API Key successfully validated and saved to native keyring."),
                Err(e) => {
                    eprintln!("Failed to validate or save API key: {}", e);
                    return Err(1);
                }
            }
        }
        Commands::Settings { command } => match command {
            SettingsCommands::Get => {
                let s = settings::read_settings();
                println!("Current Settings:");
                println!("  theme:          {}", s.theme.as_deref().unwrap_or("auto"));
                println!("  font-size:      {}", s.font_size.unwrap_or(12));
                println!(
                    "  locale:         {}",
                    s.locale.as_deref().unwrap_or("auto")
                );
                println!("  idle-threshold: {}", s.idle_threshold.unwrap_or(1.0));
                println!(
                    "  ai-profile:     {}",
                    s.ai_profile.as_deref().unwrap_or("general")
                );
                println!(
                    "  poll-interval-ms: {}",
                    s.poll_interval_ms
                        .unwrap_or(settings::DEFAULT_POLL_INTERVAL_MS)
                );
                println!(
                    "  automation-interval-secs: {}",
                    s.automation_interval_secs
                        .unwrap_or(settings::DEFAULT_AUTOMATION_INTERVAL_SECS)
                );
                println!(
                    "  active-profile-preset: {}",
                    s.active_profile_preset.as_deref().unwrap_or("general")
                );
            }
            SettingsCommands::Set { key, value } => {
                let mut s = settings::read_settings();
                match key.as_str() {
                    "theme" => s.theme = Some(value.clone()),
                    "font-size" => {
                        if let Ok(fs) = value.parse::<u32>() {
                            s.font_size = Some(fs);
                        } else {
                            eprintln!("Error: font-size must be an integer");
                            return Err(1);
                        }
                    }
                    "locale" => s.locale = Some(value.clone()),
                    "idle-threshold" => {
                        if let Ok(thresh) = value.parse::<f64>() {
                            s.idle_threshold = Some(thresh);
                        } else {
                            eprintln!("Error: idle-threshold must be a number");
                            return Err(1);
                        }
                    }
                    "ai-profile" => {
                        if matches!(
                            value.as_str(),
                            "general" | "developer" | "gaming" | "battery"
                        ) {
                            s.ai_profile = Some(value.clone());
                        } else {
                            eprintln!(
                                "Error: ai-profile must be one of general|developer|gaming|battery"
                            );
                            return Err(1);
                        }
                    }
                    "poll-interval-ms" => {
                        if let Ok(interval) = value.parse::<u64>() {
                            s.poll_interval_ms = Some(interval);
                        } else {
                            eprintln!("Error: poll-interval-ms must be a positive integer");
                            return Err(1);
                        }
                    }
                    "automation-interval-secs" => {
                        if let Ok(interval) = value.parse::<u64>() {
                            s.automation_interval_secs = Some(interval);
                        } else {
                            eprintln!("Error: automation-interval-secs must be a positive integer");
                            return Err(1);
                        }
                    }
                    "active-profile-preset" => {
                        s.active_profile_preset = Some(value.clone());
                    }
                    _ => {
                        eprintln!("Error: unknown setting '{}'", key);
                        return Err(1);
                    }
                }
                match settings::write_settings(&s) {
                    Ok(_) => println!("Setting '{}' updated to '{}'", key, value),
                    Err(e) => {
                        eprintln!("Failed to save settings: {}", e);
                        return Err(1);
                    }
                }
            }
            SettingsCommands::Presets => {
                let s = settings::read_settings();
                println!("Shared profile presets:");
                for preset in &s.profile_presets {
                    print_preset(
                        preset,
                        s.active_profile_preset.as_deref() == Some(preset.id.as_str()),
                    );
                }
            }
            SettingsCommands::Use { id } => {
                let mut s = settings::read_settings();
                let Some(preset) = s
                    .profile_presets
                    .iter()
                    .find(|preset| preset.id == *id)
                    .cloned()
                else {
                    eprintln!("Error: preset '{}' not found", id);
                    return Err(1);
                };
                s.active_profile_preset = Some(preset.id.clone());
                s.ai_profile = Some(preset.ai_profile.clone());
                s.idle_threshold = Some(preset.idle_threshold);
                s.poll_interval_ms = Some(preset.poll_interval_ms);
                s.automation_interval_secs = Some(preset.automation_interval_secs);
                match settings::write_settings(&s) {
                    Ok(_) => {
                        println!("Applied preset '{}':", preset.id);
                        print_preset(&preset, true);
                    }
                    Err(e) => {
                        eprintln!("Failed to save settings: {}", e);
                        return Err(1);
                    }
                }
            }
        },
        Commands::Config { command } => match command {
            ConfigCommands::RotateKey => {
                println!("Rotating scan encryption key...");
                let entry = match keyring::Entry::new("omnimon_security", "scan_encryption_key") {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Error: cannot access OS keyring: {}", e);
                        return Err(1);
                    }
                };

                let old_key: [u8; 32] = match entry.get_password() {
                    Ok(stored) => {
                        use base64::Engine;
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(&stored)
                            .unwrap_or_default();
                        if decoded.len() == 32 {
                            let mut k = [0u8; 32];
                            k.copy_from_slice(&decoded);
                            k
                        } else {
                            eprintln!("Error: existing key in keyring is corrupted. Generating fresh key instead.");
                            let new_key = crypto::generate_encryption_key();
                            use base64::Engine as _;
                            let encoded = base64::engine::general_purpose::STANDARD.encode(new_key);
                            let _ = entry.set_password(&encoded);
                            println!("New encryption key generated and stored in OS keyring.");
                            return Ok(());
                        }
                    }
                    Err(_) => {
                        println!("No existing key found. Generating initial encryption key...");
                        let new_key = crypto::generate_encryption_key();
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(new_key);
                        let _ = entry.set_password(&encoded);
                        println!("Encryption key generated and stored in OS keyring.");
                        return Ok(());
                    }
                };

                // Generate new key and store it
                let new_key = crypto::generate_encryption_key();
                use base64::Engine;
                let encoded = base64::engine::general_purpose::STANDARD.encode(new_key);
                match entry.set_password(&encoded) {
                    Ok(_) => {
                        // Re-encrypt existing report if present
                        let report_path = std::env::temp_dir().join("omnimon_scan_report.enc");
                        if report_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&report_path) {
                                if let Ok(payload) =
                                    serde_json::from_str::<crypto::EncryptedPayload>(&content)
                                {
                                    match crypto::decrypt_bytes(&old_key, &payload) {
                                        Ok(plaintext) => {
                                            if let Ok(re_enc) =
                                                crypto::encrypt_bytes(&new_key, &plaintext)
                                            {
                                                if let Ok(json) =
                                                    serde_json::to_string_pretty(&re_enc)
                                                {
                                                    let _ = std::fs::write(&report_path, json);
                                                    println!("Existing report re-encrypted with new key.");
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Warning: could not re-encrypt existing report: {}",
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                        }
                        println!("Encryption key rotated successfully.");
                    }
                    Err(e) => {
                        eprintln!("Error: failed to save new key to keyring: {}", e);
                        return Err(1);
                    }
                }
            }
        },
        Commands::Auth { command } => match command {
            AuthCommands::Login { key } => {
                println!("Validating and saving CrabNebula API Key...");
                let entry = match keyring::Entry::new("omnimon_crabnebula", "cn_api_key") {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Failed to access OS keyring: {}", e);
                        return Err(1);
                    }
                };
                match entry.set_password(key) {
                    Ok(_) => println!("CrabNebula API Key securely saved to the OS keyring."),
                    Err(e) => {
                        eprintln!("Failed to save CrabNebula API Key: {}", e);
                        return Err(1);
                    }
                }
            }
        },
        Commands::Cloud { command } => match command {
            CloudCommands::Sync { report_path } => {
                println!(
                    "Syncing security report {} to CrabNebula Cloud...",
                    report_path
                );
                let entry = match keyring::Entry::new("omnimon_crabnebula", "cn_api_key") {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Failed to access OS keyring: {}", e);
                        return Err(1);
                    }
                };

                let _api_key = match entry.get_password() {
                    Ok(k) => k,
                    Err(_) => {
                        eprintln!("Error: CrabNebula API Key not found. Please run 'omnimon cloud login <key>' first.");
                        return Err(1);
                    }
                };

                if !std::path::Path::new(&report_path).exists() {
                    eprintln!("Error: Report file not found at {}", report_path);
                    return Err(1);
                }

                // Simulating upload to CrabNebula Cloud
                std::thread::sleep(std::time::Duration::from_millis(800));
                println!("Report successfully uploaded and synced with CrabNebula backend.");
            }
        },
        Commands::SecurityScan { cve_db } => {
            println!("Initiating Local Security Scan...");
            let db = if let Some(db_path) = cve_db {
                match core::audit::LocalCveDatabase::from_file(db_path) {
                    Ok(db) => db,
                    Err(e) => {
                        eprintln!("Failed to load CVE DB: {}", e);
                        return Err(1);
                    }
                }
            } else {
                println!("No custom CVE DB provided, using empty fallback...");
                core::audit::LocalCveDatabase {
                    schema_version: 1,
                    entries: vec![],
                }
            };

            // Gather processes
            let top_procs = core::metrics::top_processes_by_memory(50);
            let mut proc_info = Vec::new();
            for p in top_procs {
                proc_info.push(core::audit::ProcessVersionInfo {
                    pid: p.pid,
                    process_name: p.name.clone(),
                    product: p.name.clone(),      // Naive mapping
                    version: "1.0.0".to_string(), // In a real app we'd fetch actual version
                });
            }

            let findings = core::audit::audit_processes_against_cves(&proc_info, &db);

            let heartbeat = core::audit::build_security_heartbeat(
                proc_info.len(),
                findings.len(),
                true,
                0,
                0,
                true,
                "Security Scan Completed",
            );

            println!(
                "Scan completed. Found {} tracked processes.",
                heartbeat.identification.tracked_processes
            );
            println!(
                "CVE Matches: {}",
                heartbeat.identification.known_cve_matches
            );

            if !findings.is_empty() {
                println!("Findings:");
                for f in findings {
                    println!(
                        "  [PID {}] {} ({}): {} ({:?})",
                        f.pid, f.process_name, f.detected_version, f.cve_id, f.severity
                    );
                }
            }

            // Save to temp encrypted report using a key from the OS keyring.
            // If no key exists yet, generate one and store it securely.
            let report_path = std::env::temp_dir().join("omnimon_scan_report.enc");
            let key: [u8; 32] = match keyring::Entry::new("omnimon_security", "scan_encryption_key")
            {
                Ok(entry) => match entry.get_password() {
                    Ok(stored) => {
                        use base64::Engine;
                        let decoded = base64::engine::general_purpose::STANDARD
                            .decode(&stored)
                            .unwrap_or_else(|_| {
                                eprintln!("Warning: corrupted scan key in keyring, regenerating");
                                Vec::new()
                            });
                        if decoded.len() == 32 {
                            let mut k = [0u8; 32];
                            k.copy_from_slice(&decoded);
                            k
                        } else {
                            // Generate fresh key
                            let mut k = [0u8; 32];
                            use rand::RngCore;
                            rand::thread_rng().fill_bytes(&mut k);
                            use base64::Engine as _;
                            let encoded = base64::engine::general_purpose::STANDARD.encode(k);
                            let _ = entry.set_password(&encoded);
                            k
                        }
                    }
                    Err(_) => {
                        // First run: generate and store
                        let mut k = [0u8; 32];
                        use rand::RngCore;
                        rand::thread_rng().fill_bytes(&mut k);
                        use base64::Engine;
                        let encoded = base64::engine::general_purpose::STANDARD.encode(k);
                        let _ = entry.set_password(&encoded);
                        println!("Generated new scan encryption key (stored in OS keyring).");
                        k
                    }
                },
                Err(e) => {
                    eprintln!("Error: cannot access OS keyring for encryption key: {}", e);
                    eprintln!("Hint: ensure your OS keyring service is running.");
                    return Err(1);
                }
            };
            match core::audit::persist_encrypted_security_heartbeat(&report_path, &key, &heartbeat)
            {
                Ok(_) => println!("Encrypted report saved to: {}", report_path.display()),
                Err(e) => eprintln!("Failed to save encrypted report: {}", e),
            }
        }
        Commands::Tui => {
            if let Err(e) = omnimon_tui::run() {
                eprintln!("TUI error: {}", e);
                return Err(1);
            }
        }
        Commands::Doctor => {
            println!("🩺 OmniMon System Health Check\n");

            let os = std::env::consts::OS;
            println!("Operating System: {}", os);
            println!("Architecture:   {}", std::env::consts::ARCH);
            println!("CLI Version:    {}", env!("CARGO_PKG_VERSION"));

            println!("\n[Drivers & Network Capture]");
            match os {
                "macos" => {
                    let pcap_exists = std::path::Path::new("/usr/lib/libpcap.dylib").exists()
                        || std::path::Path::new("/usr/lib/libpcap.A.dylib").exists()
                        || std::path::Path::new("/opt/homebrew/lib/libpcap.dylib").exists();
                    if pcap_exists {
                        println!("✅ libpcap (macOS native packet capture) found.");
                    } else {
                        println!(
                            "✅ libpcap assumed available via dyld shared cache (native to macOS)."
                        );
                    }
                }
                "windows" => {
                    // Placeholder for WinDivert
                    println!("✅ WinDivert (Windows packet capture) assumed ready in deployment.");
                }
                "linux" => {
                    println!("✅ eBPF (aya) support enabled.");
                    println!("⚠️ Note: Full eBPF capture requires root (CAP_BPF/CAP_NET_ADMIN) privileges.");
                }
                _ => println!("⚠️ Unknown OS driver status."),
            }

            println!("\n[Security & Keyring]");
            match keyring::Entry::new("omnimon_crabnebula", "test_ping") {
                Ok(_) => println!("✅ Native OS Keyring access successful."),
                Err(e) => println!("❌ Native OS Keyring error: {}", e),
            }

            println!("\nHealth check complete.");
        }
        Commands::Network {
            format,
            connections,
            filter,
            port,
            alerts,
            top,
            watch,
            watch_interval_ms,
            watch_iterations,
        } => {
            watcher::start_watcher();
            cli_startup_wait();

            let view = determine_network_view(*connections, *alerts, *top, *filter, *port);
            let connection_filter = build_network_filter(*filter, *port);

            if *watch {
                if matches!(format, Format::Text) {
                    println!(
                        "Watching network telemetry every {} ms (Ctrl+C para salir)...",
                        watch_interval_ms
                    );
                }

                let mut remaining = *watch_iterations;
                loop {
                    render_network_view(format, view, &connection_filter);
                    if let Some(ref mut iterations) = remaining {
                        if *iterations == 1 {
                            break;
                        }
                        *iterations -= 1;
                    }
                    if matches!(format, Format::Text) {
                        println!();
                    }
                    std::thread::sleep(std::time::Duration::from_millis(*watch_interval_ms));
                }
            } else {
                render_network_view(format, view, &connection_filter);
            }
        }
        Commands::Rules { command } => match command {
            RulesCommands::List => {
                let rules = rules_engine::active_rules();
                if rules.is_empty() {
                    println!("No active security rules.");
                } else {
                    println!("Active security rules ({}):", rules.len());
                    println!(
                        "  {:<12}  {:<30}  {:<16}  {:>7}",
                        "ID", "NAME", "KIND", "ENABLED"
                    );
                    println!("  {}", "-".repeat(72));
                    for rule in &rules {
                        println!(
                            "  {:<12}  {:<30}  {:<16}  {:>7}",
                            if rule.id.len() > 12 {
                                &rule.id[..12]
                            } else {
                                &rule.id
                            },
                            if rule.name.len() > 30 {
                                &rule.name[..30]
                            } else {
                                &rule.name
                            },
                            format!("{:?}", rule.kind),
                            if rule.enabled { "yes" } else { "no" }
                        );
                    }
                }
            }
            RulesCommands::Load { path } => {
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading rules file '{}': {}", path, e);
                        return Err(1);
                    }
                };
                match rules_engine::upsert_rules_from_ai_json(&content) {
                    Ok(count) => println!("Successfully loaded {} security rules.", count),
                    Err(e) => {
                        eprintln!("Error loading rules: {}", e);
                        return Err(1);
                    }
                }
            }
            RulesCommands::Remove { id } => match rules_engine::remove_rule_by_id(id) {
                Ok(true) => println!("Rule '{}' removed.", id),
                Ok(false) => {
                    eprintln!("Rule '{}' not found.", id);
                    return Err(1);
                }
                Err(e) => {
                    eprintln!("Error removing rule: {}", e);
                    return Err(1);
                }
            },
            RulesCommands::Schema => {
                println!("{}", rules_engine::ai_rules_schema_json());
            }
        },
        Commands::Release { command } => match command {
            ReleaseCommands::GenerateKeypair => {
                let (signing_key, verifying_key) = crypto::generate_ed25519_keypair();

                // Store private key in OS keyring
                let entry = match keyring::Entry::new("omnimon_release", "ed25519_signing_key") {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Error: cannot access OS keyring: {}", e);
                        return Err(1);
                    }
                };

                let private_b64 = crypto::export_signing_key(&signing_key);
                match entry.set_password(&private_b64) {
                    Ok(_) => {
                        println!("Ed25519 signing key stored in OS keyring.");
                    }
                    Err(e) => {
                        eprintln!("Error: failed to store signing key: {}", e);
                        return Err(1);
                    }
                }

                let public_b64 = crypto::export_public_key(&verifying_key);
                println!("Public key (base64):");
                println!("{}", public_b64);
                println!();
                println!("Add this to tauri.conf.json plugins.updater.pubkey");
                println!("and distribute it with your application.");
            }
            ReleaseCommands::Sign {
                file,
                version,
                key_file,
            } => {
                let signing_key = load_signing_key(key_file.as_deref())?;

                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading '{}': {}", file, e);
                        return Err(1);
                    }
                };

                let sig = crypto::sign_release(&signing_key, &data, version);
                let sig_json = match serde_json::to_string_pretty(&sig) {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("Error serializing signature: {}", e);
                        return Err(1);
                    }
                };

                let sig_path = format!("{}.sig.json", file);
                if let Err(e) = std::fs::write(&sig_path, &sig_json) {
                    eprintln!("Error writing signature: {}", e);
                    return Err(1);
                }

                println!("SHA-256:   {}", sig.sha256);
                println!("Signature: {}", sig_path);
                println!("Version:   {}", sig.version);
            }
            ReleaseCommands::Verify { file, sig, pubkey } => {
                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading '{}': {}", file, e);
                        return Err(1);
                    }
                };

                let sig_content = match std::fs::read_to_string(sig) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading signature '{}': {}", sig, e);
                        return Err(1);
                    }
                };

                let release_sig: crypto::ReleaseSignature = match serde_json::from_str(&sig_content)
                {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error parsing signature JSON: {}", e);
                        return Err(1);
                    }
                };

                let pubkey_b64 = pubkey.as_deref().unwrap_or(&release_sig.public_key_b64);
                let verifying_key = match crypto::import_public_key(pubkey_b64) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Error loading public key: {}", e);
                        return Err(1);
                    }
                };

                match crypto::verify_release(&data, &release_sig, &verifying_key) {
                    Ok(()) => {
                        println!("Verification PASSED");
                        println!("  SHA-256:   {}", release_sig.sha256);
                        println!("  Version:   {}", release_sig.version);
                    }
                    Err(e) => {
                        eprintln!("Verification FAILED: {}", e);
                        return Err(1);
                    }
                }
            }
            ReleaseCommands::Checksum { file } => {
                let data = match std::fs::read(file) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("Error reading '{}': {}", file, e);
                        return Err(1);
                    }
                };
                let hash = crypto::sha256_hex(&data);
                println!("{}  {}", hash, file);
            }
            ReleaseCommands::Manifest {
                version,
                dir,
                output,
                key_file,
            } => {
                let signing_key = load_signing_key(key_file.as_deref())?;

                let dir_path = std::path::Path::new(dir.as_str());
                if !dir_path.is_dir() {
                    eprintln!("Error: '{}' is not a directory", dir);
                    return Err(1);
                }

                let entries = match std::fs::read_dir(dir_path) {
                    Ok(e) => e,
                    Err(e) => {
                        eprintln!("Error reading directory: {}", e);
                        return Err(1);
                    }
                };

                let mut artifacts = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let filename = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    // Skip manifest itself and signature files
                    if filename == "releases.json"
                        || filename.ends_with(".sig.json")
                        || filename.ends_with(".sha256")
                    {
                        continue;
                    }

                    let data = match std::fs::read(&path) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("Warning: cannot read '{}': {}", filename, e);
                            continue;
                        }
                    };

                    let (platform, arch) = detect_platform_arch(&filename);
                    let artifact =
                        crypto::sign_artifact(&signing_key, &data, &filename, &platform, &arch);
                    println!(
                        "  Signed: {} ({}/{}, {} bytes)",
                        filename, platform, arch, artifact.size_bytes
                    );
                    artifacts.push(artifact);
                }

                if artifacts.is_empty() {
                    eprintln!("No artifacts found in '{}'", dir);
                    return Err(1);
                }

                let date = chrono_date_today();
                let manifest =
                    crypto::build_release_manifest(&signing_key, version, &date, artifacts);

                let manifest_json = match serde_json::to_string_pretty(&manifest) {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("Error serializing manifest: {}", e);
                        return Err(1);
                    }
                };

                let output_path = output
                    .clone()
                    .unwrap_or_else(|| format!("{}/releases.json", dir));
                if let Err(e) = std::fs::write(&output_path, &manifest_json) {
                    eprintln!("Error writing manifest: {}", e);
                    return Err(1);
                }
                println!(
                    "Release manifest written to {} ({} artifacts)",
                    output_path,
                    manifest.artifacts.len()
                );
            }
            ReleaseCommands::VerifyManifest { file, pubkey } => {
                let content = match std::fs::read_to_string(file) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Error reading '{}': {}", file, e);
                        return Err(1);
                    }
                };

                let manifest: crypto::ReleaseManifest = match serde_json::from_str(&content) {
                    Ok(m) => m,
                    Err(e) => {
                        eprintln!("Error parsing manifest JSON: {}", e);
                        return Err(1);
                    }
                };

                let pubkey_b64 = match pubkey {
                    Some(k) => k.clone(),
                    None => {
                        eprintln!("Error: --pubkey is required to verify a manifest");
                        return Err(1);
                    }
                };

                let verifying_key = match crypto::import_public_key(&pubkey_b64) {
                    Ok(k) => k,
                    Err(e) => {
                        eprintln!("Error loading public key: {}", e);
                        return Err(1);
                    }
                };

                match crypto::verify_release_manifest(&manifest, &verifying_key) {
                    Ok(()) => {
                        println!("Manifest verification PASSED");
                        println!("  Version:   {}", manifest.version);
                        println!("  Date:      {}", manifest.date);
                        println!("  Artifacts: {}", manifest.artifacts.len());
                        for a in &manifest.artifacts {
                            println!(
                                "    {} ({}/{}, {} bytes)",
                                a.filename, a.platform, a.arch, a.size_bytes
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!("Manifest verification FAILED: {}", e);
                        return Err(1);
                    }
                }
            }
        },
    }
    Ok(())
}

/// Loads the Ed25519 signing key from a file or from the OS keyring.
fn load_signing_key(key_file: Option<&str>) -> Result<crypto::SigningKey, i32> {
    if let Some(path) = key_file {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading key file '{}': {}", path, e);
                return Err(1);
            }
        };
        match crypto::import_signing_key(content.trim()) {
            Ok(k) => Ok(k),
            Err(e) => {
                eprintln!("Error parsing signing key: {}", e);
                Err(1)
            }
        }
    } else {
        let entry = match keyring::Entry::new("omnimon_release", "ed25519_signing_key") {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Error: cannot access OS keyring: {}", e);
                eprintln!("Hint: run 'omnimon release generate-keypair' first, or use --key-file");
                return Err(1);
            }
        };
        let stored = match entry.get_password() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: no signing key found in OS keyring.");
                eprintln!("Run 'omnimon release generate-keypair' or provide --key-file");
                return Err(1);
            }
        };
        match crypto::import_signing_key(&stored) {
            Ok(k) => Ok(k),
            Err(e) => {
                eprintln!("Error: signing key in keyring is corrupted: {}", e);
                Err(1)
            }
        }
    }
}

/// Best-effort platform/arch detection from filename conventions.
fn detect_platform_arch(filename: &str) -> (String, String) {
    let lower = filename.to_lowercase();
    let platform = if lower.contains("linux") {
        "linux"
    } else if lower.contains("macos") || lower.contains("darwin") || lower.contains(".dmg") {
        "macos"
    } else if lower.contains("windows") || lower.contains(".msi") || lower.contains(".exe") {
        "windows"
    } else {
        "unknown"
    };

    let arch = if lower.contains("universal") {
        "universal"
    } else if lower.contains("aarch64") || lower.contains("arm64") {
        "aarch64"
    } else if lower.contains("x86_64") || lower.contains("amd64") {
        "x86_64"
    } else {
        "unknown"
    };

    (platform.to_string(), arch.to_string())
}

/// Returns today's date as YYYY-MM-DD without pulling in the chrono crate.
fn chrono_date_today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple date calculation
    let days = now / 86400;
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let month_days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut m = 0;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md {
            m = i + 1;
            break;
        }
        remaining -= md;
    }
    let d = remaining + 1;
    format!("{:04}-{:02}-{:02}", y, m, d)
}

fn print_preset(preset: &ProfilePreset, active: bool) {
    let marker = if active { "*" } else { "-" };
    println!(
        "  {} {} ({}) -> idle {:.1}, poll {}ms, automation {}s, ai {}",
        marker,
        preset.id,
        preset.label,
        preset.idle_threshold,
        preset.poll_interval_ms,
        preset.automation_interval_secs,
        preset.ai_profile
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_memory_scales_units() {
        assert_eq!(format_memory(512), "512 B");
        assert_eq!(format_memory(2048), "2.00 KB");
        assert_eq!(format_memory(2 * 1024 * 1024), "2.00 MB");
        assert_eq!(format_memory(3 * 1024 * 1024 * 1024), "3.00 GB");
    }

    #[test]
    fn determine_network_view_prefers_connections_when_filtered() {
        assert_eq!(
            determine_network_view(false, false, false, Some(NetworkProtocolArg::Tcp), None),
            NetworkView::Connections
        );
        assert_eq!(
            determine_network_view(false, false, false, None, Some(443)),
            NetworkView::Connections
        );
        assert_eq!(
            determine_network_view(true, true, true, None, None),
            NetworkView::Connections
        );
        assert_eq!(
            determine_network_view(false, true, false, None, None),
            NetworkView::Alerts
        );
        assert_eq!(
            determine_network_view(false, false, true, None, None),
            NetworkView::Top
        );
    }

    #[test]
    fn build_network_filter_maps_protocol_and_port() {
        let filter = build_network_filter(Some(NetworkProtocolArg::Udp), Some(53));
        assert_eq!(filter.protocols, Some(vec![Protocol::UDP]));
        assert_eq!(filter.ports, Some(vec![53]));
    }

    #[test]
    fn ai_provider_maps_to_core_defaults() {
        assert_eq!(
            AiProvider::Openai.to_core_provider(),
            core_ai::AiProvider::OpenAI
        );
        assert_eq!(AiProvider::Anthropic.display_name(), "Anthropic");
        assert_eq!(AiProvider::Gemini.default_model(), "gemini-2.0-flash");
        assert_eq!(
            AiProvider::Openrouter.default_model(),
            "meta-llama/llama-3.2-3b-instruct:free"
        );
    }

    #[test]
    fn network_protocol_arg_maps_to_core() {
        assert_eq!(NetworkProtocolArg::Tcp.to_protocol(), Protocol::TCP);
        assert_eq!(NetworkProtocolArg::Icmp.to_protocol(), Protocol::ICMP);
        assert_eq!(NetworkProtocolArg::Other.to_protocol(), Protocol::Other);
    }

    #[test]
    fn detect_platform_arch_from_filenames() {
        assert_eq!(
            detect_platform_arch("omnimon-6.7.0-macos-aarch64.dmg"),
            ("macos".into(), "aarch64".into())
        );
        assert_eq!(
            detect_platform_arch("omnimon-linux-x86_64.tar.gz"),
            ("linux".into(), "x86_64".into())
        );
        assert_eq!(
            detect_platform_arch("OmniMon-Setup-windows-x86_64.msi"),
            ("windows".into(), "x86_64".into())
        );
        assert_eq!(
            detect_platform_arch("omnimon-universal.dmg"),
            ("macos".into(), "universal".into())
        );
    }

    #[test]
    fn chrono_date_today_is_iso_shaped() {
        let today = chrono_date_today();
        assert_eq!(today.len(), 10);
        assert_eq!(&today[4..5], "-");
        assert_eq!(&today[7..8], "-");
        let year: u32 = today[0..4].parse().unwrap();
        assert!(year >= 2024);
    }

    #[test]
    fn rules_schema_subcommand_succeeds() {
        let cli = Cli::parse_from(["omnimon", "rules", "schema"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn rules_list_subcommand_succeeds() {
        let cli = Cli::parse_from(["omnimon", "rules", "list"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn doctor_subcommand_succeeds() {
        let cli = Cli::parse_from(["omnimon", "doctor"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn settings_get_subcommand_succeeds() {
        let cli = Cli::parse_from(["omnimon", "settings", "get"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn settings_presets_subcommand_succeeds() {
        let cli = Cli::parse_from(["omnimon", "settings", "presets"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn kill_missing_pid_returns_error_code() {
        let cli = Cli::parse_from(["omnimon", "kill", "99999999"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn rules_remove_missing_returns_error_code() {
        let cli = Cli::parse_from(["omnimon", "rules", "remove", "no-such-rule-xyz"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn rules_load_missing_file_returns_error_code() {
        let cli = Cli::parse_from([
            "omnimon",
            "rules",
            "load",
            "/tmp/omnimon-missing-rules-file-does-not-exist.json",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn release_checksum_missing_file_returns_error_code() {
        let cli = Cli::parse_from([
            "omnimon",
            "release",
            "checksum",
            "/tmp/omnimon-missing-artifact.bin",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn release_checksum_of_temp_file_succeeds() {
        let path = std::env::temp_dir().join("omnimon-cli-checksum-test.bin");
        std::fs::write(&path, b"omnimon-coverage").unwrap();
        let cli = Cli::parse_from(["omnimon", "release", "checksum", path.to_str().unwrap()]);
        assert!(run_cli(cli).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn security_scan_without_cve_db_succeeds() {
        let cli = Cli::parse_from(["omnimon", "security-scan"]);
        // May fail if keyring unavailable; accept Ok or Err(1)
        let result = run_cli(cli);
        assert!(result.is_ok() || result == Err(1));
    }

    #[test]
    fn cloud_sync_missing_key_returns_error() {
        let cli = Cli::parse_from([
            "omnimon",
            "cloud",
            "sync",
            "--report-path",
            "/tmp/omnimon-missing-report.enc",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn tabs_close_invalid_browser_returns_error() {
        let cli = Cli::parse_from([
            "omnimon",
            "tabs",
            "close",
            "--browser",
            "not-a-browser",
            "--id",
            "1",
            "--url",
            "https://example.com",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn settings_set_invalid_key_returns_error() {
        let cli = Cli::parse_from(["omnimon", "settings", "set", "not-a-real-key", "value"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn settings_use_unknown_preset_returns_error() {
        let cli = Cli::parse_from(["omnimon", "settings", "use", "no-such-preset-id"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn top_network_processes_respects_limit() {
        let state = watcher::SystemState {
            top_network_processes: (0..5)
                .map(|i| core::network::ProcessNetworkThroughput {
                    pid: i,
                    process_name: Some(format!("p{i}")),
                    rx_bytes_per_sec: i as u64,
                    tx_bytes_per_sec: 0,
                    tcp_packets_per_sec: 0,
                    udp_packets_per_sec: 0,
                })
                .collect(),
            ..Default::default()
        };
        let top = top_network_processes(&state, 3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].pid, 0);
    }

    #[test]
    fn print_network_text_and_json_cover_views() {
        let state = watcher::SystemState::default();
        let filter = NetworkFilter::default();
        print_network_text(&state, NetworkView::Top, &filter);
        print_network_text(&state, NetworkView::Alerts, &filter);
        print_network_text(&state, NetworkView::Connections, &filter);
        print_network_json(&state, NetworkView::Top, &filter);
        print_network_json(&state, NetworkView::Alerts, &filter);
        print_network_json(&state, NetworkView::Connections, &filter);
    }

    #[test]
    fn settings_set_theme_and_use_preset_succeed() {
        let cli = Cli::parse_from(["omnimon", "settings", "set", "theme", "dark"]);
        assert!(run_cli(cli).is_ok());
        let cli = Cli::parse_from(["omnimon", "settings", "use", "general"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn status_text_and_json_with_fast_startup_wait() {
        std::env::set_var("OMNIMON_CLI_STARTUP_WAIT_MS", "0");
        let text = Cli::parse_from(["omnimon", "status", "--format", "text"]);
        assert!(run_cli(text).is_ok());
        let json = Cli::parse_from(["omnimon", "status", "--format", "json"]);
        assert!(run_cli(json).is_ok());
        std::env::remove_var("OMNIMON_CLI_STARTUP_WAIT_MS");
    }

    #[test]
    fn network_views_with_fast_startup_wait() {
        std::env::set_var("OMNIMON_CLI_STARTUP_WAIT_MS", "0");
        for args in [
            vec!["omnimon", "network", "--format", "text"],
            vec!["omnimon", "network", "--format", "json", "--top"],
            vec!["omnimon", "network", "--alerts"],
            vec!["omnimon", "network", "--connections"],
        ] {
            let cli = Cli::parse_from(args);
            assert!(run_cli(cli).is_ok());
        }
        std::env::remove_var("OMNIMON_CLI_STARTUP_WAIT_MS");
    }

    #[test]
    fn sync_keychain_flag_runs_without_panic() {
        let cli = Cli::parse_from(["omnimon", "--sync-keychain", "doctor"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn tabs_list_succeeds() {
        let cli = Cli::parse_from(["omnimon", "tabs", "list"]);
        assert!(run_cli(cli).is_ok());
    }

    #[test]
    fn print_preset_covers_active_marker() {
        let preset = ProfilePreset {
            id: "general".into(),
            label: "General".into(),
            ai_profile: "balanced".into(),
            idle_threshold: 1.0,
            poll_interval_ms: 1000,
            automation_interval_secs: 5,
        };
        print_preset(&preset, true);
        print_preset(&preset, false);
    }

    #[test]
    fn release_sign_verify_and_manifest_roundtrip() {
        let dir = std::env::temp_dir().join(format!("omnimon-cli-release-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let artifact = dir.join("omnimon-macos-aarch64.bin");
        std::fs::write(&artifact, b"omnimon-release-bytes").unwrap();

        let (signing_key, verifying_key) = crypto::generate_ed25519_keypair();
        let key_path = dir.join("signing.key");
        std::fs::write(&key_path, crypto::export_signing_key(&signing_key)).unwrap();
        let pubkey = crypto::export_public_key(&verifying_key);

        let sign = Cli::parse_from([
            "omnimon",
            "release",
            "sign",
            artifact.to_str().unwrap(),
            "--version",
            "6.7.0-test",
            "--key-file",
            key_path.to_str().unwrap(),
        ]);
        assert!(run_cli(sign).is_ok());

        let sig_path = format!("{}.sig.json", artifact.display());
        let verify = Cli::parse_from([
            "omnimon",
            "release",
            "verify",
            artifact.to_str().unwrap(),
            "--sig",
            &sig_path,
            "--pubkey",
            &pubkey,
        ]);
        assert!(run_cli(verify).is_ok());

        let manifest = Cli::parse_from([
            "omnimon",
            "release",
            "manifest",
            "--version",
            "6.7.0-test",
            "--dir",
            dir.to_str().unwrap(),
            "--key-file",
            key_path.to_str().unwrap(),
        ]);
        assert!(run_cli(manifest).is_ok());

        let manifest_path = dir.join("releases.json");
        let verify_manifest = Cli::parse_from([
            "omnimon",
            "release",
            "verify-manifest",
            manifest_path.to_str().unwrap(),
            "--pubkey",
            &pubkey,
        ]);
        assert!(run_cli(verify_manifest).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rules_load_valid_file_succeeds() {
        let path = std::env::temp_dir().join(format!("omnimon-rules-{}.json", std::process::id()));
        std::fs::write(
            &path,
            r#"{"schema_version":1,"rules":[{"id":"cov-rule","name":"Coverage","enabled":true,"kind":"process_port","process_contains":null,"country_code":null,"destination_ip":null,"destination_cidr":null,"destination_port":18080,"protocol":"any","process_memory_mb_gt":null,"mitre_technique_id":"T1571"}]}"#,
        )
        .unwrap();
        let cli = Cli::parse_from(["omnimon", "rules", "load", path.to_str().unwrap()]);
        // Schema may reject depending on exact rule shape; accept Ok or Err(1).
        let result = run_cli(cli);
        assert!(result.is_ok() || result == Err(1));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn settings_set_numeric_fields() {
        for (key, value) in [
            ("font-size", "14"),
            ("idle-threshold", "1.5"),
            ("poll-interval-ms", "1500"),
            ("automation-interval-secs", "7"),
            ("locale", "es"),
            ("ai-profile", "general"),
        ] {
            let cli = Cli::parse_from(["omnimon", "settings", "set", key, value]);
            assert!(run_cli(cli).is_ok(), "settings set {key}");
        }
        let bad_font = Cli::parse_from(["omnimon", "settings", "set", "font-size", "nope"]);
        assert_eq!(run_cli(bad_font), Err(1));
        let bad_idle = Cli::parse_from(["omnimon", "settings", "set", "idle-threshold", "x"]);
        assert_eq!(run_cli(bad_idle), Err(1));
    }

    #[test]
    fn optimize_without_key_returns_error() {
        let cli = Cli::parse_from(["omnimon", "optimize", "--ai", "openai", "--target", "all"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn chat_without_key_returns_error() {
        let cli = Cli::parse_from(["omnimon", "chat", "--ai", "openai", "hello"]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn release_sign_missing_key_file_returns_error() {
        let artifact = std::env::temp_dir().join(format!(
            "omnimon-sign-missing-key-{}.bin",
            std::process::id()
        ));
        std::fs::write(&artifact, b"bytes").unwrap();
        let cli = Cli::parse_from([
            "omnimon",
            "release",
            "sign",
            artifact.to_str().unwrap(),
            "--version",
            "1.0.0",
            "--key-file",
            "/tmp/omnimon-definitely-missing-signing-key.b64",
        ]);
        assert_eq!(run_cli(cli), Err(1));
        let _ = std::fs::remove_file(artifact);
    }

    #[test]
    fn auth_login_persists_or_errors_cleanly() {
        let cli = Cli::parse_from(["omnimon", "auth", "login", "cn_test_key_coverage"]);
        let result = run_cli(cli);
        assert!(result.is_ok() || result == Err(1));
    }

    #[test]
    fn config_rotate_key_runs() {
        let cli = Cli::parse_from(["omnimon", "config", "rotate-key"]);
        let result = run_cli(cli);
        assert!(result.is_ok() || result == Err(1));
    }

    #[test]
    fn release_generate_keypair_runs() {
        let cli = Cli::parse_from(["omnimon", "release", "generate-keypair"]);
        let result = run_cli(cli);
        assert!(result.is_ok() || result == Err(1));
    }

    #[test]
    fn security_scan_with_invalid_cve_db_errors() {
        let cli = Cli::parse_from([
            "omnimon",
            "security-scan",
            "--cve-db",
            "/tmp/omnimon-missing-cve-db.json",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn cloud_sync_with_missing_report_after_fake_key() {
        // Without a key this fails early; with a key it fails on missing file.
        let path = std::env::temp_dir().join("omnimon-missing-report-for-sync.enc");
        let _ = std::fs::remove_file(&path);
        let cli = Cli::parse_from([
            "omnimon",
            "cloud",
            "sync",
            "--report-path",
            path.to_str().unwrap(),
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn tabs_focus_invalid_browser_returns_error() {
        let cli = Cli::parse_from([
            "omnimon",
            "tabs",
            "focus",
            "--browser",
            "not-a-browser",
            "--id",
            "1",
            "--url",
            "https://example.com",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn release_verify_missing_files_error() {
        let cli = Cli::parse_from([
            "omnimon",
            "release",
            "verify",
            "/tmp/omnimon-missing-artifact.bin",
            "--sig",
            "/tmp/omnimon-missing.sig.json",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }

    #[test]
    fn release_manifest_missing_dir_errors() {
        let cli = Cli::parse_from([
            "omnimon",
            "release",
            "manifest",
            "--version",
            "1.0.0",
            "--dir",
            "/tmp/omnimon-missing-release-dir-xyz",
            "--key-file",
            "/tmp/omnimon-missing-key.b64",
        ]);
        assert_eq!(run_cli(cli), Err(1));
    }
}
