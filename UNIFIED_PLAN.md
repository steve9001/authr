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
3. **The CLI is removed entirely.** With encryption, an in-memory unlock session, and backup
   all living in the GUI, a separate `authr` command-line binary has no coherent role — and
   the bundle installs to `/Applications`, so there's nothing for a CLI to add. The app is
   launched only as a bundle.

Net effect: the migration plan's Phases 0–3 survive (re-platform, tray, read-only list,
signing) **but the CLI is deleted, not preserved**; Phase 4 (Android) is **deleted**; and
Phase 5 (UI management) is **promoted and expanded** into three real phases (management,
encryption, backup) that we ship on macOS.

---

## 1. Locked decisions (carried forward, unchanged)

| Decision | Choice | Source |
|---|---|---|
| Tray/UI framework | **Tauri 2** | migration §1 |
| Web UI tech | **Svelte** | migration §1 |
| TUI (ratatui) | **Remove** | migration §1 |
| egui/eframe GUI | **Remove** | migration §1 |
| CLI (`list`/`add`/`remove`/`show`) | **Remove entirely** (was "keep" in migration §1 — overridden, see D10) | this plan |
| Distribution | **Sign + notarize for outside distribution** | migration §1 |
| Panel shape | One small single-column popover, ~**344×568** baseline; tall screens scroll | direction-e README |

## 2. New decisions (this plan resolves)

| # | Question | Decision | Rationale |
|---|---|---|---|
| D1 | Android | **Out of scope.** Drop the `#[cfg(desktop)]` discipline and Phase 4 entirely. | Per direction. The tray code can stay un-gated since desktop is the only target. |
| D2 | Read-only UI? | **No.** The UI is the full manager (add/rename/delete/encrypt/backup). | Direction E requires it; there is no other shell to manage from on a phone. |
| D3 | Multiple windows per E screen? | **No.** All six E screens are **client-side routes inside the single `main` popover window.** | A tray popover is one window (migration §3.1). Navigation = Svelte view state, not new OS windows. |
| D4 | "Secrets never cross the bridge" vs. Direction E's delete modal showing the raw secret | **Don't show the secret on delete.** The delete-confirm modal keeps only the "no recovery" warning; the secret block from direction-e screens.md is dropped. The full no-secret-crossing rule stands — **no command ever returns a secret to the webview.** | Overrides direction-e's delete-confirm card. (A user who genuinely needs to read a stored secret does so by exporting a *plaintext* backup and reading the JSON — that's the only path, and it needs no special UI.) |
| D5 | Encryption primitive | **Locked: the `age` Rust crate.** A pure-Rust crate (from the *rage* project) with a passphrase API — **not** the standalone CLI; we link it directly, no shelling out. It backs both the at-rest store and the backup (D6) through one code path. (RustCrypto `argon2` + `XChaCha20Poly1305` was the alternative; not chosen.) | `age`'s `Encryptor::with_user_passphrase` matches the "one password, unrecoverable" model and gives us scrypt-based KDF + AEAD in one well-reviewed crate. |
| D6 | Backup file & password | **Backup has its own password, independent of the live store.** E6 prompts for a **password + confirm**; if given, the `.authr` file is encrypted with *that* password. If left blank, an explicit **"backing up in plain text"** confirmation is required before exporting plaintext JSON. The backup password is **never** assumed to equal the live-store password. | The app does not retain the live password in a reusable form, and a user may *want* a stronger password for a copy going to the cloud. So backup encryption is always a fresh, user-entered choice — not coupled to the at-rest password. |
| D7 | Session lock & re-encrypt | **Unlock persists for the running process** (until Quit); focus-loss auto-hide does **not** re-prompt. Adding/renaming/deleting re-saves the encrypted file **without** re-prompting, because the passphrase is held in memory after unlock (see §3.2). Optional idle-timeout re-lock later. | Re-prompting on every focus loss or every write would be unusable. Holding the passphrase for the session is what makes silent re-encryption on save possible. |
| D8 | Distribution channels | **Both**: (a) Developer ID + notarized **DMG** for GitHub, and (b) **App Store** build. These diverge (different cert + App Sandbox) — see §7. | Per direction ("GitHub and/or the App Store"). |
| D9 | Visual style | Direction E is **layout-only**; pick a baseline now: **dark theme, compact, monospace codes**, carry the current egui popover's feel. | direction-e README defers styling to the implementer; we lock a baseline so screens land consistently. |
| D10 | The CLI | **Removed entirely.** No `authr` command-line binary; the `authr_cli` crate is deleted. The only artifact is the `.app` bundle (DMG / App Store). | Encryption + session unlock + backup belong to the GUI; a CLI would have to re-implement the unlock UX for no benefit, and the bundle already installs to `/Applications`. |
| D11 | Backup *import* / multi-device | **One-tap additive merge, keyed on the secret. No network or cloud — the `.authr` file is the only transport.** Import *adds* what's new and *keeps* what you have; it **never deletes and never overwrites**. Identity is the immutable TOTP **secret** (not the editable name), so import is rename-safe and idempotent. Two devices that import each other's files converge to the **union** of their accounts. Merge runs in Rust core, so no secret crosses the bridge (D4). The `.authr` format is **unchanged** — this is additive-union, *not* delete-aware sync. | Resolves the §7 "backup import" open item. A snapshot file has no timestamps/tombstones, so deletions and last-writer-wins can't be resolved safely; additive union is the correct, honest primitive over a file. (LWW + tombstones — adding `modified_at` + soft-deletes to the format — was the considered alternative; **not** chosen, to keep the format stable.) |

