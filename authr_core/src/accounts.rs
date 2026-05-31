use crate::model::Account;
use crate::totp;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum AccountError {
    #[error("Account '{0}' already exists")]
    Duplicate(String),
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),
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
    let account = Account::new(name.clone(), secret);
    validate_secret(&account.secret)?;
    if accounts.iter().any(|a| a.name == name) {
        return Err(AccountError::Duplicate(name));
    }
    accounts.push(account.clone());
    Ok(account)
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
}
