use crate::cli::AddArgs;
use crate::error::{Error, Result};

use std::path::{Path, PathBuf};

// ============================================================================
// libgit2 implementation
// ============================================================================

#[cfg(feature = "libgit2")]
use git2::{BranchType, Repository, WorktreeAddOptions};

#[cfg(feature = "libgit2")]
fn open_repo() -> Result<Repository> {
    Repository::discover(".").map_err(|_| Error::NotInGitRepo)
}

#[cfg(feature = "libgit2")]
pub(crate) fn is_inside_repo() -> bool {
    Repository::discover(".").is_ok()
}

#[cfg(feature = "libgit2")]
pub(crate) fn repo_root() -> Result<PathBuf> {
    let repo = open_repo()?;
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or(Error::NotInGitRepo)
}

#[cfg(feature = "libgit2")]
pub(crate) fn list_branches() -> Result<Vec<String>> {
    let repo = open_repo()?;
    let branches = repo.branches(Some(BranchType::Local))?;

    let names: Vec<String> = branches
        .filter_map(|b| b.ok())
        .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
        .collect();

    Ok(names)
}

#[cfg(feature = "libgit2")]
pub(crate) fn list_remote_branches() -> Result<Vec<String>> {
    let repo = open_repo()?;
    let branches = repo.branches(Some(BranchType::Remote))?;

    let names: Vec<String> = branches
        .filter_map(|b| b.ok())
        .filter_map(|(branch, _)| branch.name().ok().flatten().map(String::from))
        .filter(|name| !name.contains("HEAD"))
        .collect();

    Ok(names)
}

#[cfg(feature = "libgit2")]
fn resolve_commitish<'a>(
    repo: &'a Repository,
    commitish: Option<&str>,
) -> Result<git2::Commit<'a>> {
    match commitish {
        Some(spec) => {
            let obj = repo.revparse_single(spec)?;
            obj.peel_to_commit().map_err(Into::into)
        }
        None => {
            let head = repo.head()?;
            head.peel_to_commit().map_err(Into::into)
        }
    }
}

#[cfg(feature = "libgit2")]
pub(crate) fn worktree_add(args: &AddArgs, path: &Path) -> Result<()> {
    let repo = open_repo()?;
    let mut opts = WorktreeAddOptions::new();

    // --lock
    opts.lock(args.lock);

    // Resolve the target commit
    let commit = resolve_commitish(&repo, args.commitish.as_deref())?;

    // Handle branch creation options
    let reference = if let Some(branch_name) = &args.new_branch {
        // -b: Create new branch
        let branch = repo.branch(branch_name, &commit, false)?;
        Some(branch.into_reference())
    } else if let Some(branch_name) = &args.new_branch_force {
        // -B: Force create/reset branch
        let branch = repo.branch(branch_name, &commit, true)?;
        Some(branch.into_reference())
    } else if args.detach {
        // --detach: No branch reference, use commit directly
        None
    } else if let Some(commitish) = &args.commitish {
        // Try to resolve as a branch reference
        if let Ok(reference) = repo.resolve_reference_from_short_name(commitish) {
            if reference.is_branch() {
                opts.checkout_existing(true);
                Some(reference)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        // Default: checkout existing branch with same name as worktree
        opts.checkout_existing(true);
        None
    };

    // Set reference if we have one
    if let Some(ref r) = reference {
        opts.reference(Some(r));
    }

    // Derive worktree name from path
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(Error::NotInGitRepo)?;

    // Create the worktree
    repo.worktree(name, path, Some(&opts))?;

    // Handle --track / --no-track for newly created branches
    if args.track || args.no_track {
        if let Some(branch_name) = args.new_branch.as_ref().or(args.new_branch_force.as_ref()) {
            if let Ok(mut branch) = repo.find_branch(branch_name, BranchType::Local) {
                if args.track {
                    // Try to set upstream based on remote branch
                    if let Some(commitish) = &args.commitish {
                        if commitish.contains('/') {
                            let _ = branch.set_upstream(Some(commitish));
                        }
                    }
                }
                // --no-track: don't set upstream (default behavior)
            }
        }
    }

    // Handle --guess-remote
    if args.guess_remote {
        // Find remote branch with matching name
        if let Some(branch_name) = args.new_branch.as_ref().or(args.new_branch_force.as_ref()) {
            if let Ok(mut branch) = repo.find_branch(branch_name, BranchType::Local) {
                // Look for remote branches with same basename
                if let Ok(remotes) = repo.remotes() {
                    for remote_name in remotes.iter().flatten() {
                        let remote_branch = format!("{}/{}", remote_name, branch_name);
                        if repo.find_branch(&remote_branch, BranchType::Remote).is_ok() {
                            let _ = branch.set_upstream(Some(&remote_branch));
                            break;
                        }
                    }
                }
            }
        }
    }

    // --force, --no-checkout, --no-guess-remote, --quiet are not directly supported by git2
    // These would require additional logic or may not be applicable

    Ok(())
}

#[cfg(feature = "libgit2")]
pub(crate) fn worktree_remove(path: &Path, force: bool) -> Result<()> {
    let repo = open_repo()?;

    // Find worktree by name
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(Error::NotInGitRepo)?;

    match repo.find_worktree(name) {
        Ok(wt) => {
            let mut opts = git2::WorktreePruneOptions::new();
            if force {
                opts.valid(true);
                opts.working_tree(true);
            }
            if let Err(e) = wt.prune(Some(&mut opts)) {
                // Ignore errors during rollback - best effort cleanup
                eprintln!("Warning: Failed to remove worktree: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to find worktree: {}", e);
        }
    }

    Ok(())
}

// ============================================================================
// Command implementation (default)
// ============================================================================

#[cfg(not(feature = "libgit2"))]
use std::process::Command;

#[cfg(not(feature = "libgit2"))]
pub(crate) fn is_inside_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(feature = "libgit2"))]
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

#[cfg(not(feature = "libgit2"))]
fn parse_output_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(String::from)
        .collect()
}

#[cfg(not(feature = "libgit2"))]
pub(crate) fn list_branches() -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch", "--format=%(refname:short)"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    Ok(parse_output_lines(&output.stdout))
}

#[cfg(not(feature = "libgit2"))]
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

#[cfg(not(feature = "libgit2"))]
pub(crate) fn worktree_add(args: &AddArgs, path: &Path) -> Result<()> {
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

#[cfg(not(feature = "libgit2"))]
pub(crate) fn worktree_remove(path: &Path, force: bool) -> Result<()> {
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
