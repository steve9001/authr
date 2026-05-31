//! Integration tests exercising `authr_core` through an injected storage directory.
//!
//! These are the old `authr_cli/tests/integration.rs` cases (list-empty / add+list /
//! remove / show), ported off the deleted CLI binary to drive the library directly via
//! `Storage::new(tempdir)` (UNIFIED_PLAN Phase 1, item 4).

use authr_core::accounts::add_account;
use authr_core::model::Account;
use authr_core::storage::Storage;
use authr_core::totp;
use authr_core::vault::AccountStore;
use tempfile::TempDir;

const TEST_SECRET: &str = "JBSWY3DPEHPK3PXP";

fn store() -> (TempDir, Storage) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::new(dir.path());
    (dir, storage)
}

// was: `authr list` on a fresh HOME → "No accounts found"
#[test]
fn list_on_empty_store_is_empty() {
    let (_dir, storage) = store();
    assert!(storage.load().unwrap().is_empty());
}

// was: `authr add testuser` then `authr list` shows it
#[test]
fn add_then_list_roundtrips() {
    let (_dir, storage) = store();

    let mut accounts = storage.load().unwrap();
    add_account(&mut accounts, "testuser".to_string(), TEST_SECRET.to_string()).unwrap();
    storage.save(&accounts).unwrap();

    let reloaded = storage.load().unwrap();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(reloaded[0].name, "testuser");
}

// was: add then `authr remove toremove` then `authr list` → empty
#[test]
fn remove_then_list_is_empty() {
    let (_dir, storage) = store();

    let mut accounts = Vec::new();
    add_account(&mut accounts, "toremove".to_string(), TEST_SECRET.to_string()).unwrap();
    storage.save(&accounts).unwrap();

    let mut accounts = storage.load().unwrap();
    accounts.retain(|a| a.name != "toremove");
    storage.save(&accounts).unwrap();

    assert!(storage.load().unwrap().is_empty());
}

// was: `authr show myservice` prints a 6-digit code
#[test]
fn show_produces_six_digit_code() {
    let account = Account::new("myservice".to_string(), TEST_SECRET.to_string());
    let code = totp::generate_code(&account).unwrap();
    assert_eq!(code.len(), 6);
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

// the storage dir really is injected: two stores over different dirs don't see each other
#[test]
fn separate_dirs_are_isolated() {
    let (_dir_a, a) = store();
    let (_dir_b, b) = store();

    let mut accounts = Vec::new();
    add_account(&mut accounts, "only-in-a".to_string(), TEST_SECRET.to_string()).unwrap();
    a.save(&accounts).unwrap();

    assert_eq!(a.load().unwrap().len(), 1);
    assert!(b.load().unwrap().is_empty());
}

// the persistence seam (AccountStore) is what Phase 4 swaps; confirm Storage satisfies it
#[test]
fn storage_is_usable_through_the_account_store_seam() {
    let (_dir, storage) = store();
    let store: &dyn AccountStore = &storage;

    let mut accounts = store.load().unwrap();
    add_account(&mut accounts, "via-trait".to_string(), TEST_SECRET.to_string()).unwrap();
    store.save(&accounts).unwrap();

    assert_eq!(store.load().unwrap()[0].name, "via-trait");
}
