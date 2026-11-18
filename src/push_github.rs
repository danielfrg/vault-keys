//! `keys push github`: make a repository's GitHub Actions secrets match Vault.
//!
//! Layout on the GitHub mount (default `github`, or `KEYS_GITHUB_MOUNT`):
//!
//! - `<owner>/<repo>`: the repository's own keys, named exactly like the
//!   GitHub secrets. Its custom metadata `include` lists other secrets to
//!   merge first, comma separated, e.g. `shared/ntfy,shared/cloudflare`.
//! - `shared/...`: secrets used by several repositories; never pushed alone.
//!
//! Includes are merged in order and the repository's own keys last; later
//! values win. A value of exactly `TODO` is a placeholder: it is not pushed
//! and the GitHub secret of that name is left as it is.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use vaultrs::{client::VaultClient, error::ClientError, kv2};

use crate::secrets::{self, as_strings, read_existing, read_raw};
use crate::vault;

const DEFAULT_MOUNT: &str = "github";
/// Top-level folder on the mount that holds shared secrets, not repositories.
const SHARED: &str = "shared";
/// A value that marks a key as not migrated yet.
const PLACEHOLDER: &str = "TODO";

pub fn mount() -> String {
    match std::env::var("KEYS_GITHUB_MOUNT") {
        Ok(m) if !m.is_empty() => m,
        _ => DEFAULT_MOUNT.to_string(),
    }
}

/// A desired secret: its value and the Vault secret it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub value: String,
    pub from: String,
}

#[derive(Debug, PartialEq)]
pub enum Action {
    /// Not on GitHub yet; will be created.
    Add,
    /// On GitHub; will be overwritten (values cannot be compared).
    Update,
    /// Placeholder in Vault, already on GitHub: left untouched.
    Keep,
    /// Placeholder in Vault, not on GitHub: nothing to push.
    Missing,
    /// On GitHub but not in Vault: will be deleted.
    Delete,
    /// On GitHub but not in Vault, and pruning is off: left untouched.
    Extra,
}

/// `owner/repo` with GitHub's allowed characters.
pub fn parse_repo(arg: &str) -> Result<(String, String)> {
    let ok = |s: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
    };
    match arg.split_once('/') {
        Some((owner, repo)) if ok(owner) && ok(repo) => Ok((owner.into(), repo.into())),
        _ => bail!("invalid repository `{arg}` (expected owner/repo)"),
    }
}

/// GitHub Actions secret names: letters, digits and `_`, not starting with a
/// digit or `GITHUB_`.
pub fn valid_github_name(name: &str) -> bool {
    secrets::valid_env_name(name) && !name.to_ascii_uppercase().starts_with("GITHUB_")
}

/// The `include` metadata as (mount, secret) pairs. Entries are paths on the
/// GitHub mount, or `<mount>:<secret>` for another mount.
pub fn parse_includes(include: &str, mount: &str) -> Result<Vec<(String, String)>> {
    include
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            if s.contains(':') {
                secrets::parse_ref(s)
            } else {
                Ok((mount.to_string(), s.trim_matches('/').to_string()))
            }
        })
        .collect()
}

