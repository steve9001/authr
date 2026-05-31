//! authr tray app — backend lifecycle + the narrow command surface.
//!
//! Phase 2 (UNIFIED_PLAN §5): a single hidden popover toggled from a menu-bar tray icon,
//! anchored under the icon, auto-hiding on focus loss, with an explicit Quit. Two read-only
//! commands (`list_accounts`, `get_codes`) read through `authr_core::storage::Storage` rooted
//! at the OS config dir. Secrets never cross the bridge (D4) — only `AccountView`/`CodeView`.

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

/// E1's account list: name (+issuer) only, no codes, no secrets.
#[tauri::command]
fn list_accounts(app: tauri::AppHandle) -> Result<Vec<AccountView>, String> {
    let accounts = storage_for(&app)?.load().map_err(|e| e.to_string())?;
    Ok(accounts.iter().map(AccountView::from).collect())
}

/// E1's live codes: each account projected to a `CodeView` (code + period boundary), the only
/// account-derived values that reach the webview. Computed in Rust (UNIFIED_PLAN §6).
#[tauri::command]
fn get_codes(app: tauri::AppHandle) -> Result<Vec<CodeView>, String> {
    let accounts = storage_for(&app)?.load().map_err(|e| e.to_string())?;
    accounts
        .iter()
        .map(|a| totp::generate_code_view(a).map_err(|e| e.to_string()))
        .collect()
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
        .invoke_handler(tauri::generate_handler![list_accounts, get_codes])
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
