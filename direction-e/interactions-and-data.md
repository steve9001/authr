# Direction E — Interactions, Flows & Data Model

Companion to `screens.md`. Specifies behavior and the data behind the screens.
Styling is out of scope (see `README.md`).

---

## Components (reusable building blocks)

These are referenced by the screens. Only layout/behavior is specified.

- **Countdown bar** — a thin, full-width horizontal progress element with a small numeric
  "seconds remaining" label at the right. Represents the single **global** TOTP period
  countdown. It fills/empties as the period elapses and resets when codes roll. There is
  exactly one of these; there are **no per-row timers**.

- **Search + Gear bar** — a horizontal row containing a flexible-width **search field** and,
  to its right, a **gear (settings) button**. On E1 it sits directly under the countdown bar.

- **Account row — code mode** (E1/E2): a full-width row with the account **name** on the left
  and its **current 6-digit code** on the right (always visible, conventionally grouped 3+3).
  The entire row is a single tap/click target that copies the code. Has a transient
  "✓ copied!" feedback state.

- **Account row — manage mode** (E3): a full-width row with the account **name** and trailing
  **rename** (pencil) and **delete** (trash) affordances. No code shown.

- **Section label** — a small caption that titles a group in Settings ("Security", "Backup",
  "Accounts · N").

- **Settings row** — an icon + title + optional subtext on the left, and a trailing action
  (a button, or a chevron) on the right.

- **Buttons** — three intents: **primary** (the main commit action), **ghost/secondary**
  (e.g. Cancel, Add account in settings), and **danger** (Delete). Layout/behavior only.

- **Form field** — a labeled input; optionally a hint line beneath. The secret-key field is
  a taller, monospace-friendly multi-line input.

- **Bottom sheet** — a panel that rises from the bottom over a dimmed background, with a
  grabber affordance. Used for E6 (backup). A plain dialog is an acceptable substitute where
  sheets don't fit the platform.

- **Delete-confirm card** — a centered modal card over a dimmed list; see `screens.md`.

---

## Core interaction model (the heart of Direction E)

