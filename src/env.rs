use std::io::IsTerminal;

use anyhow::{Result, bail};

use crate::{secrets, vault};

/// POSIX single-quote a value.
fn quote(v: &str) -> String {
    format!("'{}'", v.replace('\'', r"'\''"))
}

pub async fn env(args: &[String]) -> Result<()> {
    // Never print values straight to a terminal.
    if std::io::stdout().is_terminal() {
        bail!(
            "refusing to print secrets to a terminal; load them with:\n  eval \"$(keys env {})\"",
            args.join(" ")
        );
    }
    let client = vault::authed_client()?;
    let vars = secrets::load(&client, args).await?;
    for (k, v) in vars {
        println!("export {k}={}", quote(&v));
    }
    Ok(())
}
