use crate::cli::AddArgs;
use crate::error::{Error, Result};

use std::path::PathBuf;
use std::process::Command;

/// Run `git worktree add` with CLI arguments.
pub(crate) fn worktree_add(args: &AddArgs, path: &std::path::Path) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.arg("worktree").arg("add");

    if args.force {
        cmd.arg("--force");
    }
    if args.detach {
        cmd.arg("--detach");
    }
    if let Some(branch) = &args.new_branch {
        cmd.arg("-b").arg(branch);
    }
    if let Some(branch) = &args.new_branch_force {
        cmd.arg("-B").arg(branch);
    }
    if args.no_checkout {
        cmd.arg("--no-checkout");
    }
    if args.lock {
        cmd.arg("--lock");
    }
    if args.track {
        cmd.arg("--track");
    }
    if args.no_track {
        cmd.arg("--no-track");
    }
    if args.guess_remote {
        cmd.arg("--guess-remote");
    }
    if args.no_guess_remote {
        cmd.arg("--no-guess-remote");
    }
    if args.quiet {
        cmd.arg("--quiet");
    }

    cmd.arg(path);

    if let Some(commitish) = &args.commitish {
        cmd.arg(commitish);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(Error::GitWorktreeAddFailed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Get the repository root directory.
pub(crate) fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(PathBuf::from(path))
}

/// Remove a worktree. Used for rollback on failure.
pub(crate) fn worktree_remove(path: &std::path::Path, force: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.args(["worktree", "remove"]);

    if force {
        cmd.arg("--force");
    }

    cmd.arg(path);

    let output = cmd.output()?;

    if !output.status.success() {
        // Ignore errors during rollback - best effort cleanup
        eprintln!(
            "Warning: Failed to remove worktree: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    Ok(())
}

/// List local branch names.
pub(crate) fn list_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    let branches = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();

    Ok(branches)
}

/// List remote branch names (e.g., "origin/main").
pub(crate) fn list_remote_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "-r", "--format=%(refname:short)"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    let branches = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|s| !s.contains("HEAD"))
        .map(|s| s.to_string())
        .collect();

    Ok(branches)
}

/// Check if current directory is inside a git repository.
pub(crate) fn is_inside_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_inside_repo() {
        // This test depends on the environment
        // Just verify it doesn't panic
        let _ = is_inside_repo();
    }
}
