use crate::common::{MINIMAL_CONFIG, TestRepo};
use predicates::prelude::*;

#[test]
fn test_basic_worktree_remove() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let worktree_path = repo.worktree_path("to-remove");

    // First create a worktree
    repo.kabu()
        .args(["add", worktree_path.to_str().unwrap(), "-b", "to-remove"])
        .assert()
        .success();

    repo.register_worktree(worktree_path.clone());
    assert!(worktree_path.exists());

    // Now remove it
    repo.kabu()
        .args(["remove", worktree_path.to_str().unwrap()])
        .assert()
        .success();

    repo.clear_registered_worktrees();
    assert!(!worktree_path.exists());
}

#[test]
fn test_remove_dry_run() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let worktree_path = repo.worktree_path("dry-remove");

    // Create a worktree
    repo.kabu()
        .args(["add", worktree_path.to_str().unwrap(), "-b", "dry-remove"])
        .assert()
        .success();

    repo.register_worktree(worktree_path.clone());

    // Try dry-run remove
    repo.kabu()
        .args(["remove", "--dry-run", worktree_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("[dry-run]"));

    // Worktree should still exist
    assert!(worktree_path.exists());
}

#[test]
fn test_remove_with_uncommitted_changes() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let worktree_path = repo.worktree_path("uncommitted");

    // Create a worktree
    repo.kabu()
        .args(["add", worktree_path.to_str().unwrap(), "-b", "uncommitted"])
        .assert()
        .success();

    repo.register_worktree(worktree_path.clone());

    // Make uncommitted changes
    std::fs::write(worktree_path.join("new-file.txt"), "uncommitted content")
        .expect("Failed to write file");

    // Try to remove - should fail without force
    repo.kabu()
        .args(["remove", worktree_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("uncommitted").or(predicate::str::contains("untracked")));

    // Worktree should still exist
    assert!(worktree_path.exists());
}

#[test]
fn test_remove_force_with_uncommitted_changes() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let worktree_path = repo.worktree_path("force-uncommitted");

    // Create a worktree
    repo.kabu()
        .args([
            "add",
            worktree_path.to_str().unwrap(),
            "-b",
            "force-uncommitted",
        ])
        .assert()
        .success();

    repo.register_worktree(worktree_path.clone());

    // Make uncommitted changes
    std::fs::write(worktree_path.join("new-file.txt"), "uncommitted content")
        .expect("Failed to write file");

    // Force remove
    repo.kabu()
        .args(["remove", "--force", worktree_path.to_str().unwrap()])
        .assert()
        .success();

    repo.clear_registered_worktrees();
    assert!(!worktree_path.exists());
}

#[test]
fn test_cannot_remove_main_worktree() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    // Try to remove the main worktree
    repo.kabu()
        .args(["remove", repo.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("main").or(predicate::str::contains("cannot")));
}

#[test]
fn test_remove_quiet_mode() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let worktree_path = repo.worktree_path("quiet-remove");

    // Create a worktree
    repo.kabu()
        .args(["add", worktree_path.to_str().unwrap(), "-b", "quiet-remove"])
        .assert()
        .success();

    repo.register_worktree(worktree_path.clone());

    // Remove with quiet mode
    repo.kabu()
        .args(["remove", "--quiet", worktree_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    repo.clear_registered_worktrees();
    assert!(!worktree_path.exists());
}

#[test]
fn test_remove_multiple_worktrees() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let wt1_path = repo.worktree_path("multi-1");
    let wt2_path = repo.worktree_path("multi-2");

    // Create two worktrees
    repo.kabu()
        .args(["add", wt1_path.to_str().unwrap(), "-b", "multi-1"])
        .assert()
        .success();
    repo.register_worktree(wt1_path.clone());

    repo.kabu()
        .args(["add", wt2_path.to_str().unwrap(), "-b", "multi-2"])
        .assert()
        .success();
    repo.register_worktree(wt2_path.clone());

    // Remove both
    repo.kabu()
        .args([
            "remove",
            wt1_path.to_str().unwrap(),
            wt2_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    repo.clear_registered_worktrees();
    assert!(!wt1_path.exists());
    assert!(!wt2_path.exists());
}
