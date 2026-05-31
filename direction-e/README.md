# Authr GUI — Direction E (Implementation Spec)

This folder is the **authoritative, self-contained spec for the chosen design: Direction E.**
It is a derivative of the design handoff bundle in `wireframes/` (the "autthr" bundle —
note the double-`t` is a typo, ignore it). The bundle explored several alternate directions
and an obsolete interaction model; **all of that is intentionally discarded here.** Anything
not described in this folder is not part of what we are building.

An agent should be able to implement the Authr GUI from this folder alone, without reading
the original bundle.

## Scope of these docs

These docs specify **layout, structure, screens, flows, behavior, and the data model.**
They deliberately **do not** prescribe visual styling — colors, typography, exact pixel
dimensions, borders, shadows, and the prototype's hand-drawn "sketch" aesthetic are **not**
part of this spec. The prototype's look was just the medium it was drawn in. Final visual
design is open and left to the implementer / the host app's existing look and feel.

When this spec says "row," "bar," "button," "list," etc., it means the *arrangement and
behavior* of those elements, not how they should be painted.

## Index

- **`README.md`** (this file) — what Direction E is, what's in and out of scope.
- **`screens.md`** — the screens, their layout, and what each contains (layout-only).
- **`interactions-and-data.md`** — interaction behaviors, state transitions, flows, and the
  underlying data model (accounts, TOTP codes, encryption, backup).

---

## What Authr is

Authr is a TOTP (time-based one-time password) authenticator — the kind of app that shows
rolling 6-digit 2FA codes for accounts like GitHub, AWS, login.gov, etc. The GUI is a small,
compact panel (the prototype framed it as a system-tray dropdown, ~344×568 in the mock; treat
that as "small, single-column, vertically-stacked" rather than a fixed size).

## What Direction E is

Direction E is the final selected design. Its identity is best understood as **a refinement
of Direction D**, with two decisive changes. The lineage:

- **Direction B** introduced a thin horizontal **countdown bar** at the top (instead of a
  large ring) to maximize vertical room for the account list, plus a single *global* timer
  (one countdown for all codes, not per-row).
- **Direction D** kept B's thin top countdown, added a **search field paired with a gear
  (settings) icon**, and consolidated **account management + encryption + backup into a single
  Settings hub** — removing the "+" / add button from the main screen.
- **Direction E** takes D and makes two changes:
  1. **The search + gear bar moves to the top** (directly under the countdown bar) instead of
     sitting at the bottom.
  2. **The hidden-code model is dropped entirely.** Every code is **always visible** on screen.

### The two defining decisions of E

1. **Codes are always on screen.** There is no masking (`•••`), no eye icon, and no
   "reveal for 30 seconds" behavior. Every account row shows its live 6-digit code at all times.
2. **Tap a row to copy.** Tapping/clicking anywhere on an account row copies that account's
   current code to the clipboard and gives brief visual confirmation ("✓ copied!") on the row.
   Copying does **not** reveal anything that wasn't already shown — the code was already visible.

### E's screen set (6 screens)

| ID | Screen | Notes |
|----|--------|-------|
| E1 | Main list | Countdown bar → search+gear bar → list of accounts with always-visible codes. No add button here. |
| E2 | Copy feedback state | Same as E1; a tapped row shows "✓ copied!" feedback. |
| E3 | Settings hub | Security (encryption) + Backup + Accounts management (rename / delete / add) in one screen. |
| E4 | Set up / change password | Encryption password screen with an unrecoverable-password warning. |
| E5 | Add account | Name + secret-key form. Reached from Settings, not the main screen. |
| E6 | Back up | Export the single encrypted accounts file. |

E3–E6 are **identical to Direction D's** equivalents (D2/D4/D5/D6). They are fully respecified
here so this folder stands alone.

---

## Explicitly OUT of scope (discarded from the bundle)

These existed in the bundle but are **not** part of Direction E. Do not implement them:

- **The hidden/reveal interaction model** — masked `•••` codes, the per-row eye icon, and
  the "reveal for 30s then re-hide" behavior. (This was the central model for the "model"
  shots and Directions A/B/C; E drops it.)
- **The tray-dropdown framing/context shots** (fake menu bar, beak, etc.) — explanatory mock
  scaffolding, not a screen to build.
- **Direction A** — the compact countdown *ring*, the sliders icon, and a separate dedicated
  full-screen "Manage accounts" screen.
- **Direction B** — the bottom tab bar (Codes / Manage tabs with a center "+").
- **Direction C** — gesture-driven minimal UI: long-press / swipe-up row action sheets, and
  the floating "+" action button (FAB).
- **Direction D's bottom placement** of the search+gear bar (E moves it to the top).
- **Drag-to-reorder** of accounts (this appeared only in Direction B's manage tab). Account
  management in E is rename + delete + add; reordering is not specified.

## In scope, inherited and kept

- A **single global countdown** for when all codes roll (one shared timer, not per-row).
- **Search/filter** of the account list by name.
- A **Settings hub** containing account management, encryption setup, and backup.
- **Encryption**: a user password that protects the on-device accounts file *and* every
  exported backup; it is unrecoverable by design.
- **Backup**: export of a single encrypted accounts file the user can store anywhere.
- **Account management**: add, rename (inline), and delete (with a confirmation that surfaces
  the secret to copy before destroying it).
