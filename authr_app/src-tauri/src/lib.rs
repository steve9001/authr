//! authr tray app — backend lifecycle + the narrow command surface.
//!
//! Phase 2 (UNIFIED_PLAN §5): a single hidden popover toggled from a menu-bar tray icon,
//! anchored under the icon, auto-hiding on focus loss, with an explicit Quit. Two read-only
//! commands (`list_accounts`, `get_codes`) read through `authr_core::storage::Storage` rooted
//! at the OS config dir. Secrets never cross the bridge (D4) — only `AccountView`/`CodeView`.

use authr_core::accounts;
use authr_core::model::{AccountView, CodeView};
use authr_core::storage::Storage;
use authr_core::totp;
use authr_core::vault::{self, AccountStore, SecretString, Session};
use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, Manager, State, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

/// Stable label for the single popover window (declared in `tauri.conf.json`).
const MAIN_WINDOW: &str = "main";
/// Stable id for the tray menu's Quit item.
const QUIT_MENU_ID: &str = "quit";

/// In-session unlock state (UNIFIED_PLAN D7): the passphrase held in memory for the life of
/// the process once the store is unlocked. `None` = locked (or encryption not enabled). Held
/// as a zeroizing [`SecretString`] so it isn't left lying in plain `String`s. The webview
/// never sees it — only `unlock`/`set_password`/`change_password` write it, and only the Rust
/// `Session` reads it to (de)crypt the store.
#[derive(Default)]
struct VaultSession(Mutex<Option<SecretString>>);

/// `encryption_status()` payload — drives the Settings display and the unlock gate.
#[derive(Serialize)]
struct EncryptionStatus {
    /// The store on disk is an encrypted vault.
    enabled: bool,
    /// Encrypted but no passphrase is held this session — the app must show `/unlock`.
    locked: bool,
}

/// The store directory rooted at this app's OS config dir.
fn storage_for(app: &tauri::AppHandle) -> Result<Storage, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(Storage::new(dir))
}

/// Build the right [`AccountStore`] backend for a mutation/read command: the encrypted
/// [`Session`] when the store is a vault (requires the session passphrase — D7), else the
/// plaintext [`Storage`]. The boxed trait object is exactly the seam the `_impl` helpers take,
/// so their bodies are identical for both backends (UNIFIED_PLAN §3.2 item 4).
fn account_store(
    storage: Storage,
    held: Option<SecretString>,
) -> Result<Box<dyn AccountStore>, String> {
    if vault::is_encrypted(storage.dir()) {
        match held {
            Some(passphrase) => Ok(Box::new(Session::resume(storage, passphrase))),
            None => Err("Locked — unlock Authr first".to_string()),
        }
    } else {
        Ok(Box::new(storage))
    }
}

/// Clone the held passphrase (if any) out of the managed session lock.
fn held_passphrase(vault: &State<VaultSession>) -> Result<Option<SecretString>, String> {
    Ok(vault.0.lock().map_err(|e| e.to_string())?.clone())
}

// Each command's testable core: a plain helper over the `AccountStore` seam (the plaintext
// `Storage` or the encrypted `Session`), with no `AppHandle` and no Tauri runtime, so Tier 2
// can drive the real load → mutate → save round-trip against a tempfile store
// (UNIFIED_PLAN §9.2). Each `#[tauri::command]` resolves the OS config dir + the session
// passphrase and delegates here.

/// E1's account list: name (+issuer) only, no codes, no secrets.
fn list_accounts_impl(store: &dyn AccountStore) -> Result<Vec<AccountView>, String> {
    let accounts = store.load().map_err(|e| e.to_string())?;
    Ok(accounts.iter().map(AccountView::from).collect())
}

/// E1's live codes: each account projected to a `CodeView` (code + period boundary).
fn get_codes_impl(store: &dyn AccountStore) -> Result<Vec<CodeView>, String> {
    let accounts = store.load().map_err(|e| e.to_string())?;
    accounts
        .iter()
        .map(|a| totp::generate_code_view(a).map_err(|e| e.to_string()))
        .collect()
}

