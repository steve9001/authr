# authr → System Tray App: Migration Plan

> **⚠️ Superseded by [`UNIFIED_PLAN.md`](./UNIFIED_PLAN.md).** That document is the
> authoritative build plan and **wins on every decision.** This file is retained as a
> mechanics appendix — its phase detail, file maps, and security rationale are still
> referenced by section (`migration §…`) from the unified plan. **Do not act on the
> decisions below where they conflict with the unified plan.** In particular, the unified
> plan *reverses* these: the UI is **not** read-only (it's the full manager); the **CLI is
> removed entirely** (not kept); **Android is dropped** (no Phase 4); and **encryption +
> backup are in scope** (not deferred). See `UNIFIED_PLAN.md §0` for the full list of changes.

This plan describes how to retire authr's TUI and reshape the app into a macOS
**menu-bar (system tray) application** built on **Tauri**, with a **Svelte** UI,
while keeping the existing CLI and laying groundwork for an **Android** build off
the same codebase.

It is the concrete, authr-specific companion to the general field guide in
[`BUILDING_TRAY_APPS.md`](./BUILDING_TRAY_APPS.md). Section references like
*(guide §3.3)* point back into that document.

---

## 1. Decisions (locked)

| Decision | Choice | Notes |
|---|---|---|
| Tray/UI framework | **Tauri 2** | Best-trodden path for a macOS menu-bar popover; guide maps 1:1. Also gives us Android off one codebase. |
| Web UI tech | **Svelte** | Compiles to near-vanilla JS, ~no runtime, tiny bundle → instant popover. Reactivity cleanly handles the countdown, live filter, and code rollover. |
| CLI subcommands | **Keep** (`list`/`add`/`remove`/`show`) | Stay the primary way to add/remove accounts in phase 1. The tray UI is **read-only** (view + copy codes) initially. |
| Account management in UI | **Deferred** to a later phase | Required before Android is genuinely useful (no CLI on a phone). |
| TUI (ratatui) | **Remove** | This is the explicit goal of the migration. |
| egui/eframe GUI | **Remove** | Superseded by the Tauri UI. |
| Distribution | **Ship to others** | Front-load Developer ID signing + hardened runtime + notarization + DMG (macOS) and signed APK/AAB (Android). |

### Platform shape (important)

A system tray is a **desktop-only** concept — Android has no menu bar. So we
share the *logic* and the *UI*, but the *shell* differs per platform:

- **Shared:** `authr_core` (model/storage/totp) + the entire Svelte UI.
- **macOS shell:** tray icon + anchored, auto-hiding popover window.
- **Android shell:** a normal full-screen activity rendering the same Svelte UI.

All tray-specific Rust (tray icon, positioner, focus-loss auto-hide, close
intercept) lives behind `#[cfg(desktop)]` and simply does not compile into the
Android build.

---

## 2. Target workspace layout

Current:

```
authr/
  authr_core/        # lib: model, storage, totp  ← keep, lightly refactor
  authr_cli/         # bin "authr": clap CLI + tui + egui gui  ← strip to CLI-only
```

Target:

```
authr/
  authr_core/            # lib: model, storage, totp  (shared by CLI + Tauri, desktop + Android)
  authr_cli/             # bin "authr": clap CLI ONLY (list/add/remove/show)
  authr_app/             # the Tauri app
    src-tauri/           # Rust crate (workspace member) — tray, commands, lifecycle
      tauri.conf.json
      capabilities/
      icons/
      Info.plist fragments / entitlements
    src/                 # Svelte UI (components, stores, styles)
    package.json
    index.html
```

The top-level `Cargo.toml` workspace `members` list gains
`authr_app/src-tauri` and drops nothing else (`authr_core`, `authr_cli` stay).

---

## 3. `authr_core` refactor (small, enabling)

`authr_core` is already clean and UI-agnostic — keep it as the single source of
truth for both frontends and both platforms. Two targeted changes:

1. **Inject the storage directory instead of hard-coding `ProjectDirs`.**
   `storage.rs` currently resolves the path via `directories::ProjectDirs`,
   which is correct on desktop but unreliable inside the Android app sandbox.
   Refactor `load_accounts`/`save_accounts` to take a `&Path` (or a small
   `Storage { dir }` handle). Then:
   - the CLI passes the `ProjectDirs` config dir (unchanged behavior),
   - the Tauri app passes Tauri's `app_config_dir()` (works on macOS **and**
     Android) *(guide §6.1)*.

