use anyhow::{Context, Result, bail};
use vaultrs::{auth::userpass, error::ClientError, token};

use crate::{secrets, vault};

const USERPASS_MOUNT: &str = "userpass";

pub async fn login(user: &str, ttl: Option<&str>) -> Result<()> {
    let password = vault::read_hidden(&format!("Password for {user}: "))?;
    if password.is_empty() {
        bail!("empty password");
    }

    let client = vault::client("")?;
    let auth = userpass::login(&client, USERPASS_MOUNT, user, &password)
        .await
        .map_err(|e| vault::describe(e, "login failed"))?;

    let mut lease = auth.lease_duration;
    if let Some(ttl) = ttl {
        let authed = vault::client(&auth.client_token)?;
        let renewed = token::renew_self(&authed, Some(ttl))
            .await
            .map_err(|e| vault::describe(e, "could not apply --ttl"))?;
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
    let mut healthy = true;
    let mut line = |label: &str, ok: bool, text: String| {
        healthy &= ok;
        println!(
            "{:<10} {}{text}",
            format!("{label}:"),
            if ok { "" } else { "PROBLEM: " }
        );
    };

    // 1. Vault address and reachability (no token needed).
    let Ok(addr) = std::env::var("VAULT_ADDR") else {
        line("Vault", false, "VAULT_ADDR is not set".into());
        bail!("status check failed");
    };
    match vaultrs::sys::health(&vault::client("")?).await {
        Ok(h) if h.sealed => line("Vault", false, format!("{addr} is sealed")),
        Ok(h) => line("Vault", true, format!("{addr} (v{})", h.version)),
        Err(e) => {
            line("Vault", false, vault::describe(e, &addr).to_string());
            bail!("status check failed");
        }
    }

    // 2. Token.
    let Some(tok) = vault::load_token()? else {
        line(
            "Login",
            false,
            "not logged in; run `keys login --user <name>`".into(),
        );
        bail!("status check failed");
    };
    let client = vault::client(&tok)?;
    let info = match token::lookup_self(&client).await {
        Ok(info) => info,
        Err(ClientError::APIError { code: 403, .. }) => {
            line(
                "Login",
                false,
                "token expired or invalid; run `keys login --user <name>`".into(),
            );
            bail!("status check failed");
        }
        Err(e) => {
            line(
                "Login",
                false,
                vault::describe(e, "token lookup failed").to_string(),
            );
            bail!("status check failed");
        }
    };
    let user = info
        .meta
        .as_ref()
        .and_then(|m| m.get("username"))
        .unwrap_or(&info.display_name);
    line("User", true, user.clone());
    line("Policies", true, info.policies.join(", "));
    line(
        "Expires",
        true,
        format!(
            "in {}{}",
            vault::fmt_duration(info.ttl),
            info.expire_time
                .as_deref()
                .map(|t| format!(" ({t})"))
                .unwrap_or_default()
        ),
    );
    line(
        "Renewable",
        true,
        info.renewable.unwrap_or(false).to_string(),
    );

    // 3. Access to the default mount.
    let mount = secrets::default_mount();
    match secrets::list(&client, &mount, "").await {
        Ok(_) | Err(ClientError::APIError { code: 404, .. }) => {
            line("Mount", true, format!("{mount} (can list secrets)"))
        }
        Err(ClientError::APIError { code: 403, .. }) => line(
            "Mount",
            false,
            format!("no access to {mount}; the token's policies do not allow listing it"),
        ),
        Err(e) => line("Mount", false, vault::describe(e, &mount).to_string()),
    }

    if !healthy {
        bail!("status check failed");
    }
    Ok(())
}