1. **Codes are always visible.** Every account's current code is shown on E1 at all times.
   There is **no masking, no eye icon, and no reveal/auto-hide timer.** (This is the deliberate
   reversal of the bundle's earlier "hidden by default" model.)

2. **Tap a row → copy.** Tapping/clicking anywhere on an account row copies that account's
   **current** code to the system clipboard.

3. **Copy feedback.** On copy, the tapped row briefly shows **"✓ copied!"** (optionally with a
   brief highlight) in place of the digits, then reverts to showing the code after a short
   moment (~1–1.5s is reasonable; exact duration is implementer's choice). Only the tapped row
   changes; other rows are unaffected. Copying reveals nothing new — the code was already shown.

4. **One global countdown.** A single countdown bar at the top indicates when all codes roll.
   When the TOTP period elapses, all codes regenerate simultaneously and the bar resets.

5. **Search filters the list.** Typing in the search field filters the account rows by name
   (substring match on the account name). An empty query shows all accounts.
   *(Note: existing app behavior keeps the filter input focused so typing filters immediately;
   preserve that if it fits the platform.)*

---

## Navigation & flows

```
            ┌──────────────────────────────────────────┐
            │                  E1 Main                  │
            │  (countdown · search+gear · code rows)     │
            └───────────────┬───────────────┬───────────┘
              tap a row      │               │  tap gear ⚙
              = copy code     │               │
              (E2 feedback)   ▼               ▼
                          (stays on E1)   ┌─────────────┐
                                          │ E3 Settings │
                                          └──┬───┬───┬──┘
                          Encryption "Set up"│   │   │ Backup row ›
                          / "Change"          │   │   └────────► E6 Back up
                                              │   │ "Add account"
                                              ▼   └────────────► E5 Add account
                                          E4 Password           (Add / Cancel → E3)
                                          (Encrypt → E3)
                          manage row 🗑 ──► Delete-confirm modal (over E3)
                          manage row ✎ ──► inline rename (on E3)
```

Key points:
- The **only entry to management** (add / rename / delete / encryption / backup) is the
  **gear → Settings (E3)**. There is no add button or management affordance on E1.
- **Back** affordances on E3/E4/E5/E6 return toward E1 (E4/E5/E6 return to E3; E3 returns to E1).
- **Add account** is treated as a comparatively rare workflow, hence its placement in Settings.

### Add account (E5)
- User enters an account **name** and pastes a **secret key** (base32). Spaces in the secret
  are ignored. "+ Add account" commits and returns to Settings; the new account appears in the
  Accounts list and on E1. Cancel discards.

### Rename (E3)
- The pencil affordance on a manage row renames inline (edit the account's display name).

### Delete (E3 → modal)
- The trash affordance opens the **delete-confirm modal**. It warns there is **no recovery**
  and surfaces the account's **secret so the user can copy it before deleting**. Delete removes
  the account permanently; Cancel dismisses.

### Encryption (E3 → E4)
- If no password is set, the Encryption row offers **"Set up"**; once set, it shows
  **"Encryption · on"** with **"Change"**. E4 collects a new password + confirmation, shows the
  unrecoverable warning, and on commit encrypts the on-device accounts store. The password is
  **required to open Authr** thereafter and **locks every exported backup**.

### Backup (E3 → E6)
- Exports a **single file** containing all accounts (the prototype names it
  `authr-vault.authr`). When encryption is on, the exported file is **encrypted with the user's
  password**. The user saves/shares it to wherever they choose. There is no cloud dependency.

---

## Data model

### Account
Each account has:
- **name** — user-facing display label (e.g. "github", "login.gov"). Editable (rename).
- **secret** — the base32-encoded TOTP shared secret. Entered once on add; surfaced again only
  in the delete-confirm modal (so it can be copied before destruction). Spaces are ignored on
  input.
- **code** — the *derived* current TOTP value; not stored, computed from secret + current time.

The prototype's sample set (for reference / fixtures only): `github`, `heroku`, `login.gov`,
`microsoft`, `aws-docnow`, `namecheap`.

### TOTP / codes
- Codes are **6 digits**, displayed grouped as 3+3 (e.g. `342 315`).
- Standard TOTP semantics: a fixed period (the global countdown — standard is 30s; the mock's
  "22s" is just a remaining-time snapshot), all codes roll together at period boundaries.
- The code is derived from the account secret; only the secret is persisted, never the code.
- Assume standard TOTP defaults (SHA-1, 30-second period, 6 digits) unless the imported secret
  specifies otherwise. The add form only collects name + secret ("Plain text for now"), so the
  initial implementation can assume these defaults.

### Encryption
- A single user **password** encrypts the on-device accounts store and every exported backup.
- It is **unrecoverable** — the app cannot reset it; losing it means losing the accounts. The
  UI must state this plainly (E4 warning, delete-confirm "no recovery").
- When set, the password gates opening the app.

### Backup / export
- A **single file** (e.g. `authr-vault.authr`) holds all accounts.
- Encrypted with the user's password when encryption is on; the backup UI indicates this.
- Storage is entirely user-chosen (local drive, USB, cloud folder) — the app just produces the
  file and hands it off via the platform save/share mechanism.

---

## Behavioral checklist (acceptance)

- [ ] E1 shows the global countdown bar, a top search+gear bar, and one always-visible code
      per account. No add button on E1.
- [ ] Tapping anywhere on a row copies that row's current code and shows brief "✓ copied!"
      feedback on that row only, then reverts.
- [ ] No masking, eye icon, or reveal/hide behavior exists anywhere.
- [ ] Search filters the account list by name; filtering is immediate.
- [ ] All codes roll together when the period elapses; the bar resets.
- [ ] The gear opens Settings (E3), the sole hub for add / rename / delete / encryption / backup.
- [ ] Add (E5) takes name + base32 secret (spaces ignored) and returns to Settings.
- [ ] Rename is inline from a manage row.
- [ ] Delete opens a confirm modal that shows the secret to copy first and warns of no recovery.
- [ ] Encryption (E4) sets an unrecoverable password that protects the store and backups.
- [ ] Backup (E6) exports one (encrypted-when-enabled) accounts file via platform save/share.