2. **Add a "view" projection that never exposes secrets.** Introduce an
   `AccountView { name, issuer }` (no `secret`) for anything that crosses the
   Rust→webview boundary. The base32 secret must never reach the UI; only the
   generated 6-digit code does (see §5, Security).

Everything else in `model.rs`/`totp.rs` stays as-is.

---

## 4. Phased delivery

### Phase 0 — Scaffolding & teardown
- Add Rust + Node toolchain prerequisites; install the Tauri CLI.
- Scaffold `authr_app` (`create-tauri-app`, Svelte template) and wire
  `src-tauri` into the workspace.
- **Delete the TUI:** remove `authr_cli/src/tui_interface.rs`, the `tui`
  feature, and the `ratatui` + `crossterm` deps.
- **Delete the egui GUI:** remove `authr_cli/src/gui_interface.rs`, the `gui`
  feature, the `gui-worker` subcommand + the no-arg "spawn detached GUI" branch
  in `main.rs`, and the `eframe`/`egui`/`image` deps.
- Trim `authr_cli` to a pure CLI: no-arg invocation prints help (or we decide
  no-arg launches nothing, since the tray app is launched as a bundle, not via
  the `authr` binary).
- Result: `cargo build` is green with a CLI-only `authr` and an empty-shell
  Tauri app.

### Phase 1 — macOS tray app, read-only UI
The core of the migration. Build the menu-bar popover that lists accounts and
copies codes.

**Backend (`src-tauri`)** — wire the lifecycle as small single-purpose
functions *(guide §8)*:
- `set_activation_policy(Accessory)` at runtime + `LSUIElement=true` in the
  bundle plist *(guide §2.1)*.
- Pre-declare a single `"main"` popover window: `visible:false`,
  `decorations:false`, `resizable:false`, `skipTaskbar:true`, sized for a
  popover *(guide §3.1)*. Use a named constant for the label.
- Tray icon: monochrome **template** image, tooltip "authr", attached menu,
  `show_menu_on_left_click(false)` *(guide §4.1)*.
- Tray click handler: left button-up → toggle the popover; anchor it to the
  tray icon via `tauri-plugin-positioner` (`TrayCenter`) and focus it
  *(guide §3.2, §4.2)*.
- Auto-hide on focus loss *(guide §3.3)* + `Escape`-to-hide in the frontend.
- Context menu: at minimum an explicit **Quit** (`app.exit(0)`)
  *(guide §4.4)*; optionally "Open at login".

**Commands (the API surface, §5)** — minimal and read-only:
- `list_accounts() -> Vec<AccountView>` (name + issuer only).
- `get_codes() -> Vec<CodeView>` where `CodeView = { name, issuer, code,
  period_seconds, valid_until_unix }`. Codes generated in Rust via
  `authr_core::totp`.
- `copy_code(name) -> Result<(), String>` *or* let the frontend copy via the
  clipboard plugin (the code is already in the webview).

**Svelte UI** — port the existing egui popover's look/feel:
- Prominent countdown timer (green → red under 5s), driven by a 1s reactive
  tick computing `remaining` from `valid_until_unix`; re-fetch codes when the
  period rolls.
- Always-focused filter input; live, case-insensitive name filter.
- Account rows: name left, code right (monospace); **entire row clickable** to
  copy, with a brief flash for feedback (matches current behavior).
- Dark theme by default.
- Auto-focus the filter on window `focus` event *(guide §5.3)*.

**Capabilities & CSP** *(guide §5.4)*: grant the `main` window only what it uses
(window hide/show, positioner, clipboard-write). Restrictive CSP.

Exit criteria: clicking the tray icon toggles a dark popover anchored under the
icon; typing filters; clicking a row copies the current code with a flash;
clicking away hides it; Quit exits cleanly; no Dock icon.

### Phase 2 — macOS polish & hardening
- Verify no Dock icon / no app menu under both `tauri dev` and a built `.app`.
- Tune popover size/position, flash timing, empty/error states (degrade
  gracefully if `accounts.json` is missing or corrupt — return defaults, never
  panic) *(guide §6.2)*.
- Optional "Launch at login" toggle.
- App + tray icons finalized (template image for the tray; full-color for the
  bundle).

### Phase 3 — Signing, notarization, packaging (macOS)
Front-loaded because we're distributing *(guide §2.3)*:
- Developer ID Application certificate; enable **hardened runtime**.
- `Entitlements.plist` with the **minimum** set (`network.client` only if we
  ever sync; none of audio/camera). Today authr touches no gated hardware, so
  the entitlement set should be nearly empty.
