use crate::common::{CONFIG_WITH_HOOKS, TestRepo};
use predicates::prelude::*;

#[test]
fn test_trust_with_yes_flag() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Trust with --yes flag
    repo.gwtx()
        .args(["trust", "--yes"])
        .assert()
        .success()
        .stdout(predicate::str::contains("trusted"));
}

#[test]
fn test_trust_check_untrusted() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Check trust status - should fail (exit code 1) when untrusted
    repo.gwtx()
        .args(["trust", "--check"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_trust_check_trusted() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // First trust the config
    repo.trust_config();

    // Now check should succeed
    repo.gwtx().args(["trust", "--check"]).assert().success();
}

#[test]
fn test_trust_show() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Show hooks info
    repo.gwtx()
        .args(["trust", "--show"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pre_add"))
        .stdout(predicate::str::contains("post_add"));
}

#[test]
fn test_untrust() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // First trust
    repo.trust_config();

    // Verify trusted
    repo.gwtx().args(["trust", "--check"]).assert().success();

    // Now untrust
    repo.gwtx().args(["untrust"]).assert().success();

    // Verify untrusted
    repo.gwtx()
        .args(["trust", "--check"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_untrust_list() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Trust first
    repo.trust_config();

    // List trusted repos
    repo.gwtx()
        .args(["untrust", "--list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(repo.path().to_str().unwrap()));
}

#[test]
fn test_config_change_invalidates_trust() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Trust the config
    repo.trust_config();

    // Verify trusted
    repo.gwtx().args(["trust", "--check"]).assert().success();

    // Modify the config
    let new_config = r#"
hooks:
  pre_add:
    - command: echo "modified hook"
      description: Modified pre-add hook
"#;
    std::fs::write(repo.path().join(".gwtx.yaml"), new_config).expect("Failed to write config");

    // Trust should now fail
    repo.gwtx()
        .args(["trust", "--check"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn test_trust_no_hooks_implicitly_trusted() {
    use crate::common::BASIC_CONFIG;

    let repo = TestRepo::with_config(BASIC_CONFIG);

    // Create source files
    repo.create_file_and_commit("local.env", "export FOO=bar\n", "Add local.env");
    repo.create_file_and_commit("config.template", "# Config\n", "Add config");

    // Config without hooks should be implicitly trusted
    repo.gwtx().args(["trust", "--check"]).assert().success();
}

#[test]
fn test_re_trust_unchanged_config() {
    let repo = TestRepo::with_config(CONFIG_WITH_HOOKS);

    // Trust the config
    repo.trust_config();

    // Re-trusting unchanged config should show informational message
    repo.gwtx()
        .args(["trust", "--yes"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("already trusted").or(predicate::str::contains("unchanged")),
        );
}
