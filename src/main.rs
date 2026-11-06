mod cli;

use clap::Parser;

use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    let name = match &cli.command {
        Command::Run { .. } => "run",
        Command::Env { .. } => "env",
        Command::Set { .. } => "set",
        Command::Edit { .. } => "edit",
        Command::Get { .. } => "get",
        Command::List { .. } => "list",
        Command::Rm { .. } => "rm",
        Command::Export { .. } => "export",
        Command::Status => "status",
        Command::Doctor => "doctor",
        Command::Push { .. } => "push",
    };

    println!("keys {name}: not implemented");
}
