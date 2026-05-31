---
name: tauri-runtime-version-pin
description: The authr_app Tauri build breaks unless tauri-runtime and tauri-runtime-wry stay on the same version; cargo update can re-introduce the skew.
metadata:
  type: project
---

In `authr_app/src-tauri`, the freshly scaffolded Cargo.lock (tauri 2.10.3) let cargo
greedily resolve `tauri-runtime` to **2.11.2** while `tauri-runtime-wry` stayed at
**2.10.1**. Runtime 2.11 added a trait method (`eval_script_with_callback`) and changed
the `with_new_window_req_handler` signature that the 2.10.1 wry impl does not satisfy →
E0046/E0277 compile errors *inside the Tauri crates themselves* (not our code).

Fix that's in Cargo.lock: `tauri-runtime` pinned to **2.10.1** (and `wry` to **0.54.0**)
so it matches `tauri-runtime-wry 2.10.1`.

**Why:** the error message points at `tauri-runtime-wry`/`wry` and looks like a wry bug —
downgrading wry alone does NOT fix it. The real axis is tauri-runtime ↔ tauri-runtime-wry
alignment.

**How to apply:** if `cargo update` makes the Tauri build fail with missing-trait-item or
non-Send-closure errors in `tauri-runtime-wry`, run
`cargo tree -p authr_app | grep -E 'tauri-runtime'` and force the two `tauri-runtime*`
crates onto the same minor: `cargo update -p tauri-runtime --precise 2.10.1`.
