# authr → Direction E Tray App: Unified Tactical Plan

This is the **single authoritative build plan**. It merges:

- [`MIGRATION_PLAN.md`](./MIGRATION_PLAN.md) — *how* we re-platform (Tauri + Svelte,
  kill the TUI/egui, tray lifecycle, signing). The mechanics.
- [`direction-e/`](./direction-e/) — *what* we are building (screens, flows, data model,
  the full feature set). The product.
- [`BUILDING_TRAY_APPS.md`](./BUILDING_TRAY_APPS.md) — the general field guide; `§` refs
  point into it.

Where the two source docs disagree, **this document wins** and the resolution is called out
in §2. When the migration plan and Direction E both describe the same thing, this plan does
not repeat the detail — it points back.

---

## 0. What changed vs. the original migration plan

The migration plan was written assuming a **read-only desktop tray + management-later-for-Android**
shape. Two facts collapse that sequencing:

1. **Android is dropped** (per direction). So "defer account management until Android needs
   it" has no anchor — the desktop app is the *only* shell, and Direction E requires full
   management, encryption, and backup **on the desktop**.
2. **Direction E is the target**, and it puts encryption + backup + a Settings hub in scope.
   The migration plan explicitly listed those as out of scope.

Net effect: the migration plan's Phases 0–3 survive (re-platform, tray, read-only list,
signing), Phase 4 (Android) is **deleted**, and Phase 5 (UI management) is **promoted and
expanded** into three real phases (management, encryption, backup) that we ship on macOS.

---

## 1. Locked decisions (carried forward, unchanged)

| Decision | Choice | Source |
|---|---|---|
| Tray/UI framework | **Tauri 2** | migration §1 |
| Web UI tech | **Svelte** | migration §1 |
| TUI (ratatui) | **Remove** | migration §1 |
| egui/eframe GUI | **Remove** | migration §1 |
| CLI (`list`/`add`/`remove`/`show`) | **Keep**, stays interoperable with the GUI store | migration §1 |
| Distribution | **Sign + notarize for outside distribution** | migration §1 |
| Panel shape | One small single-column popover, ~**344×568** baseline; tall screens scroll | direction-e README |

## 2. New decisions (this plan resolves)

| # | Question | Decision | Rationale |
|---|---|---|---|
| D1 | Android | **Out of scope.** Drop the `#[cfg(desktop)]` discipline and Phase 4 entirely. | Per direction. The tray code can stay un-gated since desktop is the only target. |
| D2 | Read-only UI? | **No.** The UI is the full manager (add/rename/delete/encrypt/backup). | Direction E requires it; there is no other shell to manage from on a phone. |
| D3 | Multiple windows per E screen? | **No.** All six E screens are **client-side routes inside the single `main` popover window.** | A tray popover is one window (migration §3.1). Navigation = Svelte view state, not new OS windows. |
| D4 | "Secrets never cross the bridge" vs. Direction E's delete modal showing the raw secret | **Scoped exception.** Add one narrow `reveal_secret(name)` command, reachable **only** from the delete-confirm flow. | This is a *direct* conflict (migration §5 vs. direction-e screens.md "Delete-account confirmation"). Direction E deliberately surfaces the secret so the user can copy it before destroying the account. It is an explicit, user-initiated reveal to the already-authenticated local user — acceptable. The blanket "no secret command" rule is relaxed to "no *unsolicited* secret command." `show --seed` stays CLI-only as before. |
| D5 | Encryption primitive | **Recommend `age` (passphrase recipient)** for both the at-rest store and the backup file; alternative is RustCrypto (`argon2` KDF + `XChaCha20Poly1305`). **Decision point — confirm before Phase 4.** | One library covers both at-rest and the `.authr` backup as the same encrypted blob; passphrase mode matches the "one password, unrecoverable" model. |
| D6 | Backup file format | **`.authr` = the serialized store**, encrypted with the user's password **iff** encryption is on, else plaintext JSON. | direction-e: backup is "one file with all accounts," encrypted only when encryption is enabled. Reusing the at-rest format keeps one code path. |
| D7 | Session lock policy | **Unlock persists for the running process** (until Quit), so focus-loss auto-hide does **not** force re-entry of the password. Optional idle-timeout re-lock later. | Re-prompting every time the popover loses focus would be unusable for a menu-bar app you poke all day. |
| D8 | Distribution channels | **Both**: (a) Developer ID + notarized **DMG** for GitHub, and (b) **App Store** build. These diverge (different cert + App Sandbox) — see §7. | Per direction ("GitHub and/or the App Store"). |
| D9 | Visual style | Direction E is **layout-only**; pick a baseline now: **dark theme, compact, monospace codes**, carry the current egui popover's feel. | direction-e README defers styling to the implementer; we lock a baseline so screens land consistently. |

