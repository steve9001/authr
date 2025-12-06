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
            secret,
        }
    }
}
