# keys

A small CLI to fetch and manage credentials stored in
HashiCorp Vault 

Credentials should only exist when explicitly loaded, ideally only inside the
one process that needs them, instead of being exported from shell startup files.

## Installation

```bash
```

## Configuration

No config file. `keys` reads the standard Vault environment variables:

- `VAULT_ADDR`: Vault address, required (e.g. `https://vault.example.com`)
- `VAULT_USER`: default for `keys login --user`
- `VAULT_TOKEN`: optional, overrides `~/.vault-token`

## Commands

| Command | Purpose |
| --- | --- |
| `keys login --user <name> [--ttl 12h]` | Log in with userpass (hidden password prompt) and save the token to `~/.vault-token` |
| `keys logout` | Remove the saved token |
| `keys run <set>... -- <cmd>` | Run a child process with secrets as env vars (nothing exported to the shell) |
| `keys env <set>...` | Load secrets into the current interactive shell |
| `keys set <set> <FIELD>` | Store a value using hidden input |
| `keys edit <set>` | Edit a secret in `$EDITOR` and write it back |
| `keys get` / `list` / `rm` | Inspect and remove secrets (names only by default) |
| `keys export <set> --format dotenv` | Export a set in another format |
| `keys status` | Show login identity, policies and token expiry |
| `keys doctor` | Diagnose Vault/client/token/permission problems |
| `keys push github <owner/repo> <NAME>` | Push a stored value to a GitHub Actions secret |

## Vault layout

- KV v2 mount `keychain`, one secret per "set", one field per env var name
  (e.g. `keychain/ai` has `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, ...).
- Auth via userpass; the token lives in `~/.vault-token` (shared with `vault login`).
