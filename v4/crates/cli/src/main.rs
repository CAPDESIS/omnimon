use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "macmon")]
#[command(version = "4.0.0", about = "OmniMon: Monitor de sistema y navegador de próxima generación de alto rendimiento.", long_about = None)]
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
    }
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
        println!("Syncing credentials from keychain securely...");
    }

    match &cli.command {
        Commands::Status { format } => {
            let mock_memory_bytes = 4_509_715_660; // Approx 4.2 GB
            
            match format {
                Format::Json => {
                    println!("{{\"status\":\"running gracefully\",\"memory_usage_bytes\":{}}}", mock_memory_bytes);
                }
                Format::Text => {
                    println!("macmon status: running gracefully");
                    println!("Memory usage: {}", format_memory(mock_memory_bytes));
                }
            }
        }
        Commands::Kill { pid } => {
            println!("Killing process with PID: {}", pid);
        }
        Commands::Profile { command } => match command {
            ProfileCommands::Use { name } => {
                println!("Activating profile: {}", name);
            }
        },
        Commands::Update => {
            println!("Checking for macmon updates...");
        }
        Commands::Optimize { ai, target } => {
            let ai_name = match ai {
                AiProvider::Openai => "OpenAI",
                AiProvider::Anthropic => "Anthropic",
                AiProvider::Openrouter => "OpenRouter",
            };
            let target_name = target.as_deref().unwrap_or("all");
            println!("Starting Smart Optimize using {} on target '{}'", ai_name, target_name);
        }
    }
}
