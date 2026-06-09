use crate::model::Account;
use crate::totp;
use serde::Serialize;
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

/// Sort accounts alphabetically by name, case-insensitively, in place. Applied whenever the
/// store grows (or a name changes) so the persisted JSON — and the popover list — stay
/// alphabetized. Stable, so two names that differ only by case keep their relative order.
pub fn sort_accounts(accounts: &mut [Account]) {
    accounts.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
    // Spaces (and any whitespace) in a pasted secret are ignored — the add-account UI hint
    // promises "spaces ignored", and base32 carries no whitespace, so this is safe to strip.
    let secret: String = secret.chars().filter(|c| !c.is_whitespace()).collect();
    let account = Account::new(name.clone(), secret);
    validate_secret(&account.secret)?;
    if accounts.iter().any(|a| a.name == name) {
        return Err(AccountError::Duplicate(name));
    }
    accounts.push(account.clone());
    sort_accounts(accounts);
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
    sort_accounts(accounts);
    Ok(())
}

/// Remove the account named `name`. Errors with [`AccountError::NotFound`] if absent.
///
/// Permanent and irreversible — no secret is returned to the caller.
pub fn delete_account(accounts: &mut Vec<Account>, name: &str) -> Result<(), AccountError> {
    let idx = accounts
        .iter()
        .position(|a| a.name == name)
        .ok_or_else(|| AccountError::NotFound(name.to_string()))?;
    accounts.remove(idx);
    Ok(())
}

/// Counts reported back from an additive import merge, shown in the
/// one-tap result toast. Serializable because it is the `import_backup` command's return
/// value — but it carries only counts, never a secret.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportSummary {
    /// New accounts added as-is (secret absent locally, name free).
    pub added: usize,
    /// Accounts already present locally (secret matched) — left untouched (idempotent).
    pub skipped: usize,
    /// Added under a de-duplicated label because the name collided with a *different* secret.
    pub relabeled: usize,
}

/// Find a free `Name (imported)` label (then `Name (imported 2)`, …) so a relabeled import
/// never collides with an existing name — keeping the store's name-uniqueness invariant.
fn deduplicated_label(base: &str, existing: &[Account]) -> String {
    let first = format!("{base} (imported)");
    if !existing.iter().any(|a| a.name == first) {
        return first;
    }
    (2..)
        .map(|n| format!("{base} (imported {n})"))
        .find(|candidate| !existing.iter().any(|a| &a.name == candidate))
        .expect("an unbounded counter always finds a free label")
}