/// E5 add core: validate the secret + reject a duplicate name in `authr_core`, persist, and
/// return the created account projected to an `AccountView` (no secret crosses the bridge, D4).
/// The secret's whitespace is stripped in core ("spaces ignored"). When the backend is an
/// encrypted `Session`, `save` re-encrypts silently with the in-memory passphrase (D7).
fn add_account_impl(
    store: &dyn AccountStore,
    name: String,
    secret: String,
) -> Result<AccountView, String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    let added = accounts::add_account(&mut all, name, secret).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())?;
    Ok(AccountView::from(&added))
}

/// Inline rename core. Rejects a name collision / missing account in core.
fn rename_account_impl(
    store: &dyn AccountStore,
    name: String,
    new_name: String,
) -> Result<(), String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    accounts::rename_account(&mut all, &name, new_name).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())
}

/// Permanent delete core. No secret is returned (D4).
fn delete_account_impl(store: &dyn AccountStore, name: String) -> Result<(), String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    accounts::delete_account(&mut all, &name).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())
}

// --- Phase 4 encryption cores (UNIFIED_PLAN §3.3) -------------------------------------------
// Each returns the passphrase to cache in the session on success; the command wrapper stores
// it in `VaultSession`. No passphrase ever crosses the bridge (D4) — these are Rust-internal.

/// `encryption_status()` core: `enabled` = the store is a vault; `locked` = enabled and no
/// passphrase is held this session (`unlocked` is false).
fn encryption_status_impl(storage: &Storage, unlocked: bool) -> EncryptionStatus {
    let enabled = vault::is_encrypted(storage.dir());
    EncryptionStatus {
        enabled,
        locked: enabled && !unlocked,
    }
}

/// `set_password(new)` core: enable encryption on a not-yet-encrypted store. Returns the new
/// passphrase to hold for the session (the store is left unlocked).
fn set_password_impl(storage: &Storage, new: &str) -> Result<SecretString, String> {
    if vault::is_encrypted(storage.dir()) {
        return Err("Encryption is already enabled".to_string());
    }
    Session::enable(Storage::new(storage.dir()), new).map_err(|e| e.to_string())?;
    Ok(SecretString::from(new.to_owned()))
}

/// `change_password(old, new)` core: verify `old`, re-seal under `new`. Returns the new
/// passphrase to hold for the session.
fn change_password_impl(storage: &Storage, old: &str, new: &str) -> Result<SecretString, String> {
    let mut session =
        Session::unlock(Storage::new(storage.dir()), old).map_err(|e| e.to_string())?;
    session.change_passphrase(new).map_err(|e| e.to_string())?;
    Ok(SecretString::from(new.to_owned()))
}

/// `unlock(password)` core: verify the passphrase against the vault. Returns it to hold for
/// the session (D7); a wrong passphrase yields the `"Incorrect password"` string.
fn unlock_impl(storage: &Storage, password: &str) -> Result<SecretString, String> {
    Session::unlock(Storage::new(storage.dir()), password).map_err(|e| e.to_string())?;
    Ok(SecretString::from(password.to_owned()))
}

/// E1's account list: name (+issuer) only, no codes, no secrets.
#[tauri::command]
fn list_accounts(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
) -> Result<Vec<AccountView>, String> {
    let store = account_store(storage_for(&app)?, held_passphrase(&vault)?)?;
    list_accounts_impl(&*store)
}

/// E1's live codes: each account projected to a `CodeView` (code + period boundary), the only
/// account-derived values that reach the webview. Computed in Rust (UNIFIED_PLAN §6).
#[tauri::command]
fn get_codes(app: tauri::AppHandle, vault: State<VaultSession>) -> Result<Vec<CodeView>, String> {
    let store = account_store(storage_for(&app)?, held_passphrase(&vault)?)?;
    get_codes_impl(&*store)
}

/// E5 add: validate the secret + reject a duplicate name in `authr_core`, persist, and return
/// the created account projected to an `AccountView` (no secret crosses the bridge, D4). The
/// secret's whitespace is stripped in core ("spaces ignored"). On an encrypted store the save
/// re-encrypts with the in-memory passphrase (D7).
#[tauri::command]
fn add_account(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    name: String,
    secret: String,
) -> Result<AccountView, String> {
    let store = account_store(storage_for(&app)?, held_passphrase(&vault)?)?;
    add_account_impl(&*store, name, secret)
}

