use std::os::unix::process::CommandExt;
use std::process::Command;

use anyhow::{Result, anyhow};

use crate::{secrets, vault};

/// Load the secrets and replace this process with `command`.
///
/// Using exec (instead of spawning a child) means signals, the terminal and
/// the exit code all belong directly to the command, and nothing is left
/// running that holds the secrets besides the command itself.
pub async fn run(secret_args: &[String], command: &[String]) -> Result<()> {
    let client = vault::authed_client()?;
    let vars = secrets::load(&client, secret_args).await?;

    let (program, args) = command
        .split_first()
        .ok_or_else(|| anyhow!("no command given"))?;

    let err = Command::new(program).args(args).envs(vars).exec();
    // exec only returns on failure.
    Err(anyhow!("cannot run `{program}`: {err}"))
}
