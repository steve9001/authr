use crate::model::{Account, CodeView};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use totp_rs::{Algorithm, Secret, TOTP};

/// TOTP step length. RFC 6238 default; the UI's countdown bar assumes it.
pub const PERIOD_SECONDS: u64 = 30;

#[derive(Error, Debug)]
pub enum TotpError {
    #[error("Invalid secret: {0}")]
    InvalidSecret(String),
    #[error("Generation error: {0}")]
    Generation(String),
}

fn totp_for(account: &Account) -> Result<TOTP, TotpError> {
    // Assume base32 encoded secret
    let secret = Secret::Encoded(account.secret.clone());
    let secret_bytes = secret
        .to_bytes()
        .map_err(|e| TotpError::InvalidSecret(e.to_string()))?;

    Ok(TOTP::new_unchecked(
        Algorithm::SHA1,
        6,
        1,
        PERIOD_SECONDS,
        secret_bytes,
    ))
}

fn now_unix() -> Result<u64, TotpError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| TotpError::Generation(e.to_string()))
}

pub fn generate_code(account: &Account) -> Result<String, TotpError> {
    let totp = totp_for(account)?;
    totp.generate_current()
        .map_err(|e| TotpError::Generation(e.to_string()))
}

/// Generate the current code along with the unix timestamp at which it expires (the
/// next period boundary). Code and validity are computed against the same `now`, so the
/// returned `valid_until_unix` is exactly when this code rolls over — that's what drives
/// the UI's single global countdown bar rather than a client-side guess.
pub fn generate_with_validity(account: &Account) -> Result<(String, u64), TotpError> {
    let totp = totp_for(account)?;
    let now = now_unix()?;
    let code = totp.generate(now);
    let valid_until_unix = (now / PERIOD_SECONDS + 1) * PERIOD_SECONDS;
    Ok((code, valid_until_unix))
}

/// Project an account into a [`CodeView`] for the bridge — code + period boundary, no
/// secret (UNIFIED_PLAN D4).
pub fn generate_code_view(account: &Account) -> Result<CodeView, TotpError> {
    let (code, valid_until_unix) = generate_with_validity(account)?;
    Ok(CodeView {
        name: account.name.clone(),
        issuer: account.issuer.clone(),
        code,
        period_seconds: PERIOD_SECONDS,
        valid_until_unix,
    })
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

    #[test]
    fn validity_lands_on_a_period_boundary_in_the_future() {
        let account = Account::new("test".to_string(), "JBSWY3DPEHPK3PXP".to_string());
        let (code, valid_until) = generate_with_validity(&account).unwrap();
        assert_eq!(code.len(), 6);
        assert_eq!(valid_until % PERIOD_SECONDS, 0);
        let now = now_unix().unwrap();
        assert!(valid_until > now);
        assert!(valid_until - now <= PERIOD_SECONDS);
    }

    #[test]
    fn code_view_carries_no_secret_fields() {
        let account = Account::new("test".to_string(), "JBSWY3DPEHPK3PXP".to_string());
        let view = generate_code_view(&account).unwrap();
        assert_eq!(view.name, "test");
        assert_eq!(view.period_seconds, PERIOD_SECONDS);
        // CodeView has no `secret` field by construction (D4) — this is a compile-time
        // guarantee; the test documents the intent.
    }
}
