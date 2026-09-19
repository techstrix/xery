# Xery

Xery is a small terminal password manager built with [Ratatui](https://ratatui.rs/)
and [Crossterm](https://github.com/crossterm-rs/crossterm). It provides a
keyboard-first interface for creating vaults, unlocking them, saving
credentials, and revealing passwords when needed.

The terminal interface is intentionally small: the password-management logic
is provided by the sibling [`xery-lib`](../xery-lib) crate, while this crate
handles input, navigation, and rendering.

## Requirements

- Rust and Cargo with Rust 2024 edition support
- A local checkout of `xery-lib`

The two repositories must be next to each other:

```text
Documents/
├── xery/
└── xery-lib/
```

The path dependency is declared in [`Cargo.toml`](Cargo.toml):

```toml
xery-lib = { path = "../xery-lib" }
```

## Run

From the `xery` directory:

```bash
cargo run
```

Xery creates or opens `vaults.db` in the process working directory. Keep this
file private and back it up securely if it contains important vaults.

## Using the interface

### Vault screen

| Key | Action |
| --- | --- |
| `↑` / `↓` or `k` / `j` | Select a vault |
| `Enter` | Unlock the selected vault |
| `n` | Create a new vault |
| `q` | Quit |

### Unlocked vault

| Key | Action |
| --- | --- |
| `a` | Add a credential |
| `r` | Reveal a saved password |
| `l` | Lock the current vault |

### Forms

| Key | Action |
| --- | --- |
| `Tab` / `↓` | Move to the next field |
| `Shift+Tab` / `↑` | Move to the previous field |
| `Enter` | Confirm the form |
| `Backspace` | Delete the previous character |
| `Esc` | Cancel and return |

Password fields are masked while they are being entered. A revealed password
is shown only until `Enter` or `Esc` is pressed.

## Vault password behavior

Unlocking a vault establishes the active session, but the vault password is
intentionally requested again for sensitive operations:

- creating a credential
- revealing a saved password

This mirrors the API exposed by `xery-lib` and avoids silently reusing the
master password for those operations.

## Storage and security

Xery delegates storage and cryptography to `xery-lib`:

- vault passwords are hashed with Argon2
- saved passwords are encrypted with ChaCha20-Poly1305
- each encrypted password uses a generated salt and nonce
- vault names, credential identifiers, and usernames are stored as database
  metadata and should not be treated as secret

The application does not print passwords during normal operation. Terminal
access, the database file, and the machine running Xery should still be
protected like any other password manager.

## Development

Format and validate the CLI with:

```bash
cargo fmt
cargo check
cargo test
cargo clippy --all-targets -- -D warnings
```

The library has its own tests and remains a separate crate. Do not edit
`xery-lib` from this repository; changes to its API should be made and tested
in that crate first.

## Current scope

The current interface supports:

- creating and listing vaults
- unlocking and locking vaults
- adding credentials
- looking up a credential by identifier
- revealing one password at a time

Credential browsing, editing, and deletion are not currently exposed by the
`xery-lib` API used by this CLI.
