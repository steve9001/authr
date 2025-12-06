use assert_cmd::Command;
use tempfile::TempDir;

// Helper to create temp dir
fn setup() -> TempDir {
    TempDir::new().unwrap()
}

// Helper to configure command with temp env
fn authr_cmd(temp_dir: &TempDir) -> Command {
    let mut cmd = Command::cargo_bin("authr_cli").unwrap();
    cmd.env("HOME", temp_dir.path())
       .env("XDG_CONFIG_HOME", temp_dir.path().join(".config"));
    cmd
}

#[test]
fn test_list_empty() {
    let temp = setup();
    authr_cmd(&temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No accounts found"));
}

#[test]
fn test_add_and_list() {
    let temp = setup();
    authr_cmd(&temp)
        .arg("add")
        .arg("testuser")
        .arg("JBSWY3DPEHPK3PXP")
        .assert()
        .success()
        .stdout(predicates::str::contains("Account 'testuser' added"));

    authr_cmd(&temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("testuser"));
}

#[test]
fn test_remove() {
    let temp = setup();
    // Add first
    authr_cmd(&temp)
        .args(&["add", "toremove", "JBSWY3DPEHPK3PXP"])
        .assert()
        .success();

    // Remove
    authr_cmd(&temp)
        .args(&["remove", "toremove"])
        .assert()
        .success()
        .stdout(predicates::str::contains("removed"));

    // Verify gone
    authr_cmd(&temp)
        .arg("list")
        .assert()
        .success()
        .stdout(predicates::str::contains("No accounts found"));
}

#[test]
fn test_show() {
    let temp = setup();
    authr_cmd(&temp)
        .args(&["add", "myservice", "JBSWY3DPEHPK3PXP"])
        .assert()
        .success();

    authr_cmd(&temp)
        .args(&["show", "myservice"])
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"\d{6}").unwrap());
}
