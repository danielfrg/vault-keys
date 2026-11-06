use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, bail};
use vaultrs::{client::VaultClient, error::ClientError, kv2};

const DEFAULT_MOUNT: &str = "keychain";

/// Parse `<secret>` or `<mount>:<secret>` into (mount, secret).
pub fn parse_ref(arg: &str) -> Result<(String, String)> {
    let (mount, secret) = match arg.split_once(':') {
        Some((m, s)) => (m.to_string(), s),
        None => (default_mount(), arg),
    };
    if mount.is_empty() || secret.is_empty() {
        bail!("invalid secret `{arg}` (expected <secret> or <mount>:<secret>)");
    }
    Ok((mount, secret.trim_matches('/').to_string()))
}

fn default_mount() -> String {
    match std::env::var("KEYS_MOUNT") {
        Ok(m) if !m.is_empty() => m,
        _ => DEFAULT_MOUNT.to_string(),
    }
}

fn valid_env_name(k: &str) -> bool {
    let mut chars = k.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Read the keys of every secret and merge them; later secrets win.
pub async fn load(client: &VaultClient, args: &[String]) -> Result<BTreeMap<String, String>> {
    let mut merged = BTreeMap::new();
    for arg in args {
        let (mount, secret) = parse_ref(arg)?;
        let data: HashMap<String, serde_json::Value> = kv2::read(client, &mount, &secret)
            .await
            .map_err(|e| match e {
                ClientError::APIError { code: 404, .. } => {
                    anyhow::anyhow!("secret {mount}/{secret} not found")
                }
                e => crate::vault::describe(e, &format!("reading {mount}/{secret}")),
            })?;
        for (key, value) in data {
            if !valid_env_name(&key) {
                bail!("{mount}/{secret}: key `{key}` is not a valid environment variable name");
            }
            let serde_json::Value::String(value) = value else {
                bail!("{mount}/{secret}: value of `{key}` is not a string");
            };
            merged.insert(key, value);
        }
    }
    Ok(merged)
}