/// Inline rename from an E3 manage row. Rejects a name collision / missing account in core.
#[tauri::command]
fn rename_account(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    name: String,
    new_name: String,
) -> Result<(), String> {
    let store = account_store(storage_for(&app)?, held_passphrase(&vault)?)?;
    rename_account_impl(&*store, name, new_name)
}

/// Permanent delete from the E3 delete-confirm modal. No secret is returned (D4).
#[tauri::command]
fn delete_account(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    name: String,
) -> Result<(), String> {
    let store = account_store(storage_for(&app)?, held_passphrase(&vault)?)?;
    delete_account_impl(&*store, name)
}

/// Whether the store is encrypted and, if so, whether it's locked this session — drives the
/// Settings Encryption row and the `/unlock` gate (UNIFIED_PLAN §3.3, §3.4).
#[tauri::command]
fn encryption_status(app: tauri::AppHandle, vault: State<VaultSession>) -> EncryptionStatus {
    // A failure to resolve the dir or read the lock can't mean "encrypted" — degrade to the
    // plaintext/unlocked default so the UI never wedges on the gate.
    let Ok(storage) = storage_for(&app) else {
        return EncryptionStatus { enabled: false, locked: false };
    };
    let unlocked = held_passphrase(&vault).map(|p| p.is_some()).unwrap_or(false);
    encryption_status_impl(&storage, unlocked)
}

/// E4 set-password: enable encryption, then hold the new passphrase for the session (D7).
#[tauri::command]
fn set_password(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    new: String,
) -> Result<(), String> {
    let passphrase = set_password_impl(&storage_for(&app)?, &new)?;
    *vault.0.lock().map_err(|e| e.to_string())? = Some(passphrase);
    Ok(())
}

/// E4 change-password: verify the current passphrase, re-seal under the new one, hold the new.
#[tauri::command]
fn change_password(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    old: String,
    new: String,
) -> Result<(), String> {
    let passphrase = change_password_impl(&storage_for(&app)?, &old, &new)?;
    *vault.0.lock().map_err(|e| e.to_string())? = Some(passphrase);
    Ok(())
}

/// Unlock the store for the session (D7) — verify the passphrase, then hold it. The `/unlock`
/// gate calls this when the app opens encrypted+locked.
#[tauri::command]
fn unlock(
    app: tauri::AppHandle,
    vault: State<VaultSession>,
    password: String,
) -> Result<(), String> {
    let passphrase = unlock_impl(&storage_for(&app)?, &password)?;
    *vault.0.lock().map_err(|e| e.to_string())? = Some(passphrase);
    Ok(())
}

