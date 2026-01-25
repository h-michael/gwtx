use crate::common::{MINIMAL_CONFIG, TestRepo};
use predicates::prelude::*;

#[test]
fn test_list_basic() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let wt1_path = repo.worktree_path("list-wt1");
    let wt2_path = repo.worktree_path("list-wt2");

    // Create worktrees
    repo.gwtx()
        .args(["add", wt1_path.to_str().unwrap(), "-b", "list-wt1"])
        .assert()
        .success();
    repo.register_worktree(wt1_path.clone());

    repo.gwtx()
        .args(["add", wt2_path.to_str().unwrap(), "-b", "list-wt2"])
        .assert()
        .success();
    repo.register_worktree(wt2_path.clone());

    // List worktrees
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list-wt1"))
        .stdout(predicate::str::contains("list-wt2"));
}

#[test]
fn test_list_path_only() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let wt_path = repo.worktree_path("path-only-wt");

    // Create a worktree
    repo.gwtx()
        .args(["add", wt_path.to_str().unwrap(), "-b", "path-only-wt"])
        .assert()
        .success();
    repo.register_worktree(wt_path.clone());

    // List with --path-only
    repo.gwtx()
        .args(["list", "--path-only"])
        .assert()
        .success()
        .stdout(predicate::str::contains(wt_path.to_str().unwrap()));
}

#[test]
fn test_list_with_header() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let wt_path = repo.worktree_path("header-wt");

    // Create a worktree
    repo.gwtx()
        .args(["add", wt_path.to_str().unwrap(), "-b", "header-wt"])
        .assert()
        .success();
    repo.register_worktree(wt_path.clone());

    // List with --header
    repo.gwtx()
        .args(["list", "--header"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("PATH")
                .or(predicate::str::contains("BRANCH"))
                .or(predicate::str::contains("COMMIT")),
        );
}

#[test]
fn test_list_alias_ls() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    // ls should work as alias for list
    repo.gwtx().args(["ls"]).assert().success();
}

#[test]
fn test_list_shows_main_worktree() {
    let repo = TestRepo::with_config(MINIMAL_CONFIG);

    // List should show the main worktree
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains(repo.path().to_str().unwrap()));
}

#[test]
fn test_list_shows_detached_head() {
    let mut repo = TestRepo::with_config(MINIMAL_CONFIG);
    let wt_path = repo.worktree_path("detached-wt");

    // Create a detached worktree
    repo.gwtx()
        .args(["add", wt_path.to_str().unwrap(), "--detach"])
        .assert()
        .success();
    repo.register_worktree(wt_path);

    // List should show detached HEAD status
    repo.gwtx()
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("HEAD detached").or(predicate::str::contains("detached")));
}