---

## 3. Target architecture

### 3.1 Workspace layout (unchanged from migration §2)

```
authr/
  authr_core/            # shared lib: model, storage, totp, + NEW vault/crypto
  authr_cli/             # bin "authr": clap CLI ONLY (list/add/remove/show)
  authr_app/
    src-tauri/           # Rust: tray, lifecycle, commands
    src/                 # Svelte UI: the six E screens as client routes
    ...
```

Top-level `Cargo.toml` `members` gains `authr_app/src-tauri`; nothing else is dropped from
the workspace.

### 3.2 `authr_core` changes

Building on migration §3, **plus** the encryption seam:

1. **Inject the storage dir** — `load_accounts`/`save_accounts` take a `&Path` (or a
   `Storage { dir }`). CLI passes `ProjectDirs` config dir; Tauri passes `app_config_dir()`.
   *(migration §3.1 — keep this even though Android is gone; it's the clean seam for the
   encrypted store and for tests using `tempfile`.)*
2. **View projections** — `AccountView { name, issuer }` and
   `CodeView { name, issuer, code, period_seconds, valid_until_unix }`. The base32 `secret`
   never appears in a view type. *(migration §3.2)*
3. **TOTP validity** — extend `totp.rs` with the period (30s) and a
   `generate_with_validity() -> (code, valid_until_unix)` so the UI's single global
   countdown bar (direction-e) is driven by real period boundaries, not a guess.
