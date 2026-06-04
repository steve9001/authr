# authr

**authr** is a macOS menu-bar (tray) app for Time-based One-Time Passwords (TOTP).
It lives in the menu bar — click the icon and a small popover drops down with your
current 6-digit codes and a live countdown. Click a code to copy it.

It is built with [Tauri 2](https://tauri.app) (Rust backend) and
[SvelteKit](https://svelte.dev) (UI). All TOTP codes are computed in Rust; the raw
secrets never cross into the webview.

> **Note:** authr was previously a command-line / terminal-UI tool. That version (the
> `authr` CLI and TUI) has been removed entirely — the menu-bar app is now the only
> shell. There is no command-line binary.

## Features

- **Menu-bar popover** — no Dock icon, no window chrome; toggles from the tray, anchors
  under the icon, and auto-hides when it loses focus or you press `Esc`.
- **Always-visible codes** — current 6-digit codes (grouped 3 + 3, monospace) with a single
  global countdown bar driven by the real 30-second period boundary.
- **Search** — the popover opens focused on a filter box; type to filter accounts instantly.
- **Tap to copy** — click any row to copy its code; a brief "✓ copied!" flash confirms.
- **Account management** — add (name + base32 secret, spaces ignored), inline rename, and
  delete. Deleting shows a "no recovery" confirmation; the secret is never displayed.
- **Encryption at rest** — optionally protect the store with a password. The vault is
  encrypted with [`age`](https://github.com/str4d/rage) (scrypt KDF + AEAD). The password is
  **unrecoverable by design** — if you forget it, the accounts can't be recovered. Once you
  unlock for a session, the passphrase is held in memory until you Quit, so adding or
  renaming re-encrypts silently without re-prompting.
- **Backup & restore** — export all accounts to a single `.authr` file. The backup gets its
  **own** password (independent of the at-rest password); leaving it blank requires an
  explicit "plain text" confirmation. Import is a **one-tap additive merge** keyed on the
  TOTP secret — it adds what's new, keeps what you have, and **never deletes or overwrites**.

> **Backup caveat:** import is additive *union*, not delete-aware sync. Re-importing an old
> backup can bring back an account you deliberately deleted on this device.

## Security model

- Secrets never cross the Rust ⇄ webview bridge — no command returns a base32 secret to the
  UI, including the delete flow. The only way to read a stored secret is to export a
  *plaintext* backup and read the JSON.
- Codes are generated in Rust; the already-computed 6-digit value is the only
  account-derived data the UI sees.
- Restrictive CSP, local assets only, and a narrowly scoped capability set (window show/hide,
  positioner, clipboard-write, and user-selected `dialog`/`fs` for backup).

## Requirements

- **macOS** (the app is menu-bar-only / `LSUIElement`).
- [Rust](https://rustup.rs) (stable, edition 2021).
- [Node.js](https://nodejs.org) and [pnpm](https://pnpm.io) (`npm i -g pnpm`).

## Build & run from source

```bash
git clone git@github.com:steve9001/authr.git
cd authr/authr_app
pnpm install          # first time only
pnpm tauri dev        # compile the Rust backend, start Vite, and launch the tray app
```

`pnpm tauri dev` runs with hot reload on the Svelte side. To produce an optimized bundle:

```bash
cd authr_app
pnpm tauri build      # outputs to authr_app/src-tauri/target/release/bundle/
```

> Don't run `pnpm dev` (Vite) on its own — that only serves the frontend in a browser. The
> tray/menu-bar behavior comes from the Rust side, so you need the `tauri` wrapper.

## Distribution

Signed, notarized distribution is **planned but not yet available** — for now, build from
source as above. Two channels are intended:

- a Developer ID–signed, notarized **DMG** (for GitHub releases), and
- an **App Store** build (App Sandbox + Apple Distribution).

Until those land, there is no prebuilt download.

## Project layout

```
authr/
  authr_core/            # shared Rust lib: model, storage, TOTP, validation, vault/crypto
  authr_app/
    src-tauri/           # Rust: tray, lifecycle, Tauri commands
    src/                 # SvelteKit UI (the screens as client-side routes)
```

The UI screens are client-side routes inside the single popover window:

| Route                | Screen                                       |
|----------------------|----------------------------------------------|
| `/`                  | Main list (codes, countdown, search)         |
| `/settings`          | Settings hub (Security / Backup / Accounts)  |
| `/settings/add`      | Add account                                  |
| `/settings/password` | Set / change encryption password             |
| `/settings/backup`   | Back up accounts                             |
| `/unlock`            | Unlock gate (when encrypted + locked)        |

## Testing

```bash
# Rust: core unit/integration tests + Tauri command tests
cargo test

# Svelte component tests (headless, mocked backend)
cd authr_app && pnpm test
```

There is no WebDriver / full-E2E suite — driving a real WKWebView is unavailable on macOS.
Coverage is split between Svelte component tests (UI behavior against a mocked backend) and
Rust command tests (the real load → mutate → save round-trip over a temp-dir store).

## License

MIT
