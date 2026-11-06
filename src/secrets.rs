use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, bail};
use vaultrs::{client::VaultClient, error::ClientError, kv2};

const DEFAULT_MOUNT: &str = "keychain";

pub type Raw = HashMap<String, serde_json::Value>;

/// Parse `<secret>` or `<mount>:<secret>` into (mount, secret).
pub fn parse_ref(arg: &str) -> Result<(String, String)> {
    let (mount, secret) = parse_mount_path(arg)?;
    if secret.is_empty() {
        bail!("invalid secret `{arg}` (expected <secret> or <mount>:<secret>)");
    }
    Ok((mount, secret))
}

/// Like `parse_ref`, but the secret part may be empty (`mount:` or "").
pub fn parse_mount_path(arg: &str) -> Result<(String, String)> {
    let (mount, secret) = match arg.split_once(':') {
        Some((m, s)) => (m.to_string(), s),
        None => (default_mount(), arg),
    };
    if mount.is_empty() {
        bail!("invalid secret `{arg}` (expected <secret> or <mount>:<secret>)");
    }
    Ok((mount, secret.trim_matches('/').to_string()))
}

pub fn default_mount() -> String {
    match std::env::var("KEYS_MOUNT") {
        Ok(m) if !m.is_empty() => m,
        _ => DEFAULT_MOUNT.to_string(),
    }
}

pub fn valid_env_name(k: &str) -> bool {
    let mut chars = k.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Read a secret. `None` if it does not exist.
pub async fn read_raw(client: &VaultClient, mount: &str, secret: &str) -> Result<Option<Raw>> {
    match kv2::read::<Raw>(client, mount, secret).await {
        Ok(data) => Ok(Some(data)),
        Err(ClientError::APIError { code: 404, .. }) => Ok(None),
        Err(e) => Err(crate::vault::describe(
            e,
            &format!("reading {mount}/{secret}"),
        )),
    }
}

/// Read a secret, erroring if it does not exist.
pub async fn read_existing(client: &VaultClient, mount: &str, secret: &str) -> Result<Raw> {
    read_raw(client, mount, secret)
        .await?
        .ok_or_else(|| anyhow::anyhow!("secret {mount}/{secret} not found"))
}

/// Require every value to be a string.
pub fn as_strings(label: &str, data: Raw) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for (key, value) in data {
        let serde_json::Value::String(value) = value else {
            bail!("{label}: value of `{key}` is not a string");
        };
        out.insert(key, value);
    }
    Ok(out)
}

/// Write the whole secret (creates a new version).
pub async fn write(
    client: &VaultClient,
    mount: &str,
    secret: &str,
    data: &BTreeMap<String, String>,
) -> Result<()> {
    kv2::set(client, mount, secret, data)
        .await
        .map_err(|e| crate::vault::describe(e, &format!("writing {mount}/{secret}")))?;
    Ok(())
}

/// Read the keys of every secret and merge them; later secrets win.
pub async fn load(client: &VaultClient, args: &[String]) -> Result<BTreeMap<String, String>> {
    let mut merged = BTreeMap::new();
    for arg in args {
        let (mount, secret) = parse_ref(arg)?;
        let label = format!("{mount}/{secret}");
        let data = as_strings(&label, read_existing(client, &mount, &secret).await?)?;
        for (key, value) in data {
            if !valid_env_name(&key) {
                bail!("{label}: key `{key}` is not a valid environment variable name");
            }
            merged.insert(key, value);
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_prefix_is_parsed() {
        assert_eq!(
            parse_ref("homelab:aws").unwrap(),
            ("homelab".into(), "aws".into())
        );
        assert_eq!(
            parse_ref("homelab:team/aws/").unwrap(),
            ("homelab".into(), "team/aws".into())
        );
    }

    #[test]
    fn empty_parts_are_rejected() {
        assert!(parse_ref("homelab:").is_err());
        assert!(parse_ref(":aws").is_err());
        assert!(parse_mount_path(":x").is_err());
        assert_eq!(
            parse_mount_path("homelab:").unwrap(),
            ("homelab".into(), String::new())
        );
    }

    #[test]
    fn env_names() {
        assert!(valid_env_name("ANTHROPIC_API_KEY"));
        assert!(valid_env_name("_x1"));
        assert!(!valid_env_name("1X"));
        assert!(!valid_env_name("bad-name"));
        assert!(!valid_env_name(""));
    }
}
