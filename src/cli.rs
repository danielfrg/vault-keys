use clap::{Parser, Subcommand, ValueEnum};

/// Fetch and manage credentials stored in HashiCorp Vault.
#[derive(Debug, Parser)]
#[command(name = "keys", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run a command with secrets as environment variables
    Run {
        /// Secret sets to load
        #[arg(required = true)]
        sets: Vec<String>,

        /// Command to run (after `--`)
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Load secrets into the current shell
    Env {
        #[arg(required = true)]
        sets: Vec<String>,
    },

    /// Store a value (read with hidden input)
    Set { set: String, field: String },

    /// Edit a secret set in $EDITOR
    Edit { set: String },

    /// Show a secret set (field names only unless --show-values)
    Get {
        set: String,

        /// Only this field
        field: Option<String>,

        /// Print values
        #[arg(long)]
        show_values: bool,
    },

    /// List secret sets (or the fields of one set)
    List { set: Option<String> },

    /// Remove a secret set, or a single field
    Rm { set: String, field: Option<String> },

    /// Export secret sets in a given format
    Export {
        #[arg(required = true)]
        sets: Vec<String>,

        #[arg(long, value_enum, default_value_t = Format::Dotenv)]
        format: Format,
    },

    /// Show login identity, policies and token expiry
    Status,

    /// Diagnose Vault, token and permission problems
    Doctor,

    /// Push a stored value to another service
    Push {
        #[command(subcommand)]
        target: PushTarget,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Format {
    Dotenv,
}

#[derive(Debug, Subcommand)]
pub enum PushTarget {
    /// Push a value to a GitHub Actions secret
    Github {
        /// Repository, as owner/repo
        repo: String,

        /// Name of the stored value
        name: String,
    },
}
