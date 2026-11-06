use std::io::IsTerminal;

use anyhow::{Result, bail};

use crate::cli::Format;
use crate::{secrets, vault};

/// Quote a value for a .env file. Plain values stay bare; others use single
/// quotes (literal in common dotenv parsers), or double quotes with escapes
/// when the value contains a single quote or a newline.
fn dotenv_value(v: &str) -> String {
    let plain = !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:@%+=,-".contains(c));
    if plain {
        v.to_string()
    } else if !v.contains(['\'', '\n', '\r']) {
        format!("'{v}'")
    } else {
        let escaped = v
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r");
        format!("\"{escaped}\"")
    }
}

pub async fn export(args: &[String], format: &Format) -> Result<()> {
    // Never print values straight to a terminal.
    if std::io::stdout().is_terminal() {
        bail!(
            "refusing to print secrets to a terminal; redirect the output:\n  keys export {} > .env",
            args.join(" ")
        );
    }
    let client = vault::authed_client()?;
    let vars = secrets::load(&client, args).await?;
    match format {
        Format::Dotenv => {
            for (k, v) in vars {
                println!("{k}={}", dotenv_value(&v));
            }
        }
    }
    Ok(())
}
