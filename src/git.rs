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

/// Parse git command output into lines.
fn parse_output_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(String::from)
        .collect()
}

/// List local branch names.
pub(crate) fn list_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    Ok(parse_output_lines(&output.stdout))
}

/// List remote branch names (e.g., "origin/main").
pub(crate) fn list_remote_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "-r", "--format=%(refname:short)"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    Ok(parse_output_lines(&output.stdout)
        .into_iter()
        .filter(|s| !s.contains("HEAD"))
        .collect())
}

/// Check if current directory is inside a git repository.
pub(crate) fn is_inside_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List all files tracked by git in the repository root.
/// Returns paths relative to the repository root.
pub(crate) fn list_tracked_files() -> Result<Vec<PathBuf>> {
    let output = Command::new("git").args(["ls-files"]).output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    Ok(parse_output_lines(&output.stdout)
        .into_iter()
        .map(PathBuf::from)
        .collect())
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
