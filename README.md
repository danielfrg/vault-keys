# keys

A small and friendly CLI to fetch and manage secrets stored in HashiCorp Vault.

Secrets should only exist when explicitly loaded, ideally only inside the
one process that needs them, instead of being exported from shell startup files.

## Installation

macOS or Linux (amd64 or arm64):

```bash
curl -fsSL https://danielfrg.github.io/vault-keys/install.sh | bash
```

- `KEYS_CHANNEL=main`: newest build from `main` instead of the latest release
- `KEYS_VERSION=1.2.3`: a specific release
- `KEYS_INSTALL_DIR=<dir>`: install somewhere else

From source: `cargo install --path .`

## Releasing

CI (`.github/workflows/ci.yml`) runs fmt, clippy and tests on every push and PR.

- Push to `main`: builds the four binaries and deploys the landing page, `install.sh` and the `main` channel to GitHub Pages.
- Push a tag `vX.Y.Z`: builds the four binaries and publishes a GitHub release with `SHA256SUMS`.

```bash
git tag v0.1.0 && git push origin v0.1.0
```

`bash scripts/test.sh` checks packaging, the installer and the Pages build locally.

## Quickstart

Auth via userpass (saves the token in `~/.vault-token`).

```bash
export VAULT_ADDR=https://vault.example.com

keys login --user <username>
Password for <username>:
```

Load all the keys of a secret as env variables in the current shell:

```bash
# Reads keychain/ai (the default mount is keychain)
eval "$(keys env ai)"

# Read a secret from another mount: <mount>:<secret>
eval "$(keys env personal:aws)"
```

A program can't change its parent shell's environment, so `keys env` prints
`export` statements (POSIX quoting, works in bash and zsh) and your shell
evaluates them. Values never appear in shell history or process arguments, and
`keys env` refuses to print to a terminal. With several secrets, later ones win
on duplicate keys.

## Vault layout and terminology

`keys` uses Vault's terms.

- **Mount**: a KV v2 secrets engine enabled at a path, e.g. `keychain` or `personal`.
- **Secret**: a path on a mount holding key/value pairs: `keychain/<secret>`.
- **Key**: a name inside a secret, e.g. `MY_API_KEY`. It becomes the env variable name.
- **Value**: what the key holds.

`keys` uses a KV v2 mount (default name: `keychain`).
Each secret on the mount holds one key per env variable name.

| key | value |
| --- | --- |
| `KEY_ONE` | `value-1` |
| `KEY_TWO` | `value-2` |

## Configuration

Vault environment variables:

- `VAULT_ADDR`: Vault address, required (e.g. `https://vault.example.com`)
- `VAULT_TOKEN`: optional, overrides `~/.vault-token`

`keys` environment variables:

- `VAULT_USER`: default for `keys login --user`
- `KEYS_MOUNT`: default mount, `keychain` if unset

## Commands

| Command | Purpose |
| --- | --- |
| `keys login --user <name> [--ttl 12h]` | Log in with userpass |
| `keys logout` | Remove the saved token |
| `keys run <secret>... -- <cmd>` | Run a child process with the secrets' keys as env vars (nothing exported to the shell) |
| `keys env <secret>...` | Print `export` statements for the secrets' keys; use as `eval "$(keys env <secret>)"` |
| `keys list [<mount>:][<secret>] [--show-values]` | List the secrets on a mount, or the key names of one secret; values only with `--show-values` |
| `keys set <secret> <KEY>` | Add or update one key, reading the value with hidden input (or from stdin); other keys are kept |
| `keys edit <secret>` | Edit a secret as JSON in `$VISUAL`/`$EDITOR` and write it back (keys you delete are removed) |
| `keys rm <secret> [KEY] [--yes]` | Remove one key, or the whole secret |
| `keys export <secret>... [--format dotenv]` | Print secrets as a `.env` file; redirect it (`keys export ai > .env`), it refuses to print to a terminal |
| `keys status` | Check Vault connectivity, login, policies, token expiry and access to the default mount |
| `keys push github <owner/repo> [--dry-run] [--no-prune]` | Make a repository's GitHub Actions secrets match Vault |
| `keys push github --all [--dry-run]` | The same for every repository on the GitHub mount |

A secret argument is `<secret>` (on the default mount) or `<mount>:<secret>`.

## GitHub Actions secrets

`keys push github` makes Vault the only source of a repository's Actions
secrets. It reads a separate KV v2 mount, `github` by default
(`KEYS_GITHUB_MOUNT` to change it):

| Secret | Holds |
| --- | --- |
| `github:<owner>/<repo>` | The repository's own keys, named like the GitHub secrets |
| `github:shared/<name>` | Keys used by several repositories |

A repository pulls in shared secrets through the `include` custom metadata of
its secret, comma separated. Includes merge in order and the repository's own
keys come last, so they win:

```bash
vault kv metadata put -mount=github -custom-metadata=include=shared/notify acme/site
keys set github:shared/notify NOTIFY_TOKEN
keys set github:acme/site DEPLOY_ZONE

keys push github acme/site --dry-run   # add / update / delete, names only
keys push github acme/site
```

A push sets every key (GitHub never returns values, so they cannot be
compared) and deletes repository secrets that are not in Vault; `--no-prune`
keeps them. A value of exactly `TODO` is a placeholder: it is not pushed and the
existing GitHub secret is left alone, so a repository can move over one key at a
time.

The GitHub token comes from `GH_TOKEN`, `GITHUB_TOKEN`, or `gh auth token`, and
needs admin access to the repository's secrets.