**D11 one-tap merge rules** (keyed on secret): secret already present locally → **skip** (idempotent; local name/label wins); secret absent + name free → **add as-is**; secret absent + name collides with a *different* secret → **add under a de-duplicated label** (`Name (imported)`). Never deletes; never overwrites. A summary toast reports counts (added / skipped / relabeled). **Trap to surface in-UI:** with no tombstones, importing a snapshot can resurrect an account you deliberately deleted on this device (it's still in the other file) — additive merge can't distinguish "deleted here" from "new there."

---

## 3. Target architecture

### 3.1 Workspace layout (unchanged from migration §2)

```
authr/
  authr_core/            # shared lib: model, storage, totp, validation, + NEW vault/crypto
  authr_app/
    src-tauri/           # Rust: tray, lifecycle, commands
    src/                 # Svelte UI: the six E screens as client routes
    ...
```

Top-level `Cargo.toml` `members` gains `authr_app/src-tauri` and **drops `authr_cli`** (the
crate is deleted, D10). The validate-by-generating-a-code logic currently in
`authr_cli/src/commands.rs` moves into `authr_core` before the crate is removed, so the Tauri
`add_account` command and any tests can call it.

### 3.2 `authr_core` changes

Building on migration §3, **plus** the encryption seam:

1. **Inject the storage dir** — `load_accounts`/`save_accounts` take a `&Path` (or a
   `Storage { dir }`). The Tauri app passes `app_config_dir()`.
   *(migration §3.1 — keep this even though Android and the CLI are gone; it's the clean seam
   for the encrypted store and for tests using `tempfile`.)*
2. **View projections** — `AccountView { name, issuer }` and
   `CodeView { name, issuer, code, period_seconds, valid_until_unix }`. The base32 `secret`
   never appears in a view type. *(migration §3.2)*
3. **TOTP validity** — extend `totp.rs` with the period (30s) and a
   `generate_with_validity() -> (code, valid_until_unix)` so the UI's single global
   countdown bar (direction-e) is driven by real period boundaries, not a guess.
4. **NEW — `vault`/`crypto` module** (lands in Phase 4, but design the seam now). The store
   on disk is one of:
   - *plaintext* `accounts.json` (today's format — keep reading it for back-compat), or
   - *encrypted* vault (the same serialized accounts under `age`'s passphrase encryption).

   `storage` gains an in-memory `Session` that, once unlocked, holds the **passphrase** (a
   zeroized secret string) alongside the decrypted accounts. `load` decrypts on unlock; `save`
   re-encrypts using the in-memory passphrase.

   **Re-encrypting on every write without re-prompting (answers the D7 question):** `age`
   passphrase encryption runs scrypt (the KDF) over `passphrase + a fresh random salt` to
   derive a wrapping key, then AEAD-seals the data; decryption re-derives from the embedded
   salt. So a *write* only needs the passphrase, not any precomputed key. Because we keep the
   passphrase in memory for the session (D7), `add_account` → `save` simply re-runs scrypt
   with a new salt and rewrites the file — **the user is never re-prompted.** The accounts
   file is tiny and adds are occasional, so paying scrypt-on-save (a few hundred ms) is fine;
   if it ever matters we can switch to envelope encryption (a long-lived random data key
   wrapped by the passphrase-derived key) so writes skip the KDF — note it, don't build it.

`model.rs` keeps `Account { name, issuer, secret }`. Direction E's data model doesn't use
`issuer`; we keep it optional only to read existing files, and the UI just shows `name`.

### 3.3 The complete command surface (Rust ⇄ webview)

Narrow, explicitly registered (migration §5.1). Grouped by the phase that introduces it:

| Command | Returns | Phase | Notes |
|---|---|---|---|
| `list_accounts()` | `Vec<AccountView>` | 2 | name (+issuer) only |
| `get_codes()` | `Vec<CodeView>` | 2 | codes computed in Rust |
| `add_account(name, secret)` | `Result<AccountView,String>` | 3 | validates by generating a code (logic moved into `authr_core`); spaces stripped from secret |
| `rename_account(name, new_name)` | `Result<(),String>` | 3 | inline rename |
| `delete_account(name)` | `Result<(),String>` | 3 | permanent; **no secret returned** (D4) |
| `encryption_status()` | `{ enabled, locked }` | 4 | drives Settings + unlock gating |
| `set_password(new)` / `change_password(old,new)` | `Result<(),String>` | 4 | encrypts the store |
| `unlock(password)` | `Result<(),String>` | 4 | decrypts in-session (D7) |
| `export_backup(dest_path, password: Option<String>)` | `Result<(),String>` | 5 | frontend `dialog` plugin picks path; `Some(pw)` → encrypt the backup with **that** password (D6); `None` → plaintext JSON (UI requires an explicit confirmation first) |
| `import_backup(src_path, password: Option<String>)` | `Result<ImportSummary,String>` | 5 | one-tap additive merge keyed on the secret (D11); `Some(pw)` decrypts an encrypted `.authr`; **never deletes/overwrites**; returns counts (`{ added, skipped, relabeled }`) for the toast — **no secret crosses the bridge** |

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
| Delete-confirm modal (no-recovery warning only; **no secret shown**, D4) | overlay on `/settings` | trash affordance |

---

## 4. Feature → phase coverage matrix

Every Direction E acceptance item + every tray behavior, mapped to where it lands. This is
the checklist that says "the plan covers all the new features and appearance."

| Feature / behavior | Source | Phase |
|---|---|---|
| Kill TUI + egui **+ the whole CLI crate**; green build; empty Tauri shell | migration §4 P0 + D10 | **0** |
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
| Delete-confirm modal (no-recovery warning; **secret NOT shown**, D4) | this plan / direction-e | **3** |
| E4 set/change password; unrecoverable warning | direction-e | **4** |
| Encrypted store at rest; unlock-on-open gate; silent re-encrypt on write (D7) | direction-e | **4** |
| E6 backup → single `.authr` file with its **own** password+confirm (or confirmed plaintext), via save/share (D6) | direction-e + D6 | **5** |
| Import / restore → one-tap additive merge from a `.authr` file, keyed on secret, never deletes (D11) | this plan / D11 | **5** |
| Visual design pass (dark, compact, final look/feel) | D9 | **6** |
| Developer ID signing + hardened runtime + notarization + DMG | migration §4 P3, guide §2.3 | **7** |
| App Store build (App Sandbox + Apple Distribution) | D8 | **7** |
| README rewrite | migration §6 | **8** |

---

## 5. Phases (re-sequenced)

Each phase ends green and, from Phase 2 on, demoable.

### Phase 0 — Scaffold & teardown
Scaffold `authr_app` (Tauri + Svelte template), wire `src-tauri` into the workspace. First,
**lift the validate-by-generating-a-code logic and duplicate-name check out of
`authr_cli/src/commands.rs` into `authr_core`** (so it survives the CLI's deletion). Then
**delete the entire `authr_cli` crate** and drop it from the workspace `members` — that
removes `main.rs`, `commands.rs`, `tui_interface.rs`, `gui_interface.rs`, the `tui`/`gui`
features, the `gui-worker` subcommand, and the `ratatui`/`crossterm`/`eframe`/`egui`/`image`/
`clap`/`rpassword` deps in one stroke. Move `assets/icon.png` under `authr_app`. There is no
longer an `authr` command-line binary (D10).
**Exit:** `cargo build` green; workspace is `authr_core` + `authr_app/src-tauri`; empty Tauri
shell opens.

### Phase 1 — core refactor (the enabling seam)
§3.2 items 1–3. Inject storage dir; add `AccountView`/`CodeView`; add TOTP validity; land the
validation logic moved out of the old CLI. Design (but don't yet implement) the `vault` seam
so Phase 4 slots in without churning `storage`'s callers. Port the `tempfile`-based tests to
exercise `authr_core` directly (they previously drove the CLI binary).
**Exit:** `cargo test` green; core exposes view + validity + validation APIs.

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
Commands: `add_account`, `rename_account`, `delete_account` — all calling the validation +
duplicate-name logic now in `authr_core`. **No `reveal_secret` (D4).**
Svelte: E3 settings hub with Security / Backup / Accounts·N sections (Backup + Encryption rows
can be stubs/"coming next" until Phases 4–5, or build the rows now and wire later); E5 add
form (name + monospace secret field, spaces ignored, hint text); inline rename on manage rows;
delete-confirm modal with the **"no recovery" warning only — the secret is not displayed**
(deviates from direction-e screens.md's delete card by design). Grant the clipboard capability
these screens need.
**Exit:** every Direction E management acceptance item passes except encryption/backup; the
store is still plaintext on disk.

### Phase 4 — encryption (E4 + unlock gate)
Implement `authr_core::vault` on `age`'s passphrase API (D5, locked): scrypt KDF + AEAD
over the serialized accounts; read-path auto-detects plaintext-vs-encrypted for back-compat.
Commands: `encryption_status`, `set_password`/`change_password`, `unlock`. The unlocked session
holds the passphrase in memory until Quit (D7), so `add`/`rename`/`delete` re-encrypt on save
**without re-prompting** (§3.2 item 4). Svelte: E4 set/change-password screen with the
unrecoverable warning; `/unlock` view shown when the app opens while encrypted+locked
(auto-hide still works; reopening within the session does not re-prompt).
**Exit:** enabling encryption rewrites the store encrypted; relaunch requires the password;
adding an account while unlocked silently re-encrypts; wrong password fails cleanly;
change-password re-encrypts under the new passphrase.

### Phase 5 — backup / export + import (E6 + import)
Commands: `export_backup(dest_path, password)` (D6) and `import_backup(src_path, password)` (D11).
The backup gets its **own** password,
independent of the live store: E6 presents a **password + confirm** pair; the frontend
`dialog` plugin picks the destination; Rust writes the `.authr` file encrypted (via the same
`age` passphrase path as the vault) with the supplied password. If the user leaves the password
blank, the UI requires an explicit **"export in plain text"** confirmation, and Rust writes
plaintext JSON. Svelte: E6 as a bottom sheet over a dimmed Settings (plain dialog acceptable) —
file card showing `authr-vault.authr`, the password+confirm fields with the encrypted/plaintext
state, explanation, Save/share + Cancel. Add the `fs`/`dialog` capability scoped to
user-selected paths.

**Import (D11):** an "Import accounts" row in E3's Backup section opens the `dialog` plugin to
pick a `.authr` file; if it's encrypted, prompt for *that file's* password (independent of the
live store). The merge runs in Rust core — additive, keyed on the secret, idempotent, **never
deletes or overwrites** (rules in D11). `import_backup` returns `{ added, skipped, relabeled }`;
the UI shows a one-tap result toast (e.g. *"Imported 9 new accounts"*) — no review screen, no
secret returned. If the store is encrypted+unlocked, the merged result is re-encrypted on save
via the in-memory passphrase (D7). One honest caveat to word into the Backup section: import is
**additive union, not delete-aware sync**, so a re-imported snapshot can resurrect a locally
deleted account.

**Exit:** export with a password produces a single file unreadable without that password and
**round-trips back in via `import_backup`**; export without a password requires the plaintext
confirmation and yields readable JSON; importing a file whose accounts already exist locally is a
no-op (idempotent), and importing new accounts merges them without touching existing ones.

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
Replace the TUI/egui/CLI docs with tray-app docs, the encryption/backup model, and the two
install paths (DMG / App Store). Note the CLI is gone. Do this last (migration §6).

---

## 6. Security & the Rust ⇄ webview boundary (updated)

Carries migration §5, amended for Direction E:

- **Codes** are computed in Rust and the already-generated 6-digit value is the only
  account-derived data on E1/E2. Copying happens in the webview (the code is already there).
- **Secrets never cross the bridge — no exceptions** (D4). No command returns a base32 secret
  to the webview, including the delete flow. The only way to read a stored secret is to export
  a *plaintext* backup and read the JSON.
- **Encryption at rest** (Phase 4): the store is locked with the user's password; it's
  unrecoverable by design and the UI states so plainly (E4 warning, delete "no recovery").
  Session unlock persists until Quit (D7); the passphrase is held zeroized in memory for the
  session so writes re-encrypt without re-prompting.
- **Backup** (Phase 5) is encrypted with its **own** password entered at export time —
  independent of the at-rest password (D6) — or, with explicit confirmation, exported as
  plaintext. No cloud dependency; the app just hands off a file via the platform save dialog.
- **Clipboard hygiene:** copying a code is expected; **clipboard auto-clear after N seconds**
  is a nice-to-have — if we want it, it requires the optional Rust `copy_code` path (§3.3)
  rather than pure-frontend copy. Defer unless desired.
- **Capabilities + CSP:** grant the `main` window only window hide/show, positioner,
  clipboard-write, and (Phases 3+/5) scoped `dialog`/`fs`. Restrictive CSP, local assets only
  (guide §5.4).

---

## 7. Risks & open decision points

- **App Store vs. DMG entitlement drift** — App Sandbox can surprise: anything beyond
  user-selected file access and clipboard needs an entitlement and a review justification.
  Validate the sandbox build early in Phase 7, not at submission.
- **Notarization friction** — standard but fiddly (guide §2.3); starting in Phase 7 with a
  scripted flow avoids a painful retrofit.
- **Tray template glyph** — the current full-color `icon.png` reads poorly in the menu bar;
  need a clean monochrome glyph (Phase 6).
- **Backup import** — *resolved (D11).* Direction E speced only *export*; we add a one-tap
  additive `import_backup` in Phase 5 (merge keyed on the secret, never deletes). Remaining
  watch-item: it's additive *union*, not delete-aware sync — re-importing an old snapshot can
  resurrect a deleted account. If true sync is ever wanted, that's the LWW + tombstones format
  upgrade rejected in D11, and it changes the `.authr` format.

---

## 8. Suggested order of work

0. Teardown + scaffold: delete the `authr_cli` crate (after lifting its validation into core);
   green build of `authr_core` + empty Tauri shell.
1. Core refactor: storage-dir injection, view types, TOTP validity, ported validation
   (+ design the vault seam).
2. Tray lifecycle + E1/E2 (countdown, search+gear, always-visible codes, tap-to-copy).
3. Settings hub + management: E3, E5, inline rename, delete-confirm (no secret shown).
4. Encryption: E4 + unlock gate + encrypted store; silent re-encrypt on write.
5. Backup: E6 + `.authr` export with its own password (or confirmed plaintext); one-tap additive
   import/merge keyed on the secret (D11).
6. Visual design & hardening pass across all six screens; finalize icons.
7. Signing/notarization → DMG (GitHub) **and** App Store build.
8. README rewrite.

Phases 0–2 reproduce the migration plan's spine; 3–5 are the Direction E feature build that
the migration plan deferred; 6–8 are polish, distribution, and docs.

---

## 9. Tray-free integration testing (Tiers 1 & 2)

**Goal:** drive every account-management scenario (add / rename / delete / validation) and the
command surface **without launching the menu-bar tray shell**, in tests fast enough for CI.
This section is self-contained — an agent should be able to implement it from here without
re-deriving the design.

**Why not full E2E:** driving the *real* WKWebView end-to-end is effectively unavailable on
macOS. The official `@crabnebula/tauri-driver` supports only Linux (`WebKitWebDriver`) and
Windows (Edge WebDriver); Apple ships no WebDriver for an embedded WKWebView (only
`safaridriver`, which automates Safari, not a Tauri webview). Community drivers exist
(`tauri-plugin-webdriver`, CrabNebula's macOS driver, `tauri-webdriver`) but are young and not
worth adopting. **Tiers 1 + 2 below cover the same scenarios without a real window.** Do not
add a Tier 3 / WebDriver suite as part of this work.

The two tiers are complementary and stack on the existing **Tier 0** (the `authr_core::accounts`
unit tests + the `authr_core/tests/storage.rs` tempfile integration tests, already green):
- **Tier 1 (Svelte component tests)** proves UI *behavior* against a mocked backend.
- **Tier 2 (Rust command tests)** proves the command glue + storage round-trip against a real
  temp-dir store, with no mocking.

### 9.1 Tier 1 — Svelte component tests

**Stack:** Vitest + jsdom + `@testing-library/svelte` (Svelte 5 / runes compatible) +
`@testing-library/jest-dom`. Mounts the page components directly in a headless DOM; the tray,
positioner, and window chrome never load.

**The four seams the Phase 3 components import — mock all of them** (`vi.mock` at the top of
each test file, or a shared setup file):

| Module | Symbol used | Mock with |
|---|---|---|
| `@tauri-apps/api/core` | `invoke(cmd, args)` | a `vi.fn()` backed by an in-memory `Account[]`; assert call args **and** drive return/throw |
| `@tauri-apps/api/window` | `getCurrentWindow().hide()` | stub returning `{ hide: vi.fn() }` (used by Escape handling) |
| `@tauri-apps/plugin-clipboard-manager` | `writeText` | `vi.fn()` (only E1 needs it; harmless to stub globally) |
| `$app/navigation` | `goto(path)` | `vi.fn()`; assert navigation targets |

The mock `invoke` should switch on the command name and mutate a module-level
`let accounts: Account[]`, mirroring the real Rust behavior closely enough to exercise the UI:
`add_account` pushes + rejects duplicates/invalid (throw a string), `rename_account` mutates the
name + rejects collisions, `delete_account` removes, `list_accounts` returns the current list.
Note the **camelCase arg keys** the components send: `add_account` → `{ name, secret }`,
`rename_account` → `{ name, newName }`, `delete_account` → `{ name }` (Tauri maps `newName` ⇄
the Rust `new_name`; the test asserts the camelCase form the webview actually sends).

**Files under test:** `authr_app/src/routes/settings/+page.svelte` and
`authr_app/src/routes/settings/add/+page.svelte`. Put tests alongside or under
`authr_app/src/routes/settings/` (e.g. `settings.test.ts`, `add.test.ts`) — whatever Vitest's
`include` glob picks up.

**Scenario checklist (each maps to a Direction E acceptance item):**
1. **Add — happy path:** in the add form, type a name + a secret *with spaces*; click
   "+ Add account". Assert `invoke` called with `('add_account', { name: <trimmed>, secret:
   <raw, spaces still present> })` (whitespace stripping is core's job — assert the UI forwards
   the raw secret) and that `goto('/settings')` fired.
2. **Add — duplicate name:** mock `invoke` rejects with `"Account 'x' already exists"`; assert
   the inline `.error` text renders and `goto` was **not** called.
3. **Add — invalid secret:** mock rejects with an `Invalid secret: …` string; assert inline
   error renders.
4. **Add — submit gating:** empty name or empty secret leaves the primary button `disabled`.
5. **Rename — happy path:** on a manage row, click ✎; assert an input appears (focused) with the
   current name; change it, press Enter; assert `invoke('rename_account', { name: old, newName:
   next })` and that the list re-renders via a follow-up `list_accounts`.
6. **Rename — cancel:** Esc (or blur with unchanged/empty value) closes the editor without
   calling `rename_account`.
7. **Rename — collision:** mock rejects; assert the `.rename-error` text renders and the row
   stays in edit mode.
8. **Delete — modal + D4:** click 🗑; assert the confirm modal shows the "no recovery" copy and
   that **the account's base32 secret is absent from the DOM** (the D4 guarantee — there's no
   secret in scope to leak, so assert no secret-shaped text/no secret field). Cancel closes it
   with no `invoke`.
9. **Delete — confirm:** click Delete; assert `invoke('delete_account', { name })` then a
   re-fetch (`list_accounts`) and the row disappears.
10. **Re-fetch after mutation:** after add/rename/delete, assert `list_accounts` is re-invoked so
    E1/E3 reflect the change.

**Wiring:** add devDeps `vitest`, `@testing-library/svelte`, `@testing-library/jest-dom`,
`jsdom`; add a `"test": "vitest run"` (and `"test:watch": "vitest"`) script to
`authr_app/package.json`; add a minimal `vitest` config (jsdom environment, the
`@sveltejs/vite-plugin-svelte` plugin, and a setup file importing `@testing-library/jest-dom`).
Keep it a separate config from the Tauri `vite.config.ts` (or use `defineConfig` with a `test`
block) — the Tauri dev-server settings (fixed port 1420, etc.) are irrelevant to tests.

**Caveat:** Tier 1 mocks the backend, so it proves UI behavior only — it does **not** prove the
Rust commands or that anything persists. That's Tier 2's job.

### 9.2 Tier 2 — Rust command tests (no tray, no mock runtime)

The Phase 3 commands in `authr_app/src-tauri/src/lib.rs` are already thin wrappers over
`authr_core::accounts`; their **only** shell dependency is `storage_for(app)` →
`app.path().app_config_dir()`. Make the bodies testable by extracting each into a plain helper
that takes a `&Storage` (or `&dyn AccountStore`), leaving the `#[tauri::command]` as a
one-liner:

```rust
// testable core of each command — no AppHandle, no Tauri
fn add_account_impl(store: &Storage, name: String, secret: String) -> Result<AccountView, String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    let added = accounts::add_account(&mut all, name, secret).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())?;
    Ok(AccountView::from(&added))
}

#[tauri::command]
fn add_account(app: tauri::AppHandle, name: String, secret: String) -> Result<AccountView, String> {
    add_account_impl(&storage_for(&app)?, name, secret)
}
```

Do the same for `rename_account_impl(store, name, new_name) -> Result<(), String>` and
`delete_account_impl(store, name) -> Result<(), String>`. (Optionally `list_accounts_impl` /
`get_codes_impl` too, for symmetry.)

**Tests** mirror `authr_core/tests/storage.rs`: build a `Storage::new(tempdir.path())` and call
the `_impl` helpers directly. Since `_impl` lives in the bin/lib crate, either add a
`#[cfg(test)] mod tests` inside `lib.rs` or expose the helpers `pub(crate)` and add
`authr_app/src-tauri/tests/commands.rs`. Add `tempfile` as a `[dev-dependencies]` entry in
`authr_app/src-tauri/Cargo.toml`.

**What Tier 2 proves (that Tier 1 can't):** the real load → mutate → save round-trip, the exact
error **strings** the webview will display (`AccountError` → `to_string()`), that
`AccountView` carries no secret, and that the plaintext store on disk reflects each mutation.

**Scenario checklist:**
1. `add_account_impl` on an empty temp store → returns an `AccountView { name, issuer:None }`,
   and reloading the store shows one account with the whitespace-stripped secret.
2. `add_account_impl` duplicate name → `Err` whose string is the `Duplicate` message; store
   unchanged.
3. `add_account_impl` invalid secret → `Err` with the `InvalidSecret` string; store unchanged.
4. `rename_account_impl` → reload shows the new name, same secret (codes unaffected).
5. `rename_account_impl` collision → `Err` `Duplicate`; `rename_account_impl` on a missing name
   → `Err` `NotFound`.
6. `delete_account_impl` → reload shows the account gone; missing name → `Err` `NotFound`.
7. (Optional) a returned `AccountView` serializes without a `secret` field (serde round-trip
   assertion reinforcing D4).

**Why not `tauri::test::mock_builder` / `MockRuntime`?** It would invoke through the real IPC
handler (more faithful to serde marshalling), but under the mock runtime `app_config_dir()`
still resolves to the real `~/Library/Application Support/com.wwwsteve.authr`, polluting the
host — so you'd first have to make the storage dir injectable (managed `State`) anyway. The
`_impl` extraction gets ~95% of the value for ~10% of the effort and keeps tests hermetic. If a
future phase wants true IPC-level tests, the prerequisite is injecting the storage dir via Tauri
`State` rather than computing it inside `storage_for`.

### 9.3 Exit criteria for this testing work

- `pnpm test` (Vitest) green with the Tier 1 scenario checklist covered; runs headless, no
  tray, no real window.
- `cargo test` still green, now including the Tier 2 `_impl` command tests over a tempfile store.
- No regression to `pnpm build`, `pnpm check`, or the existing Tier 0 suites.
- No WebDriver / Tier 3 dependency introduced.
