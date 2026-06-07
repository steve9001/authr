//! Encryption seam (UNIFIED_PLAN §3.2 item 4, D5/D7) — implemented in Phase 4.
//!
//! On disk the store is one of:
//!   * plaintext `accounts.json` — today's format, read/written by [`crate::storage::Storage`],
//!     kept forever for back-compat, or
//!   * an `age`-passphrase-encrypted vault `accounts.vault` — the same serialized
//!     `Vec<Account>` sealed with scrypt (KDF) + ChaCha20-Poly1305 AEAD via
//!     [`age::Encryptor::with_user_passphrase`] (D5).
//!
//! Callers (the Tauri commands) talk to the [`AccountStore`] trait, **not** a concrete type.
//! The plaintext [`Storage`] is one implementor; [`Session`] — an unlocked, in-memory
//! decryption of an encrypted store — is the other. The command call sites (`store.load()` /
//! `store.save(&accounts)`) do not change between the two; that is the point of the trait.
//!
//! ## Re-encrypting on every write without re-prompting (D7)
//!
//! `age` passphrase encryption runs scrypt over `passphrase + a fresh random salt` on every
//! seal, then AEAD-seals the data; decryption re-derives the key from the embedded salt. So a
//! *write* needs only the passphrase, never a precomputed key. [`Session`] holds the passphrase
//! (as a zeroizing [`SecretString`]) for the life of the process, so `add`/`rename`/`delete` →
//! [`Session::save`] simply re-runs scrypt with a new salt and rewrites the file — the user is
//! never re-prompted. The accounts file is tiny and writes are occasional, so paying
//! scrypt-on-save is fine; if it ever mattered we'd switch to envelope encryption (a long-lived
//! random data key wrapped by the passphrase key) — noted, not built.

use crate::model::Account;
use crate::storage::{Storage, StorageError};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

/// Re-exported so the Tauri layer can hold the session passphrase as a zeroizing secret
/// without taking a direct dependency on `age`.
pub use age::secrecy::SecretString;

/// Filename of the encrypted vault inside a store directory. Its presence is what
/// [`is_encrypted`] detects; the plaintext store uses `accounts.json` (see [`Storage`]).
pub const VAULT_FILE: &str = "accounts.vault";

/// The narrow persistence surface every store backend offers. Command handlers depend on
/// this rather than on a concrete type, so the encrypted [`Session`] drops in behind the
/// same call sites as the plaintext [`Storage`].
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

/// Whether `dir` currently holds an encrypted vault — i.e. encryption is enabled. Drives
/// `encryption_status().enabled` and the unlock gate.
pub fn is_encrypted(dir: &Path) -> bool {
    dir.join(VAULT_FILE).exists()
}

/// Whether `bytes` is an `age`-encrypted payload rather than plaintext JSON. The import flow
/// (UNIFIED_PLAN §5 Phase 5) uses this to tell an encrypted `.authr` backup from a plaintext
/// one so it can prompt for the file's password. `age`'s binary format begins with its
/// version line `age-encryption.org/v1`.
pub fn is_encrypted_data(bytes: &[u8]) -> bool {
    bytes.starts_with(b"age-encryption.org")
}

/// Seal a set of accounts under a **backup's own** password (UNIFIED_PLAN D6) — independent of
/// any at-rest passphrase. Reuses the same `age` scrypt+AEAD path as the live vault so the
/// `.authr` file round-trips back through [`decrypt_accounts`]. No hand-rolled crypto.
pub fn encrypt_accounts(password: &str, accounts: &[Account]) -> Result<Vec<u8>, VaultError> {
    let plaintext = serde_json::to_vec(accounts)?;
    encrypt_bytes(&SecretString::from(password.to_owned()), &plaintext)
}

/// Open an encrypted backup produced by [`encrypt_accounts`]. A wrong password yields
/// [`VaultError::WrongPassphrase`] (surfaced to the import UI as "Incorrect password").
pub fn decrypt_accounts(password: &str, ciphertext: &[u8]) -> Result<Vec<Account>, VaultError> {
    let plaintext = decrypt_bytes(&SecretString::from(password.to_owned()), ciphertext)?;
    Ok(serde_json::from_slice(&plaintext)?)
}

