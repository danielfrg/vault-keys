use anyhow::{Context, Result, bail};
use vaultrs::{auth::userpass, token};

use crate::vault;

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
    let client = vault::authed_client()?;
    let info = token::lookup_self(&client)
        .await
        .map_err(|e| vault::describe(e, "token lookup failed"))?;

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
