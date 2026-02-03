use crate::common::{BASIC_CONFIG, JjTestRepo, MINIMAL_CONFIG, jj_available};
use predicates::prelude::*;

#[test]
fn test_jj_basic_workspace_add() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let workspace_path = repo.workspace_path("feature-workspace");

    repo.gwtx()
        .args(["add", workspace_path.to_str().unwrap()])
        .assert()
        .success();

    repo.register_workspace(workspace_path.clone());

    // Verify workspace was created
    assert!(workspace_path.exists());
    assert!(workspace_path.join("README.md").exists());
}

#[test]
fn test_jj_colocated_workspace_add() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config_colocated(MINIMAL_CONFIG);
    let workspace_path = repo.workspace_path("colocated-workspace");

    repo.gwtx()
        .args(["add", workspace_path.to_str().unwrap()])
        .assert()
        .success();

    repo.register_workspace(workspace_path.clone());

    // Verify workspace was created
    assert!(workspace_path.exists());
    assert!(workspace_path.join("README.md").exists());
}

#[test]
fn test_jj_add_with_mkdir_link_copy() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(BASIC_CONFIG);

    // Create source files for link and copy
    repo.create_file_and_snapshot("local.env", "export FOO=bar\n", "Add local.env");
    repo.create_file_and_snapshot(
        "config.template",
        "# Config template\n",
        "Add config template",
    );

    let workspace_path = repo.workspace_path("ws-ops");

    repo.gwtx()
        .args([
            "add",
            workspace_path.to_str().unwrap(),
            "--on-conflict",
            "overwrite",
        ])
        .assert()
        .success();

    repo.register_workspace(workspace_path.clone());

    // Verify mkdir
    assert!(repo.workspace_dir_exists("ws-ops", ".cache"));
    assert!(repo.workspace_dir_exists("ws-ops", "tmp"));

    // Verify link
    assert!(repo.workspace_symlink_exists("ws-ops", "local.env"));

    // Verify copy
    assert!(repo.workspace_file_exists("ws-ops", "config.local"));
    let content = repo.read_workspace_file("ws-ops", "config.local");
    assert_eq!(content, "# Config template\n");
}

#[test]
fn test_jj_add_dry_run() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let repo = JjTestRepo::with_config(BASIC_CONFIG);

    // Create source files
    repo.create_file_and_snapshot("local.env", "export FOO=bar\n", "Add local.env");
    repo.create_file_and_snapshot("config.template", "# Config\n", "Add config");

    let workspace_path = repo.workspace_path("dry-run-test");

    repo.gwtx()
        .args(["add", workspace_path.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));

    // Workspace should not be created
    assert!(!workspace_path.exists());
}

#[test]
fn test_jj_add_no_setup() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(BASIC_CONFIG);

    // Create source files
    repo.create_file_and_snapshot("local.env", "export FOO=bar\n", "Add local.env");
    repo.create_file_and_snapshot("config.template", "# Config\n", "Add config");

    let workspace_path = repo.workspace_path("no-setup-test");

    repo.gwtx()
        .args(["add", workspace_path.to_str().unwrap(), "--no-setup"])
        .assert()
        .success();

    repo.register_workspace(workspace_path.clone());

    // Workspace should exist but operations should not be applied
    assert!(workspace_path.exists());
    assert!(!repo.workspace_dir_exists("no-setup-test", ".cache"));
    assert!(!repo.workspace_symlink_exists("no-setup-test", "local.env"));
    assert!(!repo.workspace_file_exists("no-setup-test", "config.local"));
}

#[test]
fn test_jj_add_with_revision() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let workspace_path = repo.workspace_path("revision-workspace");

    // Create workspace at a specific revision (parent of current)
    // Note: In jj, we use commitish argument to specify revision
    repo.gwtx()
        .args([
            "add",
            workspace_path.to_str().unwrap(),
            "@-", // parent of current change
        ])
        .assert()
        .success();

    repo.register_workspace(workspace_path.clone());
    assert!(workspace_path.exists());
}

#[test]
fn test_jj_add_missing_source_file() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let repo = JjTestRepo::with_config(BASIC_CONFIG);
    // Note: source files (local.env, config.template) are NOT created

    let workspace_path = repo.workspace_path("missing-source");

    repo.gwtx()
        .args(["add", workspace_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("not found").or(predicate::str::contains("does not exist")),
        );
}
