# Rebrand plan: `authr_app` → `authr`

Goal: rename the application from `authr_app` to `authr` across source, config, build
manifests, and docs. This covers the npm package, the Rust bin/lib crates, the Tauri
product name, the workspace path, and all references in documentation/comments.

## Scope summary

Every literal occurrence of `authr_app` (and the derived `authr_app_lib`) lives in 6
hand-edited files plus 3 doc/comment files, 1 lockfile (auto-regenerated), and 1 directory
name. The Tauri bundle identifier and window title are **already** branded `authr` / `Authr`
and do **not** change — importantly this means **user data is not orphaned** (see Risks).

---

## Decisions to make before editing

These three choices change the exact strings used. Recommended defaults are marked.

1. **Directory `authr_app/` → ?**
   The repo root is already `authr/` and the sibling crate is `authr_core/`, so the `_app`
   suffix currently disambiguates the Tauri app from the workspace root and the core lib.
   - **(a) Keep `authr_app/` as the directory name** *(recommended — least churn, avoids
     an awkward `authr/authr/` nesting; the directory name is internal and not user-facing)*.
   - (b) Rename to `authr/` → yields `authr/authr/`.
   - (c) Rename to `app/`.
   The plan below is written for **(a) keep the directory**. If you pick (b)/(c), also update
   the workspace path in the root `Cargo.toml` and every `authr_app/...` path in `README.md`.

2. **Tauri `productName` → `authr` or `Authr`?**
   This is the user-facing macOS app name (`.app` bundle, menu bar). The window `title` is
   already `"Authr"`. **Recommended: `"Authr"`** for a polished display name. The rest of this
   plan uses `Authr`; substitute `authr` if you prefer all-lowercase.

3. **Lib crate `authr_app_lib` → ?**
   The `_lib` suffix is required to avoid a bin/lib name clash (see the comment in
   `Cargo.toml`). **Recommended: `authr_lib`.**

---

## File-by-file changes

### Must edit (hand-maintained)

| # | File | Line | From → To |
|---|------|------|-----------|
| 1 | `authr_app/package.json` | 2 | `"name": "authr_app"` → `"name": "authr"` |
| 2 | `authr_app/src-tauri/Cargo.toml` | 2 | `name = "authr_app"` → `name = "authr"` |
| 3 | `authr_app/src-tauri/Cargo.toml` | 14 | `name = "authr_app_lib"` → `name = "authr_lib"` |
| 4 | `authr_app/src-tauri/src/main.rs` | 5 | `authr_app_lib::run()` → `authr_lib::run()` |
| 5 | `authr_app/src-tauri/tauri.conf.json` | 3 | `"productName": "authr_app"` → `"productName": "Authr"` |

The root `Cargo.toml` workspace member path (`"authr_app/src-tauri"`) **stays as-is** under
decision 1(a). Note that renaming the bin crate to `authr` changes the produced binary name
from `authr_app` to `authr`; Tauri then bundles it under `productName` — see Risks for the
binary-name/productName coordination gotcha.

### Docs & comments

| # | File | Line(s) | Change |
|---|------|---------|--------|
| 6 | `authr_app/src/test/backend-mock.ts` | 4 | Comment path `authr_app/src-tauri/src/lib.rs` — only update if the directory is renamed (1b/1c). Under 1(a), no change. |
| 7 | `README.md` | 58, 66, 67, 88, 111 | `authr_app/` paths in build steps & project layout — only change if directory renamed (1b/1c). Under 1(a), no change. The crate name isn't referenced in README, so the bin rename needs no doc edit here. |
| 8 | `memory/tauri-runtime-version-pin.md` | 3, 8, 23 | References the crate as `authr_app` and the command `cargo tree -p authr_app`. After the crate rename, update to `authr` (e.g. `cargo tree -p authr`). This is the agent's own memory note, not app source — update for accuracy. |

> Note: `README.md` lines 58/66/67/88/111 and `backend-mock.ts:4` are **path** references to
> the `authr_app/` directory, not the crate. They only need editing under decision 1(b)/1(c).
> If you keep the directory (1a), the only doc that needs touching is the memory note (#8),
> because it names the *crate* `authr_app`, which is being renamed regardless.

### Auto-regenerated (do not hand-edit)

| # | File | Note |
|---|------|------|
| 9 | `Cargo.lock` (line 163, `name = "authr_app"`) | Regenerates on the next `cargo build`/`cargo check`. Run `cargo check` after editing the manifests so the lockfile updates in the same commit. |

### Not changing (already branded `authr` — verify, don't edit)

- `tauri.conf.json` `identifier`: `com.wwwsteve.authr` ✓
- `tauri.conf.json` window `title`: `Authr` ✓
- `authr_core` crate name — out of scope (only `authr_app` is being renamed).
- Icons in `authr_app/src-tauri/icons/`, `capabilities/default.json`, `gen/schemas/` — no
  `authr_app` references found.

---

## Execution order

1. Make decisions 1–3 above.
2. Edit the 5 "must edit" files (#1–#5).
3. Edit the memory note (#8). If the directory is being renamed, also do `git mv authr_app authr`
   (or `app`), then update the root `Cargo.toml` path, `README.md` paths, and `backend-mock.ts:4`.
4. Run `cargo check` (from repo root) to regenerate `Cargo.lock` and confirm the workspace
   resolves with the new crate names.
5. Run `cd authr_app && pnpm install` is not needed (no dep names changed), but run
   `pnpm tauri build` (or `pnpm tauri dev`) to confirm the bundle builds under the new
   `productName`/binary name.
6. Commit manifests + lockfile + docs together.

## Verification checklist

- [ ] `grep -rIn "authr_app" --exclude-dir={node_modules,target,.git} .` returns nothing
      (or only intended directory paths if you kept the dir but expected zero crate hits).
- [ ] `cargo check` succeeds; `Cargo.lock` now shows `name = "authr"`.
- [ ] `cargo tree -p authr` resolves (was `-p authr_app`).
- [ ] `pnpm tauri build` produces `Authr.app` and a binary named `authr`.
- [ ] App launches as a menu-bar app and reads existing accounts (identifier unchanged → same
      data dir).

## Risks & gotchas

- **Binary name vs `productName` (Tauri 2):** renaming the Cargo package to `authr` makes the
  binary `authr`, while `productName` is `Authr`. Tauri sanitizes/derives the expected main
  binary name and can error with a "binary does not exist" message if they diverge. If the
  build complains, set `"mainBinaryName": "authr"` in `tauri.conf.json` or align the casing.
  Verify in step 5 before committing.
- **User data is safe:** the macOS app data/config directory is keyed off the bundle
  `identifier` (`com.wwwsteve.authr`), which is **not** changing. Renaming the package and
  `productName` will **not** orphan existing vaults/accounts.
- **Lockfile drift:** don't hand-edit `Cargo.lock`; let `cargo check` rewrite it so the commit
  is internally consistent.
- **`_lib` suffix is load-bearing:** keep a distinct lib name (`authr_lib`) — collapsing it to
  `authr` to match the bin reintroduces the Windows bin/lib clash the existing comment warns about.
