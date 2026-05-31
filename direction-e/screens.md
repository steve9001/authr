# Direction E — Screens (layout)

Layout-only. Element *arrangement, grouping, ordering, and content* are specified;
visual styling (color, type, exact px, borders) is not. ASCII sketches show vertical
stacking and grouping, not proportions.

All screens are a single vertical column inside one small compact panel. Items stack
top-to-bottom with consistent gaps. Where a screen is taller than the others, it scrolls.

Common building blocks are defined once in **`interactions-and-data.md` → Components**:
*Countdown bar*, *Search+Gear bar*, *Account row* (code mode), *Account row* (manage mode),
*Section label*, *Settings row*, *Primary/Ghost/Danger button*, *Form field*, *Bottom sheet*,
*Delete-confirm card*.

---

## E1 — Main list (codes always visible)

The default screen. Top to bottom:

1. **Countdown bar** — a thin horizontal progress bar spanning the width, with a small
   numeric seconds-remaining label at its right end. This is the single global timer for
   when all codes roll. (No ring, no per-row timers.)
2. **Search + Gear bar** — a horizontal row: a flexible-width search field on the left
   (placeholder like "Search…") and a settings **gear** button on the right. This sits
   directly under the countdown bar.
3. **Account list** — one **account row (code mode)** per account, stacked vertically.
   Each row shows the account **name** on the left and its **live 6-digit code** on the
   right (always visible; conventionally grouped 3+3, e.g. `342 315`). The whole row is the
   tap-to-copy target. The list scrolls if it overflows.

There is **no "+" / add button on this screen.** Adding accounts happens in Settings (reached
via the gear).

```
┌─────────────────────────────────────┐
│ ▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░░░░░   22s    │  ← countdown bar (global)
│ ┌───────────────────────┐  ┌──────┐  │
│ │ 🔍  Search…           │  │  ⚙   │  │  ← search (flex) + gear
│ └───────────────────────┘  └──────┘  │
│ ┌─────────────────────────────────┐  │
│ │ github                  342 315  │  │  ← account row (tap = copy)
│ ├─────────────────────────────────┤  │
│ │ heroku                  963 623  │  │
│ ├─────────────────────────────────┤  │
│ │ login.gov               792 115  │  │
│ ├─────────────────────────────────┤  │
│ │ microsoft               788 179  │  │
│ │ … (scrolls)                      │  │
│ └─────────────────────────────────┘  │
└─────────────────────────────────────┘
```

---

## E2 — Copy feedback state

Not a separate screen — the transient state of E1 immediately after a row is tapped.
Same layout as E1. The tapped row replaces its code with a brief **"✓ copied!"** confirmation
(and may give a brief highlight on the row). After a short moment it reverts to showing the
code again. See `interactions-and-data.md` → Copy behavior.

```
│ ┌─────────────────────────────────┐  │
│ │ github                  342 315  │  │
│ ├─────────────────────────────────┤  │
│ │ heroku               ✓ copied!   │  │  ← tapped row, brief confirmation
│ ├─────────────────────────────────┤  │
│ │ login.gov               792 115  │  │
│ └─────────────────────────────────┘  │
```

There is **no eye icon and no masked state** anywhere — codes are always shown; the only
row state besides "normal" is this brief copied-confirmation.

---

## E3 — Settings hub

Reached from the gear button on E1. Identical to Direction D's settings hub. Top to bottom:

1. **Header** — a back affordance (returns to E1) and the title "Settings".
2. **Security** section:
   - Section label "Security".
   - A **settings row** for **Encryption**:
     - *Not yet set up:* title "Encryption", subtext "Password protects this device & every
       backup", trailing action button **"Set up"** → opens E4.
     - *Already on:* title "Encryption · on", subtext "Accounts file is locked with your
       password", trailing action button **"Change"** → opens E4 (change mode).
3. **Backup** section:
   - Section label "Backup".
   - A **settings row** for **Back up accounts**:
     - subtext "Save the accounts file wherever you like" (or, when encryption is on,
       "Exported file is encrypted") with a trailing chevron → opens E6.
4. **Accounts · N** section (N = number of accounts):
   - Section label "Accounts · {count}".
   - One **account row (manage mode)** per account — name on the left, with **rename** (pencil)
     and **delete** (trash) affordances on the right. (Codes are not shown in manage rows.)
   - An **"Add account"** button at the end of the list → opens E5.

```
┌─────────────────────────────────────┐
│ ←   Settings                         │
│                                      │
│ SECURITY                             │
│ ┌─────────────────────────────────┐ │
│ │ 🔒 Encryption          [Set up] │ │  ← "· on" + [Change] when enabled
│ │    Password protects this device│ │
│ └─────────────────────────────────┘ │
│ BACKUP                               │
│ ┌─────────────────────────────────┐ │
│ │ ⬇ Back up accounts          ›  │ │  → E6
│ │    Save the accounts file …     │ │
│ └─────────────────────────────────┘ │
│ ACCOUNTS · 6                         │
│ ┌─────────────────────────────────┐ │
│ │ github               ✎    🗑    │ │  ← manage row (rename / delete)
│ ├─────────────────────────────────┤ │
│ │ heroku               ✎    🗑    │ │
│ ├─────────────────────────────────┤ │
│ │ login.gov            ✎    🗑    │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │        +  Add account           │ │  → E5
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

This screen is taller than the panel and scrolls.

---

## E4 — Set up / change password (encryption)

Reached from the Encryption row's "Set up" / "Change" action. Top to bottom:

1. **Header** — back affordance and title "Set up password" (or "Change password").
2. **Explanation block** — informational text: "Encrypts the file that stores your accounts.
   You'll enter it to open Authr, and it locks every backup you export."
3. **New password** field.
4. **Confirm password** field.
5. **Warning block** — emphatic, unrecoverable warning: "Authr can't reset this. Lose it and
   the accounts are gone."
6. **Primary action** — **"Encrypt accounts"** button.

```
┌─────────────────────────────────────┐
│ ←   Set up password                  │
│ ┌─────────────────────────────────┐ │
│ │ 🔒 Encrypts the file that stores │ │  ← explanation
│ │    your accounts. You'll enter   │ │
│ │    it to open Authr…             │ │
│ └─────────────────────────────────┘ │
│ New password                         │
│ ┌─────────────────────────────────┐ │
│ │ ••••••••••                       │ │
│ └─────────────────────────────────┘ │
│ Confirm password                     │
│ ┌─────────────────────────────────┐ │
│ │ ••••••••••                       │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │ ⚠ Authr can't reset this. Lose   │ │  ← danger/warning
│ │   it and the accounts are gone.  │ │
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │ 🔒  Encrypt accounts            │ │  ← primary
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

