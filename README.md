# keys

A small and friendly CLI to fetch and manage secrets stored in HashiCorp Vault.

Secrets should only exist when explicitly loaded, ideally only inside the
one process that needs them, instead of being exported from shell startup files.

## Installation

```bash
```

## Quickstart

Auth via userpass (saves the token in `~/.vault-token`).

```bash
export VAULT_ADDR=https://vault.home.arpa

keys login --user <username>
Password for <username>:
```

Load all the keys of a secret as env variables in the current shell:

```bash
# Reads keychain/ai (the default mount is keychain)
eval "$(keys env ai)"

# Read a secret from another mount: <mount>:<secret>
eval "$(keys env homelab:aws)"
```

A program can't change its parent shell's environment, so `keys env` prints
`export` statements (POSIX quoting, works in bash and zsh) and your shell
evaluates them. Values never appear in shell history or process arguments, and
`keys env` refuses to print to a terminal. With several secrets, later ones win
on duplicate keys.

## Vault layout and terminology

`keys` uses Vault's terms.

- **Mount**: a KV v2 secrets engine enabled at a path, e.g. `keychain` or `homelab`.
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
| `keys push github <owner/repo> <KEY>` | TODO: Push a stored value to a GitHub Actions secret |

A secret argument is `<secret>` (on the default mount) or `<mount>:<secret>`.