- Notarization (`notarytool`) + stapling; produce a signed, notarized **DMG**.
- Document the release steps; ideally script them.

### Phase 4 — Android target
- `cargo tauri android init`; install Android SDK/NDK + JDK.
- Confirm tray/positioner/auto-hide code is all behind `#[cfg(desktop)]` so the
  Android build compiles.
- Storage: confirm `app_config_dir()` resolves inside the Android sandbox
  (the §3 refactor makes this clean).
- Full-screen layout pass on the Svelte UI (the same components, no popover
  chrome).
- Signed APK for sideloading onto your phone; AAB if Play Store later.

### Phase 5 — UI account management (unlocks real Android use)
- Add `add_account`, `remove_account` (and maybe `rename`) commands with
  validation on write *(guide §6.3)* — reuse the CLI's existing validation
  (generate a code to verify the secret before saving).
- Add UI screens: an "add account" form (name, issuer, secret; optional QR
  scan on Android later) and per-row delete.
- Decide whether the desktop CLI remains the canonical manager or becomes
  secondary. Both write the same `accounts.json`, so they stay interoperable.

---

## 5. The Rust ⇄ webview boundary & security

This is a TOTP app, so the boundary deserves care *(guide §5, §7)*:

- **Secrets never cross the bridge.** The base32 `secret` stays in Rust.
  Commands return `AccountView`/`CodeView` only — names, issuers, and the
  already-computed 6-digit code. There is no command that returns a secret to
  the webview. (The CLI's `show --seed` stays CLI-only.)
- **Narrow, explicitly-registered command surface** *(guide §5.1)*: just the
  read commands in phase 1, plus the management commands in phase 5. The webview
  reaches Rust no other way.
- **Restrictive CSP + per-window capabilities** *(guide §5.4)*: the popover
  loads only local assets and is granted only window-hide/show, positioner, and
  clipboard-write.
- **Clipboard hygiene:** copying a code is expected; consider (later) clearing
  the clipboard after N seconds.
- **Storage at rest:** `accounts.json` is currently plaintext (as in the CLI
  today). Encryption is on the existing roadmap (README) and is **out of scope**
  for this migration, but the injected-storage refactor (§3) is the right seam
  to add it later.

---

## 6. Files touched (quick map)

**Remove**
- `authr_cli/src/tui_interface.rs`
- `authr_cli/src/gui_interface.rs`
- `authr_cli/assets/` egui icon usage (move icons under `authr_app`)
- `tui`/`gui` features + `ratatui`, `crossterm`, `eframe`, `egui`, `image`
  deps; the `gui-worker` subcommand and no-arg spawn branch in
  `authr_cli/src/main.rs`.

**Refactor**
- `authr_core/src/storage.rs` — inject storage dir.
- `authr_core/src/model.rs` — add `AccountView`.

**Add**
- `authr_app/` (Tauri + Svelte): `src-tauri/` Rust (lifecycle, tray, commands),
  `src/` Svelte UI, config (`tauri.conf.json`, `capabilities/`, plist/
  entitlements, icons).
- Top-level `Cargo.toml` — add `authr_app/src-tauri` to workspace members.
- `README.md` — replace TUI docs with tray-app + CLI docs (do this at the end).

---

## 7. Risks & open questions

- **`directories` on Android.** Mitigated by the §3 storage-dir injection;
  verify early in Phase 4.
- **Notarization friction.** Standard but fiddly; starting in Phase 3 (not at
  the very end) avoids a painful retrofit *(guide §2.3)*.
- **Tray template icon.** Need a clean monochrome glyph; the current full-color
  `icon.png` is fine for the bundle but reads poorly in the menu bar
  *(guide §4.1)*.
- **No-arg `authr` behavior.** Once the GUI is a bundle, decide what bare
  `authr` does (print help is the safe default).
- **iOS** is not in scope but comes "for free-ish" with Tauri mobile if ever
  wanted.

---

## 8. Suggested order of work

1. Phase 0 teardown + scaffold (green build, CLI-only + empty Tauri shell).
2. `authr_core` storage/view refactor (§3).
3. Phase 1 backend lifecycle + read commands.
4. Phase 1 Svelte popover UI.
5. Phase 2 polish.
6. Phase 3 signing/notarization/DMG.
7. Phase 4 Android.
8. Phase 5 UI account management.
9. README rewrite.
