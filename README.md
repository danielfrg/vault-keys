# keys

A small CLI to fetch and manage secrets stored in HashiCorp Vault.

Secrets should only exist when explicitly loaded, ideally only inside the
one process that needs them, instead of being exported from shell startup files. This CLI solves that problem.

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
keys env ai

# Read a secret from another mount: <mount>:<secret>
keys env homelab:aws
```

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
| `keys env <secret>...` | Load the secrets' keys into the current interactive shell |
| `keys set <secret> <KEY>` | Store a value using hidden input |
| `keys edit <secret>` | Edit a secret in `$EDITOR` and write it back |
| `keys get` / `list` / `rm` | Inspect and remove secrets (key names only by default) |
| `keys export <secret> --format dotenv` | Export a secret in another format |
| `keys status` | Show login identity, policies and token expiry |
| `keys doctor` | Diagnose Vault/client/token/permission problems |
| `keys push github <owner/repo> <KEY>` | Push a stored value to a GitHub Actions secret |

A secret argument is `<secret>` (on the default mount) or `<mount>:<secret>`.
