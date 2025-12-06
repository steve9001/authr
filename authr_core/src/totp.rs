use totp_rs::{Algorithm, TOTP, Secret};
use crate::model::Account;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TotpError {
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),
    #[error("Generation error: {0}")]
    Generation(String),
}

pub fn generate_code(account: &Account) -> Result<String, TotpError> {
    // Assume base32 encoded secret
    let secret = Secret::Encoded(account.secret.clone());
    let secret_bytes = secret.to_bytes()
        .map_err(|e| TotpError::InvalidSecret(e.to_string()))?;

    let totp = TOTP::new_unchecked(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret_bytes,
    );
    
    totp.generate_current().map_err(|e| TotpError::Generation(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_secret() {
        let account = Account::new("test".to_string(), "JBSWY3DPEHPK3PXP".to_string());
        assert!(generate_code(&account).is_ok());
    }

    #[test]
    fn test_invalid_secret() {
        let account = Account::new("test".to_string(), "INVALID!!!".to_string());
        assert!(generate_code(&account).is_err());
    }
}
