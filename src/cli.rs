use clap::{Parser, Subcommand, ValueEnum};

/// Fetch and manage credentials stored in HashiCorp Vault.
#[derive(Debug, Parser)]
#[command(
    name = "keys",
    version = option_env!("KEYS_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")),
    about
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Log in to Vault with userpass and save the token to ~/.vault-token
    Login {
        /// Vault username
        #[arg(short, long, env = "VAULT_USER")]
        user: String,

        /// Requested token lifetime, e.g. 12h (limited by Vault's max TTL)
        #[arg(long)]
        ttl: Option<String>,
    },

    /// Remove the saved Vault token
    Logout,

    /// Run a command with secrets as environment variables
    Run {
        /// Secrets to load, as <secret> or <mount>:<secret>
        #[arg(required = true)]
        secrets: Vec<String>,

        /// Command to run (after `--`)
        #[arg(last = true, required = true)]
        command: Vec<String>,
    },

    /// Load secrets into the current shell
    Env {
        #[arg(required = true)]
        secrets: Vec<String>,
    },

    /// Store a value (read with hidden input)
    Set { secret: String, key: String },

    /// Edit a secret in $EDITOR
    Edit { secret: String },

    /// List secrets on a mount, or the keys of one secret (names only unless --show-values)
    List {
        /// `<secret>` or `<mount>:<secret>`; omit to list the default mount
        secret: Option<String>,

        /// Print the values too (only for a single secret)
        #[arg(long)]
        show_values: bool,
    },

    /// Remove a secret, or a single key
    Rm {
        secret: String,

        key: Option<String>,

        /// Do not ask for confirmation when removing a whole secret
        #[arg(short, long)]
        yes: bool,
    },

    /// Print secrets in a file format (redirect it: `keys export ai > .env`)
    Export {
        #[arg(required = true)]
        secrets: Vec<String>,

        #[arg(long, value_enum, default_value_t = Format::Dotenv)]
        format: Format,
    },

    /// Check Vault connectivity, login, policies, token expiry and mount access
    Status,

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
