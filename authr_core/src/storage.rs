use crate::model::Account;
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

const ACCOUNTS_FILE: &str = "accounts.json";

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Could not determine config directory")]
    ConfigDirNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Plaintext account store rooted at an explicit directory.
///
/// The directory is *injected* rather than discovered, which is the clean seam the rest
/// of the plan builds on (UNIFIED_PLAN §3.2 item 1):
///   * the Tauri app passes `app_config_dir()`,
///   * tests pass a `tempfile::TempDir`,
///   * [`Storage::default_location`] keeps the old directories-based path for back-compat.
///
/// Phase 4's encrypted vault becomes a second backend behind the same access shape — see
/// [`crate::vault`].
pub struct Storage {
    dir: PathBuf,
}

impl Storage {
    /// Store rooted at `dir`. The directory is created lazily on first [`save`](Self::save).
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The platform default config directory (`directories`-based), kept available so the
    /// app has a sensible default and existing `accounts.json` files keep loading.
    pub fn default_location() -> Result<Self, StorageError> {
        let proj_dirs =
            ProjectDirs::from("", "", "authr").ok_or(StorageError::ConfigDirNotFound)?;
        Ok(Self::new(proj_dirs.config_dir()))
    }

    /// Path to the accounts file inside this store's directory.
    pub fn accounts_path(&self) -> PathBuf {
        self.dir.join(ACCOUNTS_FILE)
    }

    /// The store's root directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Load accounts, returning an empty list if the file doesn't exist yet.
    pub fn load(&self) -> Result<Vec<Account>, StorageError> {
        let path = self.accounts_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(path)?;
        let accounts = serde_json::from_str(&content)?;
        Ok(accounts)
    }

    /// Write accounts, creating the store directory if needed.
    pub fn save(&self, accounts: &[Account]) -> Result<(), StorageError> {
        fs::create_dir_all(&self.dir)?;
        let content = serde_json::to_string_pretty(accounts)?;
        fs::write(self.accounts_path(), content)?;
        Ok(())
    }
}
