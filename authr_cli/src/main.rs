use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod gui_interface;
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
    /// Internal use only for launching GUI in background
    #[command(hide = true)]
    GuiWorker,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List) => commands::list()?,
        Some(Commands::Add { name }) => commands::add(name)?,
        Some(Commands::Remove { name }) => commands::remove(&name)?,
        Some(Commands::Show { name, seed }) => commands::show(&name, seed)?,
        Some(Commands::GuiWorker) => {
            #[cfg(feature = "gui")]
            {
                gui_interface::run()?;
                return Ok(());
            }
            #[cfg(not(feature = "gui"))]
            {
                anyhow::bail!("GUI feature not enabled");
            }
        }
        None => {
            #[cfg(feature = "gui")]
            {
                // Prefer GUI if enabled.
                // Spawn detached process and exit.
                let exe = std::env::current_exe()?;
                std::process::Command::new(exe).arg("gui-worker").spawn()?;
                return Ok(());
            }

            #[cfg(all(feature = "tui", not(feature = "gui")))]
            {
                tui_interface::run()?;
                return Ok(());
            }

            #[cfg(not(any(feature = "tui", feature = "gui")))]
            {
                use clap::CommandFactory;
                Cli::command().print_help()?;
                println!("\nNo interface features enabled. Use --help to see commands.");
                return Ok(());
            }
        }
    }

    Ok(())
}
