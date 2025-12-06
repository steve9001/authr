use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;
use crate::model::Account;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Could not determine config directory")]
    ConfigDirNotFound,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub fn load_accounts() -> Result<Vec<Account>, StorageError> {
    let path = get_config_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)?;
    let accounts = serde_json::from_str(&content)?;
    Ok(accounts)
}

pub fn save_accounts(accounts: &[Account]) -> Result<(), StorageError> {
    let path = get_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(accounts)?;
    fs::write(path, content)?;
    Ok(())
}

fn get_config_path() -> Result<PathBuf, StorageError> {
    // "authr" as application name
    let proj_dirs = ProjectDirs::from("", "", "authr")
        .ok_or(StorageError::ConfigDirNotFound)?;
    Ok(proj_dirs.config_dir().join("accounts.json"))
}