#[derive(Error, Debug)]
pub enum VaultError {
    /// Decryption failed because the supplied passphrase was wrong — the one error the unlock
    /// / change-password UI surfaces as "Incorrect password".
    #[error("Incorrect password")]
    WrongPassphrase,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Encryption error: {0}")]
    Encrypt(String),
    #[error("Decryption error: {0}")]
    Decrypt(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Seal `plaintext` under `passphrase` (scrypt KDF + AEAD, fresh salt each call).
fn encrypt_bytes(passphrase: &SecretString, plaintext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let encryptor = age::Encryptor::with_user_passphrase(passphrase.clone());
    let mut out = Vec::new();
    let mut writer = encryptor
        .wrap_output(&mut out)
        .map_err(|e| VaultError::Encrypt(e.to_string()))?;
    writer
        .write_all(plaintext)
        .map_err(|e| VaultError::Encrypt(e.to_string()))?;
    writer
        .finish()
        .map_err(|e| VaultError::Encrypt(e.to_string()))?;
    Ok(out)
}

/// Open `ciphertext` with `passphrase`. A wrong passphrase yields [`VaultError::WrongPassphrase`]
/// (the scrypt identity simply fails to unwrap the file key); malformed input yields `Decrypt`.
fn decrypt_bytes(passphrase: &SecretString, ciphertext: &[u8]) -> Result<Vec<u8>, VaultError> {
    let decryptor =
        age::Decryptor::new(ciphertext).map_err(|e| VaultError::Decrypt(e.to_string()))?;
    let identity = age::scrypt::Identity::new(passphrase.clone());
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| VaultError::WrongPassphrase)?;
    let mut out = Vec::new();
    reader
        .read_to_end(&mut out)
        .map_err(|e| VaultError::Decrypt(e.to_string()))?;
    Ok(out)
}

/// An unlocked, in-memory session over an encrypted store (UNIFIED_PLAN D7). Holds the
/// passphrase as a zeroizing [`SecretString`] for the life of the process alongside its
/// [`Storage`] (the store directory). Because `age` re-runs scrypt on every seal, writes need
/// only the passphrase — so [`save`](Session::save) re-encrypts silently with no re-prompt.
pub struct Session {
    storage: Storage,
    passphrase: SecretString,
}

impl Session {
    /// Decrypt an existing encrypted store to verify `passphrase`, returning the unlocked
    /// session. Errors with [`VaultError::WrongPassphrase`] if the passphrase is wrong.
    pub fn unlock(storage: Storage, passphrase: &str) -> Result<Self, VaultError> {
        let session = Self {
            storage,
            passphrase: SecretString::from(passphrase.to_owned()),
        };
        // A successful decrypt is the proof the passphrase is correct.
        session.load_accounts()?;
        Ok(session)
    }

    /// First-time `set_password`: encrypt a (plaintext or empty) store under `passphrase` and
    /// remove the now-superseded plaintext `accounts.json`. Returns the unlocked session.
    pub fn enable(storage: Storage, passphrase: &str) -> Result<Self, VaultError> {
        let accounts = storage.load()?; // reads existing plaintext, or [] if none
        let session = Self {
            storage,
            passphrase: SecretString::from(passphrase.to_owned()),
        };
        session.write_encrypted(&accounts)?;
        let plain = session.storage.accounts_path();
        if plain.exists() {
            fs::remove_file(plain)?;
        }
        Ok(session)
    }

    /// `change_password`: re-seal the current accounts under a new passphrase. The caller has
    /// already proven the old passphrase (this session is unlocked).
    pub fn change_passphrase(&mut self, new: &str) -> Result<(), VaultError> {
        let accounts = self.load_accounts()?;
        self.passphrase = SecretString::from(new.to_owned());
        self.write_encrypted(&accounts)?;
        Ok(())
    }