4. **NEW — `vault`/`crypto` module** (lands in Phase 4, but design the seam now). The store
   on disk is one of:
   - *plaintext* `accounts.json` (today's format — keep reading it for back-compat), or
   - *encrypted* vault (password-derived key over the same serialized accounts).

   `storage` gains an in-memory `Vault` that holds either the decrypted accounts or a
   "locked" marker. `load`/`save` route through it. The **CLI must also learn this** (D5):
   when the store is encrypted, CLI commands prompt for the password (reuse `rpassword`,
   already a dep). Same store, both frontends stay interoperable (migration §5).

`model.rs` keeps `Account { name, issuer, secret }`. Direction E's data model doesn't use
`issuer`, but we keep it optional for CLI back-compat; the UI just shows `name`.

### 3.3 The complete command surface (Rust ⇄ webview)

Narrow, explicitly registered (migration §5.1). Grouped by the phase that introduces it:

| Command | Returns | Phase | Notes |
|---|---|---|---|
| `list_accounts()` | `Vec<AccountView>` | 2 | name (+issuer) only |
| `get_codes()` | `Vec<CodeView>` | 2 | codes computed in Rust |
| `add_account(name, secret)` | `Result<AccountView,String>` | 3 | validates by generating a code (reuse CLI logic, commands.rs:35); spaces stripped from secret |
| `rename_account(name, new_name)` | `Result<(),String>` | 3 | inline rename |
| `delete_account(name)` | `Result<(),String>` | 3 | permanent |
| `reveal_secret(name)` | `Result<String,String>` | 3 | **D4 scoped exception** — delete-confirm flow only |
| `encryption_status()` | `{ enabled, locked }` | 4 | drives Settings + unlock gating |
| `set_password(new)` / `change_password(old,new)` | `Result<(),String>` | 4 | encrypts the store |
| `unlock(password)` | `Result<(),String>` | 4 | decrypts in-session (D7) |
| `export_backup(dest_path)` | `Result<(),String>` | 5 | frontend `dialog` plugin picks path; Rust writes the (encrypted-when-on) blob |

Copying a code: the code is already in the webview, so the **frontend clipboard-write**
plugin handles E1/E2 tap-to-copy. A Rust `copy_code` is optional and only worth it if we add
clipboard auto-clear (§6).

### 3.4 Screen → route map (single window, D3)

| E screen | Route | Brought up by |
|---|---|---|
| E1 main list (countdown · search+gear · always-visible code rows) | `/` | tray click |
| E2 copy feedback | `/` (transient row state) | tap a row |
| E3 settings hub | `/settings` | gear button |
| E4 set/change password | `/settings/password` | Encryption row |
| E5 add account | `/settings/add` | "Add account" |
| E6 backup (bottom sheet, or plain dialog) | `/settings/backup` | "Back up accounts" |
| Unlock prompt (implied by E4 "you'll enter it to open Authr") | `/unlock` | app open while encrypted+locked |
| Delete-confirm modal | overlay on `/settings` | trash affordance |

---

## 4. Feature → phase coverage matrix

Every Direction E acceptance item + every tray behavior, mapped to where it lands. This is
the checklist that says "the plan covers all the new features and appearance."

| Feature / behavior | Source | Phase |
|---|---|---|
| Kill TUI + egui; CLI-only `authr`; green build; empty Tauri shell | migration §4 P0 | **0** |
| Storage-dir injection + view types + TOTP validity | migration §3 | **1** |
| Tray icon (template), toggle popover, positioner, auto-hide, Escape, Quit | migration §4 P1, guide §3–4 | **2** |
| No Dock icon / Accessory policy / LSUIElement | migration §4 P1, guide §2.1 | **2** |
| E1 countdown bar (single global timer) | direction-e | **2** |
| E1 search+gear bar at top; immediate filter; always-focused input | direction-e | **2** |
| E1 always-visible 6-digit codes, grouped 3+3; no masking/eye/reveal | direction-e | **2** |
| E2 tap-row-to-copy + "✓ copied!" flash, reverts | direction-e | **2** |
| E3 Settings hub (Security / Backup / Accounts·N sections) | direction-e | **3** |
| E5 add account (name + base32 secret, spaces ignored) | direction-e | **3** |
| Inline rename from manage row | direction-e | **3** |
| Delete-confirm modal showing the secret to copy first | direction-e (D4) | **3** |
| E4 set/change password; unrecoverable warning | direction-e | **4** |
| Encrypted store at rest; unlock-on-open gate; CLI learns the password | direction-e | **4** |
| E6 backup → single `.authr` file, encrypted when enabled, via save/share | direction-e | **5** |
| Visual design pass (dark, compact, final look/feel) | D9 | **6** |
| Developer ID signing + hardened runtime + notarization + DMG | migration §4 P3, guide §2.3 | **7** |
| App Store build (App Sandbox + Apple Distribution) | D8 | **7** |
| README rewrite | migration §6 | **8** |

---

## 5. Phases (re-sequenced)

Each phase ends green and, from Phase 2 on, demoable.

### Phase 0 — Scaffold & teardown
Exactly migration §4 Phase 0. Scaffold `authr_app` (Tauri + Svelte template), wire
`src-tauri` into the workspace. **Delete** `tui_interface.rs`, `gui_interface.rs`, the
`tui`/`gui` features, the `gui-worker` subcommand + no-arg spawn branch in `main.rs`
(main.rs:39–85), and the `ratatui`/`crossterm`/`eframe`/`egui`/`image` deps
(authr_cli/Cargo.toml:10–24). Move `assets/icon.png` under `authr_app`. Bare `authr` prints
help.
**Exit:** `cargo build` green, CLI-only `authr`, empty Tauri shell opens.

### Phase 1 — core refactor (the enabling seam)
§3.2 items 1–3. Inject storage dir; add `AccountView`/`CodeView`; add TOTP validity. Design
(but don't yet implement) the `vault` seam so Phase 4 slots in without churning `storage`'s
callers. Update the CLI call sites and the `tempfile`-based integration tests.
**Exit:** `cargo test` green; CLI behavior unchanged; core exposes view + validity APIs.

### Phase 2 — macOS tray + E1/E2 (the "it feels like the app" milestone)
Backend lifecycle per migration §4 Phase 1 / guide §3–4, §8: accessory policy + LSUIElement;
single pre-declared `main` popover (`visible:false`, `decorations:false`, `resizable:false`,
`skipTaskbar:true`); template tray icon; left-click toggles + anchors via
`tauri-plugin-positioner` (`TrayCenter`); auto-hide on focus loss; Escape-to-hide; Quit menu.
Commands: `list_accounts`, `get_codes`.
Svelte E1: countdown bar driven by `valid_until_unix` (re-fetch on rollover); top search+gear
bar, always-focused filter, immediate substring filter; account rows name-left / code-right
(monospace, 3+3); whole row tap-to-copy via clipboard plugin with E2 "✓ copied!" flash.
Per-window capability is window hide/show + positioner + clipboard-write; restrictive CSP
(guide §5.4). The gear is present but can route to a stub until Phase 3.
**Exit (from migration §4 P1):** clicking the tray toggles a dark popover under the icon;
typing filters; clicking a row copies + flashes; clicking away hides; Quit exits; no Dock icon.

### Phase 3 — Settings hub + account management (E3, E5, rename, delete)
Commands: `add_account`, `rename_account`, `delete_account`, `reveal_secret` (D4). Reuse the
CLI's validate-by-generating-a-code logic (commands.rs:35–38) and duplicate-name check.
Svelte: E3 settings hub with Security / Backup / Accounts·N sections (Backup + Encryption rows
can be stubs/"coming next" until Phases 4–5, or build the rows now and wire later); E5 add
form (name + monospace secret field, spaces ignored, hint text); inline rename on manage rows;
delete-confirm modal that calls `reveal_secret` to show the base32 before destroying, with the
"no recovery" warning. Grant the `dialog`/clipboard capability bits these screens need.
**Exit:** every Direction E management acceptance item passes except encryption/backup; the
CLI and GUI still read/write the same plaintext store.

### Phase 4 — encryption (E4 + unlock gate)
Confirm D5 first. Implement `authr_core::vault`: KDF + AEAD over the serialized accounts;
read-path auto-detects plaintext-vs-encrypted for back-compat. Commands: `encryption_status`,
`set_password`/`change_password`, `unlock`; session stays unlocked until Quit (D7). Svelte: E4
set/change-password screen with the unrecoverable warning; `/unlock` view shown when the app
opens while encrypted+locked (auto-hide still works; reopening within the session does not
re-prompt). **Update the CLI** to prompt for the password when the store is encrypted (reuse
`rpassword`).
**Exit:** enabling encryption rewrites the store encrypted; relaunch requires the password in
both GUI and CLI; wrong password fails cleanly; disabling/among-change works.

### Phase 5 — backup / export (E6)
Command: `export_backup(dest_path)`; frontend `dialog` plugin picks the destination, Rust
writes the `.authr` blob (D6 — encrypted iff encryption on). Svelte: E6 as a bottom sheet over
a dimmed Settings (plain dialog acceptable), file card showing `authr-vault.authr` + the
"🔒 encrypted" note when on, explanation, Save/share + Cancel. Add the `fs`/`dialog` capability
scoped to user-selected paths.
**Exit:** export produces a single file; with encryption on it's unreadable without the
password and round-trips back in (manually, for now — import is not a Direction E screen).

### Phase 6 — visual design & hardening pass
Apply the D9 baseline across all six screens: dark theme, compact spacing, monospace codes,
flash timing, empty/error/locked states that degrade gracefully (missing/corrupt store →
defaults, never panic; guide §6.2). Finalize app icon (full-color bundle) + tray glyph
(monochrome template; migration §7 risk). Verify no Dock icon / no app menu under both
`tauri dev` and a built `.app`. Tune popover size around the 344×568 baseline and confirm E3
scrolls.
**Exit:** the app looks finished and matches Direction E's layout on every screen.

### Phase 7 — signing, notarization, packaging (both channels, D8)
Front-loaded discipline from migration §4 Phase 3 (guide §2.3). **Two diverging paths:**

- **DMG for GitHub:** Developer ID Application cert; **hardened runtime**;
  `Entitlements.plist` with the minimum set (no camera/audio; `network.client` only if/when
  sync ever exists — today none); notarize via `notarytool` + staple; produce a signed,
  notarized DMG. Script the release.
- **App Store:** **App Sandbox** entitlement (`com.apple.security.app-sandbox`) is mandatory;
  backup save/open needs `com.apple.security.files.user-selected.read-write` (Tauri's dialog
  plugin uses the powerbox, so this works under sandbox); signed with **Apple Distribution**
  cert + provisioning profile; uploaded via Transporter/`notarytool`. A menu-bar-only
  (`LSUIElement`) app is allowed on the App Store but expect review scrutiny — note it.

Keep both entitlement sets in the repo; the two builds differ only in cert + sandbox + the
packaging step.
**Exit:** a notarized DMG installs and launches without Gatekeeper warnings; the App Store
build validates in Transporter.

### Phase 8 — README rewrite
Replace TUI/egui docs with tray-app + CLI docs, the encryption/backup model, and the two
install paths (DMG / App Store). Do this last (migration §6).

---

## 6. Security & the Rust ⇄ webview boundary (updated)

Carries migration §5, amended for Direction E:

- **Codes** are computed in Rust and the already-generated 6-digit value is the only
  account-derived data on E1/E2. Copying happens in the webview (the code is already there).
- **Secrets** still don't cross the bridge **except** the single `reveal_secret(name)` for the
  delete-confirm modal (D4) — explicit, user-initiated, delete-flow-only. `show --seed`
  remains CLI-only.
- **Encryption at rest** (Phase 4): the store is locked with the user's password; it's
  unrecoverable by design and the UI states so plainly (E4 warning, delete "no recovery").
  Session unlock persists until Quit (D7).
- **Backup** (Phase 5) is encrypted with the same password when encryption is on (D6); no
  cloud dependency — the app just hands off a file via the platform save dialog.
- **Clipboard hygiene:** copying a code is expected; **clipboard auto-clear after N seconds**
  is a nice-to-have — if we want it, it requires the optional Rust `copy_code` path (§3.3)
  rather than pure-frontend copy. Defer unless desired.
- **Capabilities + CSP:** grant the `main` window only window hide/show, positioner,
  clipboard-write, and (Phases 3+/5) scoped `dialog`/`fs`. Restrictive CSP, local assets only
  (guide §5.4).

---

## 7. Risks & open decision points

- **D5 encryption library** — confirm `age` vs. RustCrypto before Phase 4. Affects the vault
  module and the CLI's unlock path.
- **D4 secret-reveal exception** — confirm we're comfortable with one delete-flow-only command
  returning a secret to the webview. (The design requires the secret be visible to copy.)
- **App Store vs. DMG entitlement drift** — App Sandbox can surprise: anything beyond
  user-selected file access and clipboard needs an entitlement and a review justification.
  Validate the sandbox build early in Phase 7, not at submission.
- **CLI ↔ encrypted store** — once Phase 4 lands, the CLI *must* understand the encrypted
  format or it silently breaks for encrypted users. This is in-scope in Phase 4, flagged here
  so it isn't forgotten.
- **Notarization friction** — standard but fiddly (guide §2.3); starting in Phase 7 with a
  scripted flow avoids a painful retrofit.
- **Tray template glyph** — the current full-color `icon.png` reads poorly in the menu bar;
  need a clean monochrome glyph (Phase 6).
- **Backup import** — Direction E specs *export* (E6) but no import screen. Restoring a
  `.authr` file is therefore a manual/CLI step for now; flag if a UI import is wanted later.

---

## 8. Suggested order of work

0. Teardown + scaffold (green CLI-only build + empty Tauri shell).
1. Core refactor: storage-dir injection, view types, TOTP validity (+ design the vault seam).
2. Tray lifecycle + E1/E2 (countdown, search+gear, always-visible codes, tap-to-copy).
3. Settings hub + management: E3, E5, inline rename, delete-confirm w/ `reveal_secret`.
4. Encryption: E4 + unlock gate + encrypted store + CLI password support.
5. Backup: E6 + `.authr` export.
6. Visual design & hardening pass across all six screens; finalize icons.
7. Signing/notarization → DMG (GitHub) **and** App Store build.
8. README rewrite.

Phases 0–2 reproduce the migration plan's spine; 3–5 are the Direction E feature build that
the migration plan deferred; 6–8 are polish, distribution, and docs.
