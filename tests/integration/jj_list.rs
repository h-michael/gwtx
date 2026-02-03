use crate::common::{JjTestRepo, MINIMAL_CONFIG, jj_available};
use predicates::prelude::*;

#[test]
fn test_jj_list_basic() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws1_path = repo.workspace_path("list-ws1");
    let ws2_path = repo.workspace_path("list-ws2");

    // Create workspaces (name derived from path)
    repo.gwtx()
        .args(["add", ws1_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws1_path.clone());

    repo.gwtx()
        .args(["add", ws2_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws2_path.clone());

    // List workspaces
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list-ws1"))
        .stdout(predicate::str::contains("list-ws2"));
}

#[test]
fn test_jj_list_path_only() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("path-only-ws");

    // Create a workspace
    repo.gwtx()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // List with --path-only
    // Note: macOS may resolve /var to /private/var, so just check the workspace name in the path
    repo.gwtx()
        .args(["list", "--path-only"])
        .assert()
        .success()
        .stdout(predicate::str::contains("path-only-ws"));
}

#[test]
fn test_jj_list_with_header() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("header-ws");

    // Create a workspace
    repo.gwtx()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // List with --header
    repo.gwtx()
        .args(["list", "--header"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("PATH")
                .or(predicate::str::contains("BOOKMARK"))
                .or(predicate::str::contains("COMMIT")),
        );
}

#[test]
fn test_jj_list_alias_ls() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let repo = JjTestRepo::with_config(MINIMAL_CONFIG);

    // ls should work as alias for list
    repo.gwtx().args(["ls"]).assert().success();
}

#[test]
fn test_jj_list_shows_default_workspace() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let repo = JjTestRepo::with_config(MINIMAL_CONFIG);

    // List should show the default workspace (repo path)
    // Note: The repo path contains "repo" directory name
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo"));
}

#[test]
fn test_jj_list_colocated() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config_colocated(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("colocated-list-ws");

    // Create a workspace in colocated repo
    repo.gwtx()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // List should show both default (repo path) and new workspace
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("repo"))
        .stdout(predicate::str::contains("colocated-list-ws"));
}
