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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_normalization() {
        let account = Account::new("test".to_string(), "jbswy3dpehpk3pxp".to_string());
        assert_eq!(account.secret, "JBSWY3DPEHPK3PXP");
    }
}
