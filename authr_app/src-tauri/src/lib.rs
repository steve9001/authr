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
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, Manager, WindowEvent,
};
use tauri_plugin_positioner::{Position, WindowExt};

/// Stable label for the single popover window (declared in `tauri.conf.json`).
const MAIN_WINDOW: &str = "main";
/// Stable id for the tray menu's Quit item.
const QUIT_MENU_ID: &str = "quit";

/// The plaintext store rooted at this app's OS config dir. Phase 4 swaps the backend behind
/// the same shape (UNIFIED_PLAN §3.2 item 4) — these call sites do not change.
fn storage_for(app: &tauri::AppHandle) -> Result<Storage, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    Ok(Storage::new(dir))
}

// Each command's testable core: a plain helper over a `&Storage` (the `AccountStore` seam),
// with no `AppHandle` and no Tauri runtime, so Tier 2 can drive the real load → mutate → save
// round-trip against a tempfile store (UNIFIED_PLAN §9.2). Each `#[tauri::command]` is the
// one-liner that resolves the OS config dir via `storage_for` and delegates here.

/// E1's account list: name (+issuer) only, no codes, no secrets.
fn list_accounts_impl(store: &Storage) -> Result<Vec<AccountView>, String> {
    let accounts = store.load().map_err(|e| e.to_string())?;
    Ok(accounts.iter().map(AccountView::from).collect())
}

/// E1's live codes: each account projected to a `CodeView` (code + period boundary).
fn get_codes_impl(store: &Storage) -> Result<Vec<CodeView>, String> {
    let accounts = store.load().map_err(|e| e.to_string())?;
    accounts
        .iter()
        .map(|a| totp::generate_code_view(a).map_err(|e| e.to_string()))
        .collect()
}

/// E5 add core: validate the secret + reject a duplicate name in `authr_core`, persist, and
/// return the created account projected to an `AccountView` (no secret crosses the bridge, D4).
/// The secret's whitespace is stripped in core ("spaces ignored").
fn add_account_impl(store: &Storage, name: String, secret: String) -> Result<AccountView, String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    let added = accounts::add_account(&mut all, name, secret).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())?;
    Ok(AccountView::from(&added))
}

/// Inline rename core. Rejects a name collision / missing account in core.
fn rename_account_impl(store: &Storage, name: String, new_name: String) -> Result<(), String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    accounts::rename_account(&mut all, &name, new_name).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())
}

/// Permanent delete core. No secret is returned (D4).
fn delete_account_impl(store: &Storage, name: String) -> Result<(), String> {
    let mut all = store.load().map_err(|e| e.to_string())?;
    accounts::delete_account(&mut all, &name).map_err(|e| e.to_string())?;
    store.save(&all).map_err(|e| e.to_string())
}

/// E1's account list: name (+issuer) only, no codes, no secrets.
#[tauri::command]
fn list_accounts(app: tauri::AppHandle) -> Result<Vec<AccountView>, String> {
    list_accounts_impl(&storage_for(&app)?)
}

/// E1's live codes: each account projected to a `CodeView` (code + period boundary), the only
/// account-derived values that reach the webview. Computed in Rust (UNIFIED_PLAN §6).
#[tauri::command]
fn get_codes(app: tauri::AppHandle) -> Result<Vec<CodeView>, String> {
    get_codes_impl(&storage_for(&app)?)
}

/// E5 add: validate the secret + reject a duplicate name in `authr_core`, persist, and return
/// the created account projected to an `AccountView` (no secret crosses the bridge, D4). The
/// secret's whitespace is stripped in core ("spaces ignored").
#[tauri::command]
fn add_account(app: tauri::AppHandle, name: String, secret: String) -> Result<AccountView, String> {
    add_account_impl(&storage_for(&app)?, name, secret)
}

/// Inline rename from an E3 manage row. Rejects a name collision / missing account in core.
#[tauri::command]
fn rename_account(app: tauri::AppHandle, name: String, new_name: String) -> Result<(), String> {
    rename_account_impl(&storage_for(&app)?, name, new_name)
}

/// Permanent delete from the E3 delete-confirm modal. No secret is returned (D4).
#[tauri::command]
fn delete_account(app: tauri::AppHandle, name: String) -> Result<(), String> {
    delete_account_impl(&storage_for(&app)?, name)
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
        .invoke_handler(tauri::generate_handler![
            list_accounts,
            get_codes,
            add_account,
            rename_account,
            delete_account
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
}
