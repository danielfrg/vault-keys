use std::io::{IsTerminal, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::process::Command;

use anyhow::{Context, Result, bail};
use vaultrs::{client::VaultClient, error::ClientError, kv2};

use crate::secrets::{self, as_strings, parse_mount_path, parse_ref, read_existing, read_raw};
use crate::vault;

/// `keys set <secret> <KEY>`: add or replace one key, keeping the others.
pub async fn set(secret_arg: &str, key: &str) -> Result<()> {
    let (mount, secret) = parse_ref(secret_arg)?;
    if !secrets::valid_env_name(key) {
        bail!("`{key}` is not a valid environment variable name");
    }
    let value = vault::read_hidden(&format!("Value for {key}: "))?;
    if value.is_empty() {
        bail!("empty value");
    }

    let client = vault::authed_client()?;
    let label = format!("{mount}/{secret}");
    let mut data = match read_raw(&client, &mount, &secret).await? {
        Some(raw) => as_strings(&label, raw)?,
        None => Default::default(),
    };
    let existed = data.insert(key.to_string(), value).is_some();
    secrets::write(&client, &mount, &secret, &data).await?;
    println!(
        "{} {key} in {label}",
        if existed { "Updated" } else { "Set" }
    );
    Ok(())
}

/// `keys edit <secret>`: edit the secret as JSON in $EDITOR.
pub async fn edit(secret_arg: &str) -> Result<()> {
    let (mount, secret) = parse_ref(secret_arg)?;
    let label = format!("{mount}/{secret}");
    let client = vault::authed_client()?;

    let current = match read_raw(&client, &mount, &secret).await? {
        Some(raw) => as_strings(&label, raw)?,
        None => Default::default(),
    };
    let before = serde_json::to_string_pretty(&current)? + "\n";

    // A private (0700) directory, removed on drop, so editor swap/backup
    // files do not outlive the edit.
    let dir = tempfile::Builder::new().prefix("keys-").tempdir()?;
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700))?;
    let path = dir.path().join("secret.json");
    {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        f.write_all(before.as_bytes())?;
    }

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let status = Command::new("sh")
        .args(["-c", &format!("exec {editor} \"$1\""), "sh"])
        .arg(&path)
        .status()
        .with_context(|| format!("starting editor `{editor}`"))?;
    if !status.success() {
        bail!("editor exited with {status}; nothing written");
    }

    let after = std::fs::read_to_string(&path)?;
    if after == before {
        println!("No changes");
        return Ok(());
    }
    let new: std::collections::BTreeMap<String, String> = serde_json::from_str(&after)
        .context("edited file is not a JSON object of string values; nothing written")?;
    for key in new.keys() {
        if !secrets::valid_env_name(key) {
            bail!("key `{key}` is not a valid environment variable name; nothing written");
        }
    }
    secrets::write(&client, &mount, &secret, &new).await?;
    println!("Saved {label}");
    Ok(())
}

fn print_secret(data: &secrets::Raw, show_values: bool) {
    let mut entries: Vec<_> = data.iter().collect();
    entries.sort_by_key(|(k, _)| k.as_str());
    for (k, v) in entries {
        match (show_values, v) {
            (false, _) => println!("{k}"),
            (true, serde_json::Value::String(v)) => println!("{k}={v}"),
            (true, other) => println!("{k}={other}"),
        }
    }
}

/// `keys list [<mount>:][<secret>] [--show-values]`
pub async fn list(arg: Option<&str>, show_values: bool) -> Result<()> {
    let (mount, path) = parse_mount_path(arg.unwrap_or(""))?;
    let client = vault::authed_client()?;

    // A path that is a secret shows its keys; otherwise treat it as a folder.
    if !path.is_empty()
        && let Some(data) = read_raw(&client, &mount, &path).await?
    {
        print_secret(&data, show_values);
        return Ok(());
    }
    if show_values {
        bail!("--show-values needs a single secret, e.g. `keys list ai --show-values`");
    }
    match secrets::list(&client, &mount, &path).await {
        Ok(mut names) => {
            names.sort();
            for n in names {
                println!("{n}");
            }
            Ok(())
        }
        Err(ClientError::APIError { code: 404, .. }) if path.is_empty() => Ok(()),
        Err(ClientError::APIError { code: 404, .. }) => {
            bail!("secret {mount}/{path} not found")
        }
        Err(e) => Err(vault::describe(e, &format!("listing {mount}/{path}"))),
    }
}

/// `keys rm <secret> [KEY] [--yes]`
pub async fn rm(secret_arg: &str, key: Option<&str>, yes: bool) -> Result<()> {
    let (mount, secret) = parse_ref(secret_arg)?;
    let label = format!("{mount}/{secret}");
    let client = vault::authed_client()?;

    match key {
        Some(key) => {
            let mut data = as_strings(&label, read_existing(&client, &mount, &secret).await?)?;
            if data.remove(key).is_none() {
                bail!("key `{key}` not found in {label}");
            }
            secrets::write(&client, &mount, &secret, &data).await?;
            println!("Removed {key} from {label}");
        }
        None => {
            read_existing(&client, &mount, &secret).await?;
            if !yes && !confirm(&format!("Delete {label} and all its versions?"))? {
                bail!("aborted");
            }
            delete_secret(&client, &mount, &secret).await?;
            println!("Deleted {label}");
        }
    }
    Ok(())
}

async fn delete_secret(client: &VaultClient, mount: &str, secret: &str) -> Result<()> {
    kv2::delete_metadata(client, mount, secret)
        .await
        .map_err(|e| vault::describe(e, &format!("deleting {mount}/{secret}")))
}

fn confirm(question: &str) -> Result<bool> {
    if !std::io::stdin().is_terminal() {
        bail!("refusing to delete without confirmation; pass --yes");
    }
    eprint!("{question} [y/N] ");
    std::io::stderr().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes"))
}