/// Toggle the popover: visible → hide; hidden → anchor under the tray icon, show, focus.
fn toggle_main_window(app: &tauri::AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        let _ = window.move_window(Position::TrayCenter);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(VaultSession::default())
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            get_codes,
            add_account,
            rename_account,
            delete_account,
            encryption_status,
            set_password,
            change_password,
            unlock
        ])
        .setup(|app| {
            // Hide from the Dock / app menu at runtime too (belt-and-suspenders with
            // LSUIElement in Info.plist; guide §2.1). Accessory keeps keyboard focus working.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            // Tray menu — Quit is the only guaranteed way out of a Dock-less app (guide §4.4).
            let quit = MenuItem::with_id(app, QUIT_MENU_ID, "Quit Authr", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            // Reuse the bundle icon as a template glyph for now; a dedicated monochrome
            // menu-bar glyph is Phase 6 (UNIFIED_PLAN §7 risk).
            let icon = app
                .default_window_icon()
                .cloned()
                .expect("bundle ships a default window icon");
            TrayIconBuilder::with_id("main-tray")
                .icon(icon)
                .icon_as_template(true)
                .tooltip("Authr")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id().as_ref() == QUIT_MENU_ID {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    // Let the positioner cache the tray rectangle so TrayCenter works.
                    tauri_plugin_positioner::on_tray_event(app, &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_main_window(app);
                    }
                })
                .build(app)?;

            // Auto-hide the popover when it loses focus — what makes it feel like a native
            // menu-bar panel rather than a floating window (guide §3.3).
            if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
                let w = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = w.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    //! Tier 2 command tests (UNIFIED_PLAN §9.2): drive the `_impl` helpers directly over a
    //! `Storage::new(tempdir)` store — the command glue + the real load → mutate → save
    //! round-trip and exact error strings, with no tray, no mock runtime, no `AppHandle`.
    //! Mirrors `authr_core/tests/storage.rs`.
    use super::*;
    use tempfile::TempDir;

    const TEST_SECRET: &str = "JBSWY3DPEHPK3PXP";

    fn store() -> (TempDir, Storage) {
        let dir = TempDir::new().unwrap();
        let storage = Storage::new(dir.path());
        (dir, storage)
    }

    // 1. add happy path: returns a secret-free AccountView and persists the stripped secret.
    #[test]
    fn add_account_impl_persists_and_strips_whitespace() {
        let (_dir, store) = store();
        let view =
            add_account_impl(&store, "alice".to_string(), "JBSW Y3DP EHPK 3PXP".to_string())
                .unwrap();
        assert_eq!(view.name, "alice");
        assert_eq!(view.issuer, None);

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].name, "alice");
        assert_eq!(reloaded[0].secret, TEST_SECRET);
    }

    // 2. duplicate name → the `Duplicate` string; store unchanged.
    #[test]
    fn add_account_impl_rejects_duplicate_name() {
        let (_dir, store) = store();
        add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();
        let err =
            add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap_err();
        assert_eq!(err, "Account 'alice' already exists");
        assert_eq!(store.load().unwrap().len(), 1);
    }

    // 3. invalid secret → the `InvalidSecret` string; store unchanged.
    #[test]
    fn add_account_impl_rejects_invalid_secret() {
        let (_dir, store) = store();
        let err =
            add_account_impl(&store, "bob".to_string(), "INVALID!!!".to_string()).unwrap_err();
        assert!(
            err.starts_with("Invalid secret:"),
            "unexpected error string: {err}"
        );
        assert!(store.load().unwrap().is_empty());
    }

    // 4. rename happy path: reload shows the new name with the same (immutable) secret.
    #[test]
    fn rename_account_impl_changes_name_keeps_secret() {
        let (_dir, store) = store();
        add_account_impl(&store, "old".to_string(), TEST_SECRET.to_string()).unwrap();
        rename_account_impl(&store, "old".to_string(), "new".to_string()).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].name, "new");
        assert_eq!(reloaded[0].secret, TEST_SECRET);
    }

    // 5a. rename collision → the `Duplicate` string.
    #[test]
    fn rename_account_impl_rejects_collision() {
        let (_dir, store) = store();
        add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();
        add_account_impl(&store, "bob".to_string(), TEST_SECRET.to_string()).unwrap();
        let err =
            rename_account_impl(&store, "bob".to_string(), "alice".to_string()).unwrap_err();
        assert_eq!(err, "Account 'alice' already exists");
    }

    // 5b. rename a missing name → the `NotFound` string.
    #[test]
    fn rename_account_impl_rejects_missing() {
        let (_dir, store) = store();
        let err =
            rename_account_impl(&store, "ghost".to_string(), "x".to_string()).unwrap_err();
        assert_eq!(err, "Account 'ghost' not found");
    }

    // 6a. delete happy path: reload shows the account gone.
    #[test]
    fn delete_account_impl_removes_account() {
        let (_dir, store) = store();
        add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();
        add_account_impl(&store, "bob".to_string(), TEST_SECRET.to_string()).unwrap();
        delete_account_impl(&store, "alice".to_string()).unwrap();

        let reloaded = store.load().unwrap();
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].name, "bob");
    }

    // 6b. delete a missing name → the `NotFound` string.
    #[test]
    fn delete_account_impl_rejects_missing() {
        let (_dir, store) = store();
        let err = delete_account_impl(&store, "ghost".to_string()).unwrap_err();
        assert_eq!(err, "Account 'ghost' not found");
    }

    // list_accounts reads back what add persisted (the command-glue projection, no secret).
    #[test]
    fn list_accounts_impl_projects_persisted_accounts() {
        let (_dir, store) = store();
        assert!(list_accounts_impl(&store).unwrap().is_empty());
        add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();
        let views = list_accounts_impl(&store).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "alice");
    }

    // 7. D4: an AccountView serializes without any `secret` field.
    #[test]
    fn account_view_serializes_without_secret() {
        let (_dir, store) = store();
        let view = add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("secret"), "AccountView JSON leaked a secret field: {json}");
        assert!(!json.contains(TEST_SECRET), "AccountView JSON leaked the secret: {json}");
    }

    // --- Phase 4: encryption command cores (UNIFIED_PLAN §9 extended) ----------------------

    // status on a plaintext store: not enabled, not locked.
    #[test]
    fn encryption_status_impl_plaintext_is_disabled() {
        let (_dir, store) = store();
        let s = encryption_status_impl(&store, false);
        assert!(!s.enabled);
        assert!(!s.locked);
    }

    // set_password encrypts the store: status flips to enabled, and locked tracks `unlocked`.
    #[test]
    fn set_password_impl_enables_encryption() {
        let (_dir, store) = store();
        add_account_impl(&store, "alice".to_string(), TEST_SECRET.to_string()).unwrap();

        let _pass = set_password_impl(&store, "pw").unwrap();
        assert!(vault::is_encrypted(store.dir()));
        assert!(encryption_status_impl(&store, true).enabled);
        assert!(encryption_status_impl(&store, false).locked, "no held passphrase ⇒ locked");
        assert!(!encryption_status_impl(&store, true).locked, "held passphrase ⇒ unlocked");
    }

    // set_password on an already-encrypted store is rejected.
    #[test]
    fn set_password_impl_rejects_when_already_encrypted() {
        let (_dir, store) = store();
        set_password_impl(&store, "pw").unwrap();
        let err = set_password_impl(&store, "pw2").unwrap_err();
        assert_eq!(err, "Encryption is already enabled");
    }

    // The full command-layer round-trip: encrypt → lock → unlock → read, with a write that
    // silently re-encrypts (D7), routed through `account_store` exactly like the commands.
    #[test]
    fn encrypted_round_trip_through_account_store() {
        let (dir, store) = store();
        let pass = set_password_impl(&store, "hunter2").unwrap();

        // Unlocked: a mutation re-encrypts via the in-memory passphrase.
        let unlocked = account_store(Storage::new(dir.path()), Some(pass)).unwrap();
        add_account_impl(&*unlocked, "alice".to_string(), TEST_SECRET.to_string()).unwrap();

        // Locked: no passphrase held ⇒ the store can't be opened.
        let locked = account_store(Storage::new(dir.path()), None);
        assert!(locked.is_err(), "locked store must refuse to build a backend");

        // Wrong password is rejected; the right one unlocks.
        assert_eq!(
            unlock_impl(&store, "nope").unwrap_err(),
            "Incorrect password"
        );
        let pass = unlock_impl(&store, "hunter2").unwrap();
        let reopened = account_store(Storage::new(dir.path()), Some(pass)).unwrap();
        let views = list_accounts_impl(&*reopened).unwrap();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "alice");
    }

    // change_password verifies the old passphrase and re-seals under the new one.
    #[test]
    fn change_password_impl_reseals_and_rejects_wrong_old() {
        let (dir, store) = store();
        let pass = set_password_impl(&store, "old").unwrap();
        let unlocked = account_store(Storage::new(dir.path()), Some(pass)).unwrap();
        add_account_impl(&*unlocked, "alice".to_string(), TEST_SECRET.to_string()).unwrap();

        assert_eq!(
            change_password_impl(&store, "wrong", "new").unwrap_err(),
            "Incorrect password"
        );
        let pass = change_password_impl(&store, "old", "new").unwrap();

        // Old no longer opens it; the new passphrase (returned by change) does, data intact.
        assert_eq!(unlock_impl(&store, "old").unwrap_err(), "Incorrect password");
        let reopened = account_store(Storage::new(dir.path()), Some(pass)).unwrap();
        assert_eq!(list_accounts_impl(&*reopened).unwrap()[0].name, "alice");
    }
}