---

## E5 — Add account

Reached from the "Add account" button in Settings (E3). Top to bottom:

1. **Header** — back affordance and title "Add account".
2. **Account name** field — placeholder e.g. "GitHub".
3. **Secret key** field — a taller, monospace-oriented input for pasting the base32 secret.
   Hint text below: "Plain text for now · spaces ignored".
4. **Primary action** — **"+ Add account"** button (anchored toward the bottom).
5. A secondary **"Cancel"** button.

```
┌─────────────────────────────────────┐
│ ←   Add account                      │
│ Account name                         │
│ ┌─────────────────────────────────┐ │
│ │ GitHub                           │ │
│ └─────────────────────────────────┘ │
│ Secret key                           │
│ ┌─────────────────────────────────┐ │
│ │ JBSWY3DPEHPK 3PXP DEFG H2QR      │ │  ← paste base32; spaces ignored
│ │                                  │ │
│ └─────────────────────────────────┘ │
│ Plain text for now · spaces ignored  │  ← hint
│                                      │
│ ┌─────────────────────────────────┐ │
│ │ +  Add account                  │ │  ← primary
│ └─────────────────────────────────┘ │
│ ┌─────────────────────────────────┐ │
│ │           Cancel                 │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

---

## E6 — Back up (export file)

> **Override (see `UNIFIED_PLAN.md` D6):** backup uses its **own** password (password +
> confirm) entered at export time, independent of the live-store password — not "encrypted
> with your password." Exporting without a password is allowed but requires an explicit
> plaintext confirmation. Build E6 per D6, not the "🔒 encrypted with your password" wording
> below.

Reached from the "Back up accounts" row in Settings (E3). Presented as a **bottom sheet over a
dimmed Settings screen** in the prototype; on platforms where a sheet doesn't fit, a plain
dialog/screen with the same content is acceptable. Content, top to bottom:

1. Title "Back up accounts".
2. **File card** — a document affordance with the export filename (e.g. `authr-vault.authr`)
   and, when encryption is on, a note "🔒 encrypted with your password".
3. **Explanation** — "One file with all your accounts. Save it to iCloud, a drive, a USB
   stick — wherever you trust."
4. **Primary action** — **"Save / share file"** button (invokes the platform save/share).
5. Secondary **"Cancel"** button.

```
┌─────────────────────────────────────┐
│ (settings, dimmed behind)            │
│ ┌─────────────────────────────────┐ │
│ │ ▁▁▁▁  (sheet grabber)            │ │
│ │ Back up accounts                 │ │
│ │ ┌─────────────────────────────┐ │ │
│ │ │ 📄 authr-vault.authr        │ │ │  ← the file it hands you
│ │ │    🔒 encrypted with your   │ │ │
│ │ │       password              │ │ │
│ │ └─────────────────────────────┘ │ │
│ │ One file with all your accounts.│ │
│ │ Save it to iCloud, a drive…     │ │
│ │ ┌─────────────────────────────┐ │ │
│ │ │ ⬇  Save / share file        │ │ │  ← primary
│ │ └─────────────────────────────┘ │ │
│ │ ┌─────────────────────────────┐ │ │
│ │ │         Cancel              │ │ │
│ │ └─────────────────────────────┘ │ │
│ └─────────────────────────────────┘ │
└─────────────────────────────────────┘
```

---

## Delete-account confirmation (modal over E3)

> **Override (see `UNIFIED_PLAN.md` D4):** the secret is **not** displayed in this modal. Drop
> the "secret (copy before deleting)" block below; keep only the no-recovery warning. No
> command ever returns a secret to the UI.

Not a numbered E screen, but a required modal. Triggered by the trash affordance on a manage
row in Settings (E3). The list behind it is dimmed; a centered confirmation card asks to
confirm deletion. Card content, top to bottom:

1. A warning indicator.
2. Title: **Delete "{account name}"?**
3. Body: "Removes it from Authr. There's **no recovery** — you'd need the original secret to
   add it again."
4. A **secret display block** so the user can copy the secret *before* deleting it — labeled
   "secret (copy before deleting)" with the account's base32 secret shown.
5. Two buttons: **Cancel** and **Delete**.

```
            ┌───────────────────────────┐
            │            ⚠               │
            │     Delete "github"?       │
            │  Removes it from Authr.    │
            │  There's no recovery —     │
            │  you'd need the original   │
            │  secret to add it again.   │
            │  ┌─────────────────────┐   │
            │  │ secret (copy first) │   │
            │  │ JBSWY3DPEHPK3PXP    │   │
            │  └─────────────────────┘   │
            │  [ Cancel ]   [ 🗑 Delete ]│
            └───────────────────────────┘
```
