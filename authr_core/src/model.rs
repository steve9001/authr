use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub name: String,
    pub issuer: Option<String>,
    pub secret: String, // Stored as base32 string usually
}

impl Account {
    pub fn new(name: String, secret: String) -> Self {
        Self {
            name,
            issuer: None,
            secret: secret.to_uppercase(),
        }
    }
}

/// Account data safe to cross the Rust ⇄ webview bridge for E1's list — name (+issuer)
/// only. The base32 `secret` is deliberately absent (UNIFIED_PLAN D4: secrets never
/// cross the bridge).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountView {
    pub name: String,
    pub issuer: Option<String>,
}

impl From<&Account> for AccountView {
    fn from(account: &Account) -> Self {
        Self {
            name: account.name.clone(),
            issuer: account.issuer.clone(),
        }
    }
}

/// A generated code plus the period boundary that drives the UI's single global
/// countdown bar. Like [`AccountView`], it carries no secret (D4); the 6-digit `code`
/// is the only account-derived value that reaches the webview.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodeView {
    pub name: String,
    pub issuer: Option<String>,
    pub code: String,
    /// TOTP step length in seconds (30).
    pub period_seconds: u64,
    /// Unix time (seconds) at which `code` expires — the next period boundary.
    pub valid_until_unix: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_normalization() {
        let account = Account::new("test".to_string(), "jbswy3dpehpk3pxp".to_string());
        assert_eq!(account.secret, "JBSWY3DPEHPK3PXP");
    }
}