    /// `disable` (remove password): decrypt the vault and restore the plaintext `accounts.json`,
    /// then delete the vault. The caller has already proven the passphrase (this session is
    /// unlocked). Order matters: write the plaintext *first*, delete the vault *last*, so a crash
    /// mid-way never loses data (worst case both files exist and the vault still wins on launch).
    pub fn disable(self) -> Result<(), VaultError> {
        let accounts = self.load_accounts()?; // decrypt in memory
        self.storage.save(&accounts)?; // write plaintext accounts.json
        let vault = self.vault_path();
        if vault.exists() {
            fs::remove_file(vault)?;
        }
        Ok(())
    }

    /// Resume an already-unlocked session from a passphrase held in memory — no I/O, no
    /// verification. The command layer caches the [`SecretString`] across calls (D7) and
    /// rebuilds the session per command with this; the next `load`/`save` does the crypto.
    pub fn resume(storage: Storage, passphrase: SecretString) -> Self {
        Self {
            storage,
            passphrase,
        }
    }

    fn vault_path(&self) -> std::path::PathBuf {
        self.storage.dir().join(VAULT_FILE)
    }

    fn load_accounts(&self) -> Result<Vec<Account>, VaultError> {
        let ciphertext = fs::read(self.vault_path())?;
        let plaintext = decrypt_bytes(&self.passphrase, &ciphertext)?;
        let accounts = serde_json::from_slice(&plaintext)?;
        Ok(accounts)
    }

