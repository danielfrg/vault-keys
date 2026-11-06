mod auth;
mod cli;
mod vault;

use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Command::Login { user, ttl } => auth::login(user, ttl.as_deref()).await,
        Command::Logout => auth::logout(),
        Command::Status => auth::status().await,
        other => {
            println!("keys: not implemented: {other:?}");
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("keys: {e:#}");
        std::process::exit(1);
    }
}
