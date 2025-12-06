use anyhow::{Context, Result, bail};
use authr_core::model::Account;
use authr_core::storage::{load_accounts, save_accounts};
use authr_core::totp;

pub fn list() -> Result<()> {
    let accounts = load_accounts().context("Failed to load accounts")?;
    if accounts.is_empty() {
        println!("No accounts found.");
        return Ok(());
    }
    println!("{:<20}", "Name");
    println!("{}", "-".repeat(20));
    for account in accounts {
        println!("{:<20}", account.name);
    }
    Ok(())
}

pub fn add(name: String, secret: String) -> Result<()> {
    // Validate secret by trying to generate a code (or just decoding)
    // totp-rs validation happens in generate or constructor
    // Simple validation:
    // We can create a dummy account and see if it works.
    let test_account = Account::new(name.clone(), secret.clone());
    if let Err(e) = totp::generate_code(&test_account) {
        bail!("Invalid secret or generation error: {}", e);
    }

    let mut accounts = load_accounts().unwrap_or_default();
    
    // Check for duplicates
    if accounts.iter().any(|a| a.name == name) {
        bail!("Account '{}' already exists", name);
    }

    accounts.push(test_account);
    save_accounts(&accounts).context("Failed to save accounts")?;
    println!("Account '{}' added successfully.", name);
    Ok(())
}

pub fn remove(name: &str) -> Result<()> {
    let mut accounts = load_accounts().context("Failed to load accounts")?;
    let len_before = accounts.len();
    accounts.retain(|a| a.name != name);
    
    if accounts.len() == len_before {
        bail!("Account '{}' not found", name);
    }

    save_accounts(&accounts).context("Failed to save accounts")?;
    println!("Account '{}' removed.", name);
    Ok(())
}

pub fn show(name: &str, show_seed: bool) -> Result<()> {
    let accounts = load_accounts().context("Failed to load accounts")?;
    let account = accounts.iter().find(|a| a.name == name)
        .context(format!("Account '{}' not found", name))?;

    if show_seed {
        println!("Secret: {}", account.secret);
    } else {
        let code = totp::generate_code(account).context("Failed to generate TOTP")?;
        println!("{}", code);
    }
    Ok(())
}