    fn write_encrypted(&self, accounts: &[Account]) -> Result<(), VaultError> {
        fs::create_dir_all(self.storage.dir())?;
        let plaintext = serde_json::to_vec(accounts)?;
        let ciphertext = encrypt_bytes(&self.passphrase, &plaintext)?;
        fs::write(self.vault_path(), ciphertext)?;
        Ok(())
    }
}

impl AccountStore for Session {
    fn load(&self) -> Result<Vec<Account>, StorageError> {
        self.load_accounts()
            .map_err(|e| StorageError::Vault(e.to_string()))
    }
    fn save(&self, accounts: &[Account]) -> Result<(), StorageError> {
        self.write_encrypted(accounts)
            .map_err(|e| StorageError::Vault(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const SECRET: &str = "JBSWY3DPEHPK3PXP";

    fn acct(name: &str) -> Account {
        Account::new(name.to_string(), SECRET.to_string())
    }

    // Low-level crypto round-trips, and a different passphrase fails to open the ciphertext.
    #[test]
    fn encrypt_decrypt_round_trips() {
        let pass = SecretString::from("correct horse".to_owned());
        let ciphertext = encrypt_bytes(&pass, b"hello vault").unwrap();
        assert_ne!(ciphertext, b"hello vault"); // actually sealed
        let plaintext = decrypt_bytes(&pass, &ciphertext).unwrap();
        assert_eq!(plaintext, b"hello vault");
    }

    #[test]
    fn decrypt_with_wrong_passphrase_is_rejected() {
        let pass = SecretString::from("right".to_owned());
        let ciphertext = encrypt_bytes(&pass, b"secret data").unwrap();
        let wrong = SecretString::from("wrong".to_owned());
        let err = decrypt_bytes(&wrong, &ciphertext).unwrap_err();
        assert!(matches!(err, VaultError::WrongPassphrase));
    }

    // enable → an encrypted store with the plaintext file removed; the seam reads it back.
    #[test]
    fn enable_encrypts_and_removes_plaintext() {
        let dir = TempDir::new().unwrap();
        let storage = Storage::new(dir.path());
        storage.save(&[acct("alice")]).unwrap();
        assert!(storage.accounts_path().exists());

        let session = Session::enable(Storage::new(dir.path()), "pw").unwrap();
        assert!(is_encrypted(dir.path()));
        assert!(!storage.accounts_path().exists(), "plaintext file left behind");

        let loaded = session.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "alice");
        assert_eq!(loaded[0].secret, SECRET);
    }

    // The full encrypt → lock → unlock → read cycle, with a write in between (silent re-encrypt).
    #[test]
    fn round_trip_encrypt_lock_unlock_read() {
        let dir = TempDir::new().unwrap();
        let session = Session::enable(Storage::new(dir.path()), "hunter2").unwrap();
        session.save(&[acct("alice"), acct("bob")]).unwrap();

        // "Lock" = drop the session (passphrase leaves memory).
        drop(session);
        assert!(is_encrypted(dir.path()));

        // Wrong passphrase is rejected.
        assert!(matches!(
            Session::unlock(Storage::new(dir.path()), "nope"),
            Err(VaultError::WrongPassphrase)
        ));

        // Right passphrase unlocks and reads back what was written.
        let session = Session::unlock(Storage::new(dir.path()), "hunter2").unwrap();
        let names: Vec<_> = session.load().unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(names, vec!["alice", "bob"]);
    }

    // change_passphrase re-seals: the old passphrase no longer opens the store, the new one does.
    #[test]
    fn change_passphrase_reseals_under_new_password() {
        let dir = TempDir::new().unwrap();
        let mut session = Session::enable(Storage::new(dir.path()), "old").unwrap();
        session.save(&[acct("alice")]).unwrap();
        session.change_passphrase("new").unwrap();
        drop(session);

        assert!(matches!(
            Session::unlock(Storage::new(dir.path()), "old"),
            Err(VaultError::WrongPassphrase)
        ));
        let session = Session::unlock(Storage::new(dir.path()), "new").unwrap();
        assert_eq!(session.load().unwrap()[0].name, "alice");
    }

    // disable round-trips back to plaintext: accounts.json returns, the vault is gone, data intact.
    #[test]
    fn disable_writes_plaintext_and_removes_vault() {
        let dir = TempDir::new().unwrap();
        let session = Session::enable(Storage::new(dir.path()), "pw").unwrap();
        session.save(&[acct("alice"), acct("bob")]).unwrap();

        Session::unlock(Storage::new(dir.path()), "pw")
            .unwrap()
            .disable()
            .unwrap();

        assert!(!is_encrypted(dir.path()), "vault left behind");
        let storage = Storage::new(dir.path());
        assert!(storage.accounts_path().exists(), "plaintext file not written");
        let names: Vec<_> = storage.load().unwrap().into_iter().map(|a| a.name).collect();
        assert_eq!(names, vec!["alice", "bob"]);
    }

    // A fresh (never-saved) directory is not encrypted.
    #[test]
    fn empty_dir_is_not_encrypted() {
        let dir = TempDir::new().unwrap();
        assert!(!is_encrypted(dir.path()));
    }

    // Backup helpers (D6): encrypt under the backup's own password and read it back.
    #[test]
    fn encrypt_decrypt_accounts_round_trips() {
        let accounts = vec![acct("alice"), acct("bob")];
        let sealed = encrypt_accounts("backup-pw", &accounts).unwrap();
        assert!(is_encrypted_data(&sealed));
        let opened = decrypt_accounts("backup-pw", &sealed).unwrap();
        let names: Vec<_> = opened.iter().map(|a| a.name.clone()).collect();
        assert_eq!(names, vec!["alice", "bob"]);
        assert_eq!(opened[0].secret, SECRET);
    }

    #[test]
    fn decrypt_accounts_rejects_wrong_password() {
        let sealed = encrypt_accounts("right", &[acct("alice")]).unwrap();
        assert!(matches!(
            decrypt_accounts("wrong", &sealed),
            Err(VaultError::WrongPassphrase)
        ));
    }

    // Plaintext JSON is not mistaken for an encrypted payload.
    #[test]
    fn plaintext_json_is_not_detected_as_encrypted() {
        let json = serde_json::to_vec(&[acct("alice")]).unwrap();
        assert!(!is_encrypted_data(&json));
    }

    // The ciphertext on disk never contains the plaintext secret (D6/at-rest guarantee).
    #[test]
    fn vault_file_does_not_leak_the_secret() {
        let dir = TempDir::new().unwrap();
        let session = Session::enable(Storage::new(dir.path()), "pw").unwrap();
        session.save(&[acct("alice")]).unwrap();
        let bytes = fs::read(dir.path().join(VAULT_FILE)).unwrap();
        // SECRET is ASCII; assert its byte sequence is absent from the sealed file.
        assert!(
            bytes.windows(SECRET.len()).all(|w| w != SECRET.as_bytes()),
            "vault file leaked the plaintext secret"
        );
    }
}
