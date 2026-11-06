use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use vaultrs::client::{VaultClient, VaultClientSettingsBuilder};

/// Path of the token file shared with the `vault` CLI.
pub fn token_path() -> Result<PathBuf> {
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".vault-token"))
}

/// Token from `VAULT_TOKEN`, else from `~/.vault-token`.
pub fn load_token() -> Result<Option<String>> {
    if let Ok(t) = std::env::var("VAULT_TOKEN")
        && !t.trim().is_empty()
    {
        return Ok(Some(t.trim().to_string()));
    }
    match std::fs::read_to_string(token_path()?) {
        Ok(t) if !t.trim().is_empty() => Ok(Some(t.trim().to_string())),
        Ok(_) => Ok(None),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).context("reading ~/.vault-token"),
    }
}

/// Write the token to `~/.vault-token` with mode 0600.
pub fn save_token(token: &str) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let path = token_path()?;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .with_context(|| format!("writing {}", path.display()))?;
    f.write_all(token.as_bytes())?;
    Ok(())
}

pub fn client(token: &str) -> Result<VaultClient> {
    let addr = match std::env::var("VAULT_ADDR") {
        Ok(a) if !a.is_empty() => a,
        _ => bail!("VAULT_ADDR is not set (e.g. export VAULT_ADDR=https://vault.example.com)"),
    };
    let settings = VaultClientSettingsBuilder::default()
        .address(addr)
        .token(token)
        .build()
        .context("invalid Vault client settings")?;
    VaultClient::new(settings).context("creating Vault client")
}

/// Format seconds as e.g. `11h59m`.
pub fn fmt_duration(secs: u64) -> String {
    let (d, h, m, s) = (
        secs / 86400,
        secs % 86400 / 3600,
        secs % 3600 / 60,
        secs % 60,
    );
    match (d, h, m) {
        (0, 0, 0) => format!("{s}s"),
        (0, 0, _) => format!("{m}m{s:02}s"),
        (0, _, _) => format!("{h}h{m:02}m"),
        _ => format!("{d}d{h}h"),
    }
}
