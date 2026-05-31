use crate::model::Account;
use crate::totp;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AccountError {
    #[error("Account '{0}' already exists")]
    Duplicate(String),
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),
    #[error("Account '{0}' not found")]
    NotFound(String),
}

/// Validate a base32 secret by attempting to generate a TOTP code from it.
///
/// Lifted out of the old `authr_cli` so the GUI commands (and tests) can call it
/// directly. The probe account is throwaway — only the secret is exercised.
pub fn validate_secret(secret: &str) -> Result<(), AccountError> {
    let probe = Account::new("__probe__".to_string(), secret.to_string());
    totp::generate_code(&probe).map_err(|e| AccountError::InvalidSecret(e.to_string()))?;
    Ok(())
}

/// Add a validated, name-unique account to `accounts`, returning the created account.
///
/// Validates the secret by generating a code and rejects a name that already exists —
/// the two checks the CLI's `add` performed, now reusable by the Tauri `add_account`
/// command.
pub fn add_account(
    accounts: &mut Vec<Account>,
    name: String,
    secret: String,
) -> Result<Account, AccountError> {
    // Spaces (and any whitespace) in a pasted secret are ignored — the E5 hint promises
    // "spaces ignored", and base32 carries no whitespace, so this is safe to strip.
    let secret: String = secret.chars().filter(|c| !c.is_whitespace()).collect();
    let account = Account::new(name.clone(), secret);
    validate_secret(&account.secret)?;
    if accounts.iter().any(|a| a.name == name) {
        return Err(AccountError::Duplicate(name));
    }
    accounts.push(account.clone());
    Ok(account)
}

/// Rename the account named `name` to `new_name`, in place.
///
/// Mirrors [`add_account`]'s name-uniqueness check: errors with [`AccountError::NotFound`] if
/// no account has `name`, and [`AccountError::Duplicate`] if `new_name` is already taken by a
/// *different* account. The immutable secret is untouched, so codes are unaffected.
pub fn rename_account(
    accounts: &mut [Account],
    name: &str,
    new_name: String,
) -> Result<(), AccountError> {
    let idx = accounts
        .iter()
        .position(|a| a.name == name)
        .ok_or_else(|| AccountError::NotFound(name.to_string()))?;
    if accounts
        .iter()
        .enumerate()
        .any(|(i, a)| i != idx && a.name == new_name)
    {
        return Err(AccountError::Duplicate(new_name));
    }
    accounts[idx].name = new_name;
    Ok(())
}

/// Remove the account named `name`. Errors with [`AccountError::NotFound`] if absent.
///
/// Permanent and irreversible — no secret is returned to the caller (UNIFIED_PLAN D4).
pub fn delete_account(accounts: &mut Vec<Account>, name: &str) -> Result<(), AccountError> {
    let idx = accounts
        .iter()
        .position(|a| a.name == name)
        .ok_or_else(|| AccountError::NotFound(name.to_string()))?;
    accounts.remove(idx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_secret_passes() {
        assert!(validate_secret("JBSWY3DPEHPK3PXP").is_ok());
    }

    #[test]
    fn invalid_secret_fails() {
        assert!(matches!(
            validate_secret("INVALID!!!"),
            Err(AccountError::InvalidSecret(_))
        ));
    }

    #[test]
    fn add_account_appends_and_returns() {
        let mut accounts = Vec::new();
        let added = add_account(
            &mut accounts,
            "alice".to_string(),
            "JBSWY3DPEHPK3PXP".to_string(),
        )
        .unwrap();
        assert_eq!(added.name, "alice");
        assert_eq!(accounts.len(), 1);
    }

    #[test]
    fn add_account_rejects_duplicate_name() {
        let mut accounts = Vec::new();
        add_account(
            &mut accounts,
            "alice".to_string(),
            "JBSWY3DPEHPK3PXP".to_string(),
        )
        .unwrap();
        let err = add_account(
            &mut accounts,
            "alice".to_string(),
            "JBSWY3DPEHPK3PXP".to_string(),
        )
        .unwrap_err();
        assert_eq!(err, AccountError::Duplicate("alice".to_string()));
        assert_eq!(accounts.len(), 1);
    }

    #[test]
    fn add_account_rejects_invalid_secret() {
        let mut accounts = Vec::new();
        let err = add_account(
            &mut accounts,
            "bob".to_string(),
            "INVALID!!!".to_string(),
        )
        .unwrap_err();
        assert!(matches!(err, AccountError::InvalidSecret(_)));
        assert!(accounts.is_empty());
    }

    #[test]
    fn add_account_strips_whitespace_from_secret() {
        let mut accounts = Vec::new();
        let added = add_account(
            &mut accounts,
            "spaced".to_string(),
            "JBSW Y3DP EHPK 3PXP".to_string(),
        )
        .unwrap();
        assert_eq!(added.secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn rename_account_changes_name_in_place() {
        let mut accounts = Vec::new();
        add_account(&mut accounts, "old".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        rename_account(&mut accounts, "old", "new".to_string()).unwrap();
        assert_eq!(accounts[0].name, "new");
        assert_eq!(accounts[0].secret, "JBSWY3DPEHPK3PXP");
    }

    #[test]
    fn rename_account_to_same_name_is_ok() {
        let mut accounts = Vec::new();
        add_account(&mut accounts, "alice".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        rename_account(&mut accounts, "alice", "alice".to_string()).unwrap();
        assert_eq!(accounts[0].name, "alice");
    }

    #[test]
    fn rename_account_rejects_collision() {
        let mut accounts = Vec::new();
        add_account(&mut accounts, "alice".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        add_account(&mut accounts, "bob".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        let err = rename_account(&mut accounts, "bob", "alice".to_string()).unwrap_err();
        assert_eq!(err, AccountError::Duplicate("alice".to_string()));
        assert_eq!(accounts[1].name, "bob");
    }

    #[test]
    fn rename_account_rejects_missing() {
        let mut accounts: Vec<Account> = Vec::new();
        let err = rename_account(&mut accounts, "ghost", "x".to_string()).unwrap_err();
        assert_eq!(err, AccountError::NotFound("ghost".to_string()));
    }

    #[test]
    fn delete_account_removes_by_name() {
        let mut accounts = Vec::new();
        add_account(&mut accounts, "alice".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        add_account(&mut accounts, "bob".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        delete_account(&mut accounts, "alice").unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].name, "bob");
    }

    #[test]
    fn delete_account_rejects_missing() {
        let mut accounts: Vec<Account> = Vec::new();
        let err = delete_account(&mut accounts, "ghost").unwrap_err();
        assert_eq!(err, AccountError::NotFound("ghost".to_string()));
    }
}