/// Additive, idempotent, rename-safe merge of `imported` into `existing`.
///
/// Identity is the **immutable base32 secret**, not the editable name, so the merge is
/// rename-safe and idempotent. Per-account rules:
///   * secret already present locally → **skip** (local name/label wins; never overwrites),
///   * secret absent + name free → **add as-is**,
///   * secret absent + name collides with a *different* secret → **add under a
///     de-duplicated label** (`Name (imported)`).
///
/// Never deletes and never overwrites — this is additive union, not delete-aware sync.
/// The caller persists `existing` afterward (re-encrypting via the session passphrase if the
/// live store is an unlocked vault). Runs entirely in core, so no secret crosses the
/// bridge.
pub fn merge_accounts(existing: &mut Vec<Account>, imported: Vec<Account>) -> ImportSummary {
    let mut summary = ImportSummary::default();
    for incoming in imported {
        if existing.iter().any(|a| a.secret == incoming.secret) {
            summary.skipped += 1;
        } else if existing.iter().any(|a| a.name == incoming.name) {
            let name = deduplicated_label(&incoming.name, existing);
            existing.push(Account { name, ..incoming });
            summary.relabeled += 1;
        } else {
            existing.push(incoming);
            summary.added += 1;
        }
    }
    sort_accounts(existing);
    summary
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

    // Adding out of order leaves the store alphabetized (case-insensitive) by name.
    #[test]
    fn add_account_keeps_store_alphabetized() {
        let mut accounts = Vec::new();
        add_account(&mut accounts, "charlie".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        add_account(&mut accounts, "Alice".to_string(), "GEZDGNBVGY3TQOJQ".to_string()).unwrap();
        add_account(&mut accounts, "bob".to_string(), "JBSWY3DPEHPK3PXP".to_string()).unwrap();
        let names: Vec<_> = accounts.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Alice", "bob", "charlie"]);
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

    // --- merge_accounts ---------------------------------------------------------------------

    // A distinct base32 secret per account so identity-on-secret is exercised.
    const SECRET_A: &str = "JBSWY3DPEHPK3PXP";
    const SECRET_B: &str = "GEZDGNBVGY3TQOJQ";
    const SECRET_C: &str = "KRSXG5BAONSWG4TFOQ======";

    fn acct(name: &str, secret: &str) -> Account {
        Account::new(name.to_string(), secret.to_string())
    }

    // secret absent + name free → added as-is.
    #[test]
    fn merge_adds_new_accounts() {
        let mut existing = vec![acct("alice", SECRET_A)];
        let summary = merge_accounts(&mut existing, vec![acct("bob", SECRET_B)]);
        assert_eq!(summary, ImportSummary { added: 1, skipped: 0, relabeled: 0 });
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[1].name, "bob");
    }

    // An interleaved import lands alphabetized alongside the existing accounts.
    #[test]
    fn merge_result_is_alphabetized() {
        let mut existing = vec![acct("delta", SECRET_A)];
        let summary = merge_accounts(
            &mut existing,
            vec![acct("alpha", SECRET_B), acct("charlie", SECRET_C)],
        );
        assert_eq!(summary, ImportSummary { added: 2, skipped: 0, relabeled: 0 });
        let names: Vec<_> = existing.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "charlie", "delta"]);
    }

    // secret already present (even under a different local name) → skipped, local name wins.
    #[test]
    fn merge_skips_present_secret_and_keeps_local_name() {
        let mut existing = vec![acct("alice-local", SECRET_A)];
        let summary = merge_accounts(&mut existing, vec![acct("alice-other", SECRET_A)]);
        assert_eq!(summary, ImportSummary { added: 0, skipped: 1, relabeled: 0 });
        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].name, "alice-local", "local name/label wins");
    }

    // secret absent + name collides with a *different* secret → relabeled, never overwritten.
    #[test]
    fn merge_relabels_name_collision_with_different_secret() {
        let mut existing = vec![acct("work", SECRET_A)];
        let summary = merge_accounts(&mut existing, vec![acct("work", SECRET_B)]);
        assert_eq!(summary, ImportSummary { added: 0, skipped: 0, relabeled: 1 });
        assert_eq!(existing.len(), 2);
        assert_eq!(existing[0].name, "work");
        assert_eq!(existing[0].secret, SECRET_A, "original untouched");
        assert_eq!(existing[1].name, "work (imported)");
        assert_eq!(existing[1].secret, SECRET_B);
    }

    // A repeated relabel collision falls through to a numbered label.
    #[test]
    fn merge_relabel_avoids_existing_imported_label() {
        let mut existing = vec![acct("work", SECRET_A), acct("work (imported)", SECRET_B)];
        let summary = merge_accounts(&mut existing, vec![acct("work", SECRET_C)]);
        assert_eq!(summary.relabeled, 1);
        // The store is alphabetized on merge, so assert by identity, not position: the third
        // "work" lands under the numbered label, carrying its own secret.
        assert!(existing
            .iter()
            .any(|a| a.name == "work (imported 2)" && a.secret == SECRET_C));
    }

    // Idempotent: re-importing the same file is a pure no-op (everything skipped).
    #[test]
    fn merge_is_idempotent() {
        let imported = vec![acct("alice", SECRET_A), acct("bob", SECRET_B)];
        let mut existing: Vec<Account> = Vec::new();
        let first = merge_accounts(&mut existing, imported.clone());
        assert_eq!(first, ImportSummary { added: 2, skipped: 0, relabeled: 0 });

        let second = merge_accounts(&mut existing, imported);
        assert_eq!(second, ImportSummary { added: 0, skipped: 2, relabeled: 0 });
        assert_eq!(existing.len(), 2, "re-import added nothing");
    }

    // Merge never deletes: a local-only account survives importing a file that lacks it.
    #[test]
    fn merge_never_deletes_local_only_accounts() {
        let mut existing = vec![acct("local-only", SECRET_A)];
        merge_accounts(&mut existing, vec![acct("bob", SECRET_B)]);
        assert!(existing.iter().any(|a| a.name == "local-only"));
    }
}
