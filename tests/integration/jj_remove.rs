use crate::common::{JjTestRepo, MINIMAL_CONFIG, jj_available};
use predicates::prelude::*;

#[test]
fn test_jj_remove_basic() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("remove-ws");

    // Create a workspace
    repo.kabu()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    assert!(ws_path.exists());

    // Remove the workspace (--force needed because jj workspaces have unpushed working-copy commit)
    repo.kabu()
        .args(["remove", ws_path.to_str().unwrap(), "--force"])
        .assert()
        .success();

    // Workspace directory should be removed
    assert!(!ws_path.exists());

    // Clean up registered workspace since it's already removed
    repo.clear_registered_workspaces();

    // Verify workspace is no longer listed
    let workspaces = repo.list_workspaces();
    assert!(!workspaces.contains(&"remove-ws".to_string()));
}

#[test]
fn test_jj_remove_dry_run() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("remove-dry-run-ws");

    // Create a workspace
    repo.kabu()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // Remove with dry-run
    repo.kabu()
        .args(["remove", ws_path.to_str().unwrap(), "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));

    // Workspace should still exist
    assert!(ws_path.exists());

    // Workspace should still be listed
    let workspaces = repo.list_workspaces();
    assert!(workspaces.contains(&"remove-dry-run-ws".to_string()));
}

#[test]
fn test_jj_remove_force() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("remove-force-ws");

    // Create a workspace
    repo.kabu()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // Create uncommitted changes in the workspace
    std::fs::write(ws_path.join("uncommitted.txt"), "uncommitted content\n")
        .expect("Failed to write file");

    // Remove with force should succeed
    repo.kabu()
        .args(["remove", ws_path.to_str().unwrap(), "--force"])
        .assert()
        .success();

    // Workspace directory should be removed
    assert!(!ws_path.exists());
    repo.clear_registered_workspaces();
}

#[test]
fn test_jj_remove_colocated() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config_colocated(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("colocated-remove-ws");

    // Create a workspace in colocated repo
    repo.kabu()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    assert!(ws_path.exists());

    // Remove the workspace (--force needed because jj workspaces have unpushed working-copy commit)
    repo.kabu()
        .args(["remove", ws_path.to_str().unwrap(), "--force"])
        .assert()
        .success();

    // Workspace directory should be removed
    assert!(!ws_path.exists());
    repo.clear_registered_workspaces();
}

#[test]
fn test_jj_remove_quiet() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);
    let ws_path = repo.workspace_path("remove-quiet-ws");

    // Create a workspace
    repo.kabu()
        .args(["add", ws_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws_path.clone());

    // Remove with quiet mode (--force needed because jj workspaces have unpushed working-copy commit)
    repo.kabu()
        .args(["remove", ws_path.to_str().unwrap(), "--quiet", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    assert!(!ws_path.exists());
    repo.clear_registered_workspaces();
}

#[test]
fn test_jj_remove_multiple() {
    if !jj_available() {
        eprintln!("Skipping test: jj not available");
        return;
    }

    let mut repo = JjTestRepo::with_config(MINIMAL_CONFIG);

    let ws1_path = repo.workspace_path("multi-remove-ws1");
    let ws2_path = repo.workspace_path("multi-remove-ws2");

    // Create two workspaces
    repo.kabu()
        .args(["add", ws1_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws1_path.clone());

    repo.kabu()
        .args(["add", ws2_path.to_str().unwrap()])
        .assert()
        .success();
    repo.register_workspace(ws2_path.clone());

    assert!(ws1_path.exists());
    assert!(ws2_path.exists());

    // Remove both workspaces (--force needed because jj workspaces have unpushed working-copy commit)
    repo.kabu()
        .args(["remove", ws1_path.to_str().unwrap(), "--force"])
        .assert()
        .success();

    repo.kabu()
        .args(["remove", ws2_path.to_str().unwrap(), "--force"])
        .assert()
        .success();

    assert!(!ws1_path.exists());
    assert!(!ws2_path.exists());
    repo.clear_registered_workspaces();

    // Only default workspace should remain
    let workspaces = repo.list_workspaces();
    assert_eq!(workspaces.len(), 1);
    assert!(workspaces.contains(&"default".to_string()));
}
