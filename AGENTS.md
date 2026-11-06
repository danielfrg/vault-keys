# AGENTS.md

## Project

`keys` is a Rust CLI for fetching and managing credentials in a
HashiCorp Vault.
See `README.md` for the commands and layout.

## Stack

- Rust, single binary, must work on macOS and Linux
- Vault client: the [`vaultrs`](https://github.com/jmgilman/vaultrs) crate
- Token is read from / compatible with `~/.vault-token`

## Rules

- Never print secret values by default; printing must be an explicit flag.
- Never pass secret values via command-line args or log them.
- Prefer small, boring, predictable behavior over extra features.
- Give clear errors for missing login, expired token, unreachable Vault and
  missing permissions.

## Workflow

- Use `cargo fmt` and `cargo clippy` before finishing a change.
- Run `cargo test` for verification.
- Git worktrees go in `~/code/.worktrees/keys/<name>`, never inside the repo.
