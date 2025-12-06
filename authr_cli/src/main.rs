use clap::{Parser, Subcommand};
use anyhow::Result;

mod commands;
mod tui_interface;

#[derive(Parser)]
#[command(name = "authr")]
#[command(about = "Time-based One-Time Password authenticator")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all accounts
    List,
    /// Add a new account
    Add {
        /// Account name
        name: String,
        /// Secret key (base32)
        secret: String,
    },
    /// Remove an account
    Remove {
        /// Account name
        name: String,
    },
    /// Show current TOTP for an account
    Show {
        /// Account name
        name: String,
        /// Show the seed key instead of the TOTP
        #[arg(long)]
        seed: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List) => commands::list()?,
        Some(Commands::Add { name, secret }) => commands::add(name, secret)?,
        Some(Commands::Remove { name }) => commands::remove(&name)?,
        Some(Commands::Show { name, seed }) => commands::show(&name, seed)?,
        None => tui_interface::run()?,
    }

    Ok(())
}
