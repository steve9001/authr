//! Encryption seam — **DESIGN ONLY**, implemented in Phase 4 (UNIFIED_PLAN §3.2 item 4,
//! D5/D7). Nothing here pulls in the `age` crate yet; this file exists so Phase 4 slots
//! in without churning storage's callers.
//!
//! ## The shape of the seam
//!
//! On disk the store is one of:
//!   * plaintext `accounts.json` — today's format, read/written by [`crate::storage::Storage`],
//!     kept forever for back-compat, or
//!   * an `age`-passphrase-encrypted vault — the same serialized `Vec<Account>` sealed with
//!     scrypt (KDF) + AEAD via `age::Encryptor::with_user_passphrase` (D5).
//!
//! Callers (the Phase 2/3 Tauri commands) talk to the [`AccountStore`] trait, **not** to a
//! concrete type. Today the only implementor is the plaintext `Storage`. Phase 4 adds
//! `Session` (sketched below) as a second implementor and the command call sites — `store.load()`
//! / `store.save(&accounts)` — do not change. That is the whole point of introducing the
//! trait now.

use crate::model::Account;
use crate::storage::{Storage, StorageError};

/// The narrow persistence surface every store backend offers. Command handlers depend on
/// this rather than on `Storage` directly, so Phase 4's encrypted backend drops in without
/// touching their call sites.
pub trait AccountStore {
    fn load(&self) -> Result<Vec<Account>, StorageError>;
    fn save(&self, accounts: &[Account]) -> Result<(), StorageError>;
}

impl AccountStore for Storage {
    fn load(&self) -> Result<Vec<Account>, StorageError> {
        Storage::load(self)
    }
    fn save(&self, accounts: &[Account]) -> Result<(), StorageError> {
        Storage::save(self, accounts)
    }
}

// ---------------------------------------------------------------------------------------
// Phase 4 sketch — DO NOT IMPLEMENT YET. Kept as a doc-comment so it compiles with no deps.
// ---------------------------------------------------------------------------------------
//
// /// Which on-disk format a store directory currently holds. The read path auto-detects this
// /// (sniff the `age` armor/header, or a `.vault` filename) so encrypted and legacy-plaintext
// /// stores both load — back-compat (Phase 4 exit criteria).
// pub enum StoreFormat { Plaintext, Encrypted }
//
// pub fn detect(dir: &std::path::Path) -> StoreFormat { /* sniff header / filename */ }
//
// /// An unlocked, in-memory session (UNIFIED_PLAN D7). Holds the passphrase as a zeroized
// /// secret *for the life of the process* alongside its `Storage`. Because `age` passphrase
// /// encryption re-runs scrypt over `passphrase + a fresh random salt` on every seal, a write
// /// needs only the passphrase — never a precomputed key — so `save` re-encrypts silently with
// /// no re-prompt. (If scrypt-on-save ever hurts, switch to envelope encryption: a long-lived
// /// random data key wrapped by the passphrase key. Note it, don't build it.)
// pub struct Session {
//     storage: Storage,
//     passphrase: zeroize::Zeroizing<String>,
// }
//
// impl Session {
//     /// Decrypt an existing encrypted store; errors on wrong passphrase.
//     pub fn unlock(storage: Storage, passphrase: &str) -> Result<Self, VaultError> { todo!() }
//     /// First-time `set_password`: encrypt a (possibly plaintext) store under `passphrase`.
//     pub fn enable(storage: Storage, passphrase: &str) -> Result<Self, VaultError> { todo!() }
//     /// `change_password`: re-seal under a new passphrase.
//     pub fn change_passphrase(&mut self, new: &str) -> Result<(), VaultError> { todo!() }
// }
//
// impl AccountStore for Session {
//     fn load(&self) -> Result<Vec<Account>, StorageError> { /* age-decrypt with self.passphrase */ }
//     fn save(&self, accounts: &[Account]) -> Result<(), StorageError> { /* scrypt + AEAD re-seal */ }
// }
//
// #[derive(thiserror::Error, Debug)]
// pub enum VaultError { /* WrongPassphrase, Storage(StorageError), Crypto(..) */ }
