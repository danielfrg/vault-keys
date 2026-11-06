mod auth;
mod cli;
mod env;
mod export;
mod manage;
mod run;
mod secrets;
mod vault;

use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Command::Login { user, ttl } => auth::login(user, ttl.as_deref()).await,
        Command::Logout => auth::logout(),
        Command::Run { secrets, command } => run::run(secrets, command).await,
        Command::Set { secret, key } => manage::set(secret, key).await,
        Command::Edit { secret } => manage::edit(secret).await,
        Command::List {
            secret,
            show_values,
        } => manage::list(secret.as_deref(), *show_values).await,
        Command::Rm { secret, key, yes } => manage::rm(secret, key.as_deref(), *yes).await,
        Command::Env { secrets } => env::env(secrets).await,
        Command::Export { secrets, format } => export::export(secrets, format).await,
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
