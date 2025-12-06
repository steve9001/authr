# authr

authr is a Time-based One-Time Password (TOTP) application written in Rust. It features both a command line interface (CLI) for scripting and quick access, and a curses-based terminal user interface (TUI) for interactive use.

## Installation

To install authr, clone the repository and install using cargo:

```bash
git clone <repository-url>
cd authr
cargo install --path ./authr_cli
```

## Usage

### Command Line Interface

authr supports the following commands:

- List all accounts:
  `authr list`

- Add a new account:
  `authr add <name> <secret>`
  Example: `authr add MyService MakeSureSecretIsLongEnough`

- Remove an account:
  `authr remove <name>`

- Show the current TOTP for an account:
  `authr show <name>`

- Show the secret key for an account:
  `authr show <name> --seed`

### TUI

Running `authr` without any arguments opens the interactive TUI:

`authr`

Controls:
- Type to filter the list of accounts.
- Backspace to delete the filter.
- Esc to exit the application.

## Roadmap

Future planned features include:

- Encrypted storage for account secrets.
- Cloud storage support for syncing accounts across devices.
