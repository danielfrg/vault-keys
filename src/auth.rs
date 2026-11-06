use std::io::{IsTerminal, Read};

use anyhow::{Context, Result, bail};
use vaultrs::{auth::userpass, error::ClientError, token};

use crate::vault;

const USERPASS_MOUNT: &str = "userpass";

fn read_password(user: &str) -> Result<String> {
    if std::io::stdin().is_terminal() {
        Ok(rpassword::prompt_password(format!(
            "Password for {user}: "
        ))?)
    } else {
        // Allows `pass show vault | keys login --user x`.
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s)?;
        Ok(s.trim_end_matches(['\r', '\n']).to_string())
    }
}

pub async fn login(user: &str, ttl: Option<&str>) -> Result<()> {
    let password = read_password(user)?;
    if password.is_empty() {
        bail!("empty password");
    }

    let client = vault::client("")?;
    let auth = userpass::login(&client, USERPASS_MOUNT, user, &password)
        .await
        .map_err(|e| describe(e, "login failed"))?;

    let mut lease = auth.lease_duration;
    if let Some(ttl) = ttl {
        let authed = vault::client(&auth.client_token)?;
        let renewed = token::renew_self(&authed, Some(ttl))
            .await
            .map_err(|e| describe(e, "could not apply --ttl"))?;
        lease = renewed.lease_duration;
    }

    vault::save_token(&auth.client_token)?;
    println!(
        "Logged in as {user}; token expires in {}",
        vault::fmt_duration(lease)
    );
    Ok(())
}

pub fn logout() -> Result<()> {
    let path = vault::token_path()?;
    match std::fs::remove_file(&path) {
        Ok(()) => println!("Removed {}", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => println!("Not logged in"),
        Err(e) => return Err(e).with_context(|| format!("removing {}", path.display())),
    }
    Ok(())
}

pub async fn status() -> Result<()> {
    let Some(tok) = vault::load_token()? else {
        bail!("not logged in; run `keys login --user <name>`");
    };
    let client = vault::client(&tok)?;
    let info = token::lookup_self(&client)
        .await
        .map_err(|e| describe(e, "token lookup failed"))?;

    let user = info
        .meta
        .as_ref()
        .and_then(|m| m.get("username"))
        .unwrap_or(&info.display_name);
    println!("User:      {user}");
    println!("Policies:  {}", info.policies.join(", "));
    println!(
        "Expires:   in {}{}",
        vault::fmt_duration(info.ttl),
        info.expire_time
            .as_deref()
            .map(|t| format!(" ({t})"))
            .unwrap_or_default()
    );
    println!("Renewable: {}", info.renewable.unwrap_or(false));
    Ok(())
}

/// Turn Vault client errors into short, actionable messages.
fn describe(e: ClientError, context: &str) -> anyhow::Error {
    match &e {
        ClientError::APIError { code: 403, .. } => {
            anyhow::anyhow!(
                "{context}: permission denied (token expired or invalid? try `keys login`)"
            )
        }
        ClientError::APIError { code, errors } if *code == 400 => {
            anyhow::anyhow!("{context}: {}", errors.join("; "))
        }
        ClientError::RestClientError { .. } => {
            anyhow::anyhow!("{context}: cannot reach Vault (check VAULT_ADDR and network)")
        }
        _ => anyhow::anyhow!("{context}: {e}"),
    }
}
