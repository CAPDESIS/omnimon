use clap::{Parser, Subcommand, ValueEnum};
use core::ai as core_ai;
use core::killer;
use core::metrics;
use core::watcher;

#[derive(Parser)]
#[command(name = "macmon")]
#[command(version = "4.3.0", about = "OmniMon: Monitor de sistema y navegador de próxima generación de alto rendimiento.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Force secure credential sync from keychain
    #[arg(long, global = true)]
    sync_keychain: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the status of macmon
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
    /// Manage profiles
    Profile {
        #[command(subcommand)]
        command: ProfileCommands,
    },
    /// Update macmon to the latest version
    Update,
    /// Smart Optimize via AI
    Optimize {
        /// AI Provider to use
        #[arg(long, value_enum)]
        ai: AiProvider,
        /// Target to optimize (e.g. browsers, all)
        #[arg(long)]
        target: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Use a specific profile
    Use {
        /// The name of the profile to use
        name: String,
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

fn main() {
    let cli = Cli::parse();

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
            // Start the system watcher and wait for initial data collection
            watcher::start_watcher();
            std::thread::sleep(std::time::Duration::from_millis(2500));

            let state = watcher::get_cached_state();
            let top_procs = metrics::top_processes_by_memory(10);

            match format {
                Format::Json => {
                    let procs_json: Vec<serde_json::Value> = top_procs
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "pid": p.pid,
                                "name": p.name,
                                "memory_bytes": p.memory_bytes
                            })
                        })
                        .collect();

                    let output = serde_json::json!({
                        "status": "running",
                        "total_memory_bytes": state.total_memory_bytes,
                        "used_memory_bytes": state.used_memory_bytes,
                        "free_memory_bytes": state.free_memory_bytes,
                        "free_percent": state.free_percent,
                        "swap_used_mb": state.swap_used_mb,
                        "cpu_usage_percent": state.cpu_usage_percent,
                        "net_rx_bytes_per_sec": state.net_rx_bytes_per_sec,
                        "net_tx_bytes_per_sec": state.net_tx_bytes_per_sec,
                        "top_processes": procs_json
                    });
                    println!("{}", serde_json::to_string_pretty(&output).unwrap());
                }
                Format::Text => {
                    println!("macmon status: running");
                    println!();
                    println!(
                        "  Memory: {} / {} ({:.1}% used)",
                        format_memory(state.used_memory_bytes),
                        format_memory(state.total_memory_bytes),
                        100.0 - state.free_percent as f64
                    );
                    println!("  Swap:   {} MB used", state.swap_used_mb);
                    println!("  CPU:    {:.1}%", state.cpu_usage_percent);
                    println!(
                        "  Net:    rx {} /s  tx {} /s",
                        format_memory(state.net_rx_bytes_per_sec),
                        format_memory(state.net_tx_bytes_per_sec)
                    );
                    println!();
                    println!("  Top processes by memory:");
                    println!("  {:>6}  {:<30}  {:>12}", "PID", "NAME", "MEMORY");
                    println!("  {}", "-".repeat(52));
                    for p in &top_procs {
                        println!(
                            "  {:>6}  {:<30}  {:>12}",
                            p.pid,
                            if p.name.len() > 30 {
                                &p.name[..30]
                            } else {
                                &p.name
                            },
                            format_memory(p.memory_bytes)
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
                    std::process::exit(1);
                }
            }
        }
        Commands::Profile { command } => match command {
            ProfileCommands::Use { name } => {
                println!("Profile management is not yet implemented.");
                println!("Would activate profile: {}", name);
            }
        },
        Commands::Update => {
            println!("Self-update is not yet implemented.");
            println!("Check https://github.com/chochy2001/omnimon for the latest version.");
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

            // Gather current process data
            let top_procs = metrics::top_processes_by_memory(25);
            let procs_json = serde_json::to_string(&top_procs).unwrap_or_else(|_| "[]".to_string());

            let profile = target_name;

            // Build a tokio runtime for the async AI call
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create async runtime");

            match rt.block_on(core_ai::analyze_with_ai(
                core_provider,
                model,
                &procs_json,
                profile,
            )) {
                Ok(suggestions) => {
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
                    std::process::exit(1);
                }
            }
        }
    }
}