/// Compare what Vault wants with the secret names GitHub has.
pub fn plan(
    desired: &BTreeMap<String, Entry>,
    existing: &BTreeSet<String>,
    prune: bool,
) -> Vec<(String, Action)> {
    let mut out = Vec::new();
    for (name, entry) in desired {
        let on_github = existing.contains(name);
        let action = match (entry.value == PLACEHOLDER, on_github) {
            (true, true) => Action::Keep,
            (true, false) => Action::Missing,
            (false, true) => Action::Update,
            (false, false) => Action::Add,
        };
        out.push((name.clone(), action));
    }
    for name in existing {
        if !desired.contains_key(name) {
            let action = if prune { Action::Delete } else { Action::Extra };
            out.push((name.clone(), action));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// Merge the includes and the repository's own secret.
async fn desired(
    client: &VaultClient,
    mount: &str,
    repo: &str,
) -> Result<(BTreeMap<String, Entry>, Vec<String>)> {
    let label = format!("{mount}/{repo}");
    let include = match kv2::read_metadata(client, mount, repo).await {
        Ok(meta) => meta
            .custom_metadata
            .and_then(|m| m.get("include").cloned())
            .unwrap_or_default(),
        Err(ClientError::APIError { code: 404, .. }) => bail!("secret {label} not found"),
        Err(e) => return Err(vault::describe(e, &format!("reading metadata of {label}"))),
    };

    let mut sources = Vec::new();
    for (m, s) in parse_includes(&include, mount)? {
        let src = format!("{m}/{s}");
        sources.push((src.clone(), read_existing(client, &m, &s).await?));
    }
    // The repository secret may hold no keys of its own, or have no version.
    let own = read_raw(client, mount, repo).await?.unwrap_or_default();
    sources.push((label, own));

    let mut merged = BTreeMap::new();
    let mut names = Vec::new();
    for (src, raw) in sources {
        for (key, value) in as_strings(&src, raw)? {
            if !valid_github_name(&key) {
                bail!("{src}: `{key}` is not a valid GitHub secret name");
            }
            merged.insert(
                key,
                Entry {
                    value,
                    from: src.clone(),
                },
            );
        }
        names.push(src);
    }
    Ok((merged, names))
}

/// Every `<owner>/<repo>` secret on the mount, skipping `shared/`.
async fn all_repos(client: &VaultClient, mount: &str) -> Result<Vec<String>> {
    let list = |path: String| async move {
        match secrets::list(client, mount, &path).await {
            Ok(names) => Ok(names),
            Err(ClientError::APIError { code: 404, .. }) => Ok(Vec::new()),
            Err(e) => Err(vault::describe(e, &format!("listing {mount}/{path}"))),
        }
    };
    let mut repos = Vec::new();
    for owner in list(String::new()).await? {
        let Some(owner) = owner.strip_suffix('/') else {
            continue;
        };
        if owner == SHARED {
            continue;
        }
        for repo in list(owner.to_string()).await? {
            if !repo.ends_with('/') {
                repos.push(format!("{owner}/{repo}"));
            }
        }
    }
    repos.sort();
    Ok(repos)
}

/// `keys push github <owner/repo> | --all [--dry-run] [--no-prune]`
pub async fn push(repo: Option<&str>, all: bool, dry_run: bool, prune: bool) -> Result<()> {
    let mount = mount();
    let client = vault::authed_client()?;
    let repos = match (repo, all) {
        (Some(r), false) => vec![r.to_string()],
        (None, true) => all_repos(&client, &mount).await?,
        _ => bail!("pass a repository (owner/repo) or --all"),
    };
    if repos.is_empty() {
        bail!("no repositories on {mount}/ (expected <owner>/<repo> secrets)");
    }
    for r in &repos {
        parse_repo(r)?;
    }

    let gh = Github::new()?;
    for (i, r) in repos.iter().enumerate() {
        if i > 0 {
            println!();
        }
        sync_repo(&client, &gh, &mount, r, dry_run, prune).await?;
    }
    if dry_run {
        println!("\nDry run: nothing changed on GitHub");
    }
    Ok(())
}

async fn sync_repo(
    client: &VaultClient,
    gh: &Github,
    mount: &str,
    repo: &str,
    dry_run: bool,
    prune: bool,
) -> Result<()> {
    let (want, sources) = desired(client, mount, repo).await?;
    let have = gh.list(repo).await?;
    let steps = plan(&want, &have, prune);

    println!("{repo}  <- {}", sources.join(", "));
    if steps.is_empty() {
        println!("  (no secrets)");
        return Ok(());
    }
    let width = steps.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
    let key = if dry_run || !steps.iter().any(|(_, a)| writes(a)) {
        None
    } else {
        Some(gh.public_key(repo).await?)
    };

    for (name, action) in &steps {
        let from = want.get(name).map(|e| e.from.as_str()).unwrap_or("");
        let (verb, note) = match action {
            Action::Add => ("add", format!("from {from}")),
            Action::Update => ("update", format!("from {from}")),
            Action::Keep => (
                "keep",
                format!("{PLACEHOLDER} in {from}; GitHub value left as is"),
            ),
            Action::Missing => ("missing", format!("{PLACEHOLDER} in {from}; not on GitHub")),
            Action::Delete => ("delete", "not in Vault".to_string()),
            Action::Extra => ("extra", "not in Vault; kept (--no-prune)".to_string()),
        };
        println!("  {verb:<7} {name:<width$}  {note}");
        if let Some(key) = &key {
            match action {
                Action::Add | Action::Update => gh.put(repo, name, &want[name].value, key).await?,
                Action::Delete => gh.delete(repo, name).await?,
                _ => {}
            }
        }
    }
    Ok(())
}

fn writes(a: &Action) -> bool {
    matches!(a, Action::Add | Action::Update | Action::Delete)
}

/// Encrypt a value for GitHub with a libsodium sealed box.
pub fn seal(public_key: &[u8; 32], value: &str) -> Result<String> {
    let pk = crypto_box::PublicKey::from(*public_key);
    let sealed = pk
        .seal(&mut crypto_box::aead::OsRng, value.as_bytes())
        .map_err(|_| anyhow!("encrypting secret value"))?;
    Ok(B64.encode(sealed))
}

struct RepoKey {
    id: String,
    key: [u8; 32],
}

struct Github {
    http: reqwest::Client,
    api: String,
    token: String,
}

impl Github {
    fn new() -> Result<Self> {
        let api = std::env::var("GITHUB_API_URL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "https://api.github.com".to_string());
        let http = reqwest::Client::builder()
            .user_agent(concat!("keys/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("creating HTTP client")?;
        Ok(Self {
            http,
            api: api.trim_end_matches('/').to_string(),
            token: github_token()?,
        })
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        self.http
            .request(method, format!("{}{path}", self.api))
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
    }

    async fn send(&self, req: reqwest::RequestBuilder, what: &str) -> Result<reqwest::Response> {
        let resp = req
            .send()
            .await
            .with_context(|| format!("{what}: cannot reach GitHub"))?;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        let body: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(200)
            .collect();
        let hint = match status.as_u16() {
            401 => " (GitHub token invalid or expired)",
            403 | 404 => " (repository not found, or the token lacks admin access to its secrets)",
            _ => "",
        };
        bail!("{what}: GitHub returned {status}{hint}: {}", body.trim())
    }

    async fn list(&self, repo: &str) -> Result<BTreeSet<String>> {
        let mut names = BTreeSet::new();
        for page in 1.. {
            let path = format!("/repos/{repo}/actions/secrets?per_page=100&page={page}");
            let what = format!("listing secrets of {repo}");
            let resp = self
                .send(self.request(reqwest::Method::GET, &path), &what)
                .await?;
            let body: serde_json::Value = resp.json().await.with_context(|| what.clone())?;
            let items = body["secrets"]
                .as_array()
                .ok_or_else(|| anyhow!("{what}: unexpected response"))?;
            for s in items {
                if let Some(n) = s["name"].as_str() {
                    names.insert(n.to_string());
                }
            }
            if items.len() < 100 {
                break;
            }
        }
        Ok(names)
    }

    async fn public_key(&self, repo: &str) -> Result<RepoKey> {
        let what = format!("reading the secrets public key of {repo}");
        let path = format!("/repos/{repo}/actions/secrets/public-key");
        let resp = self
            .send(self.request(reqwest::Method::GET, &path), &what)
            .await?;
        let body: serde_json::Value = resp.json().await.with_context(|| what.clone())?;
        let (Some(id), Some(key)) = (body["key_id"].as_str(), body["key"].as_str()) else {
            bail!("{what}: unexpected response");
        };
        let key: [u8; 32] = B64
            .decode(key)
            .ok()
            .and_then(|k| k.try_into().ok())
            .ok_or_else(|| anyhow!("{what}: malformed key"))?;
        Ok(RepoKey {
            id: id.to_string(),
            key,
        })
    }

    async fn put(&self, repo: &str, name: &str, value: &str, key: &RepoKey) -> Result<()> {
        let body = serde_json::json!({
            "encrypted_value": seal(&key.key, value)?,
            "key_id": key.id,
        });
        let path = format!("/repos/{repo}/actions/secrets/{name}");
        let req = self.request(reqwest::Method::PUT, &path).json(&body);
        self.send(req, &format!("setting {name} on {repo}")).await?;
        Ok(())
    }

    async fn delete(&self, repo: &str, name: &str) -> Result<()> {
        let path = format!("/repos/{repo}/actions/secrets/{name}");
        let req = self.request(reqwest::Method::DELETE, &path);
        self.send(req, &format!("deleting {name} from {repo}"))
            .await?;
        Ok(())
    }
}

/// `GH_TOKEN` or `GITHUB_TOKEN`, else the GitHub CLI's token.
fn github_token() -> Result<String> {
    for var in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(t) = std::env::var(var)
            && !t.trim().is_empty()
        {
            return Ok(t.trim().to_string());
        }
    }
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .stderr(std::process::Stdio::null())
        .output();
    match out {
        Ok(o) if o.status.success() && !o.stdout.is_empty() => {
            Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
        }
        _ => bail!("no GitHub token: set GH_TOKEN or log in with `gh auth login`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(value: &str) -> Entry {
        Entry {
            value: value.into(),
            from: "github/x".into(),
        }
    }

    #[test]
    fn repos_are_owner_slash_name() {
        assert_eq!(
            parse_repo("acme/site.example").unwrap(),
            ("acme".into(), "site.example".into())
        );
        assert!(parse_repo("acme").is_err());
        assert!(parse_repo("acme/").is_err());
        assert!(parse_repo("a/b/c").is_err());
        assert!(parse_repo("acme/re po").is_err());
    }

    #[test]
    fn github_names() {
        assert!(valid_github_name("NTFY_TOKEN"));
        assert!(!valid_github_name("GITHUB_TOKEN"));
        assert!(!valid_github_name("github_x"));
        assert!(!valid_github_name("1X"));
    }

    #[test]
    fn includes_default_to_the_github_mount() {
        assert_eq!(
            parse_includes(" shared/ntfy, other:team/x ,,", "github").unwrap(),
            vec![
                ("github".into(), "shared/ntfy".into()),
                ("other".into(), "team/x".into())
            ]
        );
        assert!(parse_includes("", "github").unwrap().is_empty());
    }

    #[test]
    fn plan_covers_every_case() {
        let desired = BTreeMap::from([
            ("NEW".to_string(), entry("v")),
            ("OLD".to_string(), entry("v")),
            ("PEND".to_string(), entry(PLACEHOLDER)),
            ("GONE".to_string(), entry(PLACEHOLDER)),
        ]);
        let existing = BTreeSet::from(["OLD".to_string(), "PEND".to_string(), "X".to_string()]);
        assert_eq!(
            plan(&desired, &existing, true),
            vec![
                ("GONE".into(), Action::Missing),
                ("NEW".into(), Action::Add),
                ("OLD".into(), Action::Update),
                ("PEND".into(), Action::Keep),
                ("X".into(), Action::Delete),
            ]
        );
        assert_eq!(
            plan(&desired, &existing, false).last().unwrap(),
            &("X".into(), Action::Extra)
        );
    }

    #[test]
    fn sealed_values_open_with_the_secret_key() {
        let sk = crypto_box::SecretKey::generate(&mut crypto_box::aead::OsRng);
        let sealed = seal(sk.public_key().as_bytes(), "s3cret").unwrap();
        let opened = sk.unseal(&B64.decode(sealed).unwrap()).unwrap();
        assert_eq!(opened, b"s3cret");
    }
}
