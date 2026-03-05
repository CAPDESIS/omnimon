use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get the status of macmon
    Status,
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
}

#[derive(Subcommand)]
enum ProfileCommands {
    /// Use a specific profile
    Use {
        /// The name of the profile to use
        name: String,
    },
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

    match &cli.command {
        Commands::Status => {
            let mock_memory_bytes = 4_509_715_660; // Approx 4.2 GB
            println!("macmon status: running gracefully");
            println!("Memory usage: {}", format_memory(mock_memory_bytes));
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
    }
}
