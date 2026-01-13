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

/// Get the repository name (directory name of repo root).
pub(crate) fn repo_name() -> Result<String> {
    let root = repo_root()?;
    root.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or(Error::NotInGitRepo)
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

/// Information about a worktree.
#[derive(Debug, Clone)]
pub(crate) struct WorktreeInfo {
    pub path: PathBuf,
    pub head: String,
    pub branch: Option<String>,
    pub is_main: bool,
    pub is_locked: bool,
}

/// List all worktrees with their information.
pub(crate) fn list_worktrees() -> Result<Vec<WorktreeInfo>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    parse_worktree_list(&output.stdout)
}

fn parse_worktree_list(bytes: &[u8]) -> Result<Vec<WorktreeInfo>> {
    let text = String::from_utf8_lossy(bytes);
    let mut worktrees = Vec::new();
    let mut current: Option<WorktreeInfo> = None;
    let mut is_first = true;

    for line in text.lines() {
        if line.starts_with("worktree ") {
            if let Some(wt) = current.take() {
                worktrees.push(wt);
            }
            // Use empty string as fallback if prefix is missing (shouldn't happen in normal git output)
            let path = PathBuf::from(line.strip_prefix("worktree ").unwrap_or(""));
            current = Some(WorktreeInfo {
                path,
                head: String::new(),
                branch: None,
                is_main: is_first,
                is_locked: false,
            });
            is_first = false;
        } else if line.starts_with("HEAD ") {
            if let Some(ref mut wt) = current {
                wt.head = line.strip_prefix("HEAD ").unwrap_or("").to_string();
            }
        } else if line.starts_with("branch ") {
            if let Some(ref mut wt) = current {
                wt.branch = Some(line.strip_prefix("branch ").unwrap_or("").to_string());
            }
        } else if line == "detached" {
            // HEAD is detached, branch remains None
        } else if line.starts_with("locked") {
            if let Some(ref mut wt) = current {
                wt.is_locked = true;
            }
        }
    }

    if let Some(wt) = current {
        worktrees.push(wt);
    }

    Ok(worktrees)
}

/// Working tree status information.
#[derive(Debug, Clone)]
pub(crate) struct WorktreeStatus {
    pub has_uncommitted_changes: bool,
    pub modified_count: usize,
    pub deleted_count: usize,
    pub untracked_count: usize,
}

/// Get the status of a worktree.
pub(crate) fn worktree_status(worktree_path: &std::path::Path) -> Result<WorktreeStatus> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        return Err(Error::NotInGitRepo);
    }

    parse_status_output(&output.stdout)
}

fn parse_status_output(bytes: &[u8]) -> Result<WorktreeStatus> {
    let text = String::from_utf8_lossy(bytes);
    let mut modified_count = 0;
    let mut deleted_count = 0;
    let mut untracked_count = 0;

    for line in text.lines() {
        if line.len() < 2 {
            continue;
        }
        let index = line.chars().next().unwrap_or(' ');
        let worktree = line.chars().nth(1).unwrap_or(' ');

        if index == '?' && worktree == '?' {
            untracked_count += 1;
        } else if index == 'M' || worktree == 'M' {
            modified_count += 1;
        } else if index == 'D' || worktree == 'D' {
            deleted_count += 1;
        } else if index == 'A' {
            modified_count += 1;
        }
    }

    let has_uncommitted_changes = modified_count > 0 || deleted_count > 0 || untracked_count > 0;

    Ok(WorktreeStatus {
        has_uncommitted_changes,
        modified_count,
        deleted_count,
        untracked_count,
    })
}

/// Unpushed commits information.
#[derive(Debug, Clone)]
pub(crate) struct UnpushedCommits {
    pub has_unpushed: bool,
    pub count: usize,
}

/// Check for unpushed commits in a worktree.
pub(crate) fn worktree_unpushed_commits(
    worktree_path: &std::path::Path,
) -> Result<UnpushedCommits> {
    let upstream = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
        .current_dir(worktree_path)
        .output();

    if upstream.is_err() || !upstream.as_ref().unwrap().status.success() {
        return check_unpushed_against_remote(worktree_path);
    }

    let output = Command::new("git")
        .args(["log", "--oneline", "@{upstream}..HEAD"])
        .current_dir(worktree_path)
        .output()?;

    if !output.status.success() {
        return Ok(UnpushedCommits {
            has_unpushed: false,
            count: 0,
        });
    }

    parse_log_output(&output.stdout)
}

fn check_unpushed_against_remote(worktree_path: &std::path::Path) -> Result<UnpushedCommits> {
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(worktree_path)
        .output()?;

    if !branch_output.status.success() {
        return Ok(UnpushedCommits {
            has_unpushed: false,
            count: 0,
        });
    }

    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    if branch == "HEAD" {
        return Ok(UnpushedCommits {
            has_unpushed: false,
            count: 0,
        });
    }

    let remote_ref = format!("origin/{}", branch);
    let check = Command::new("git")
        .args(["rev-parse", "--verify", &remote_ref])
        .current_dir(worktree_path)
        .output();

    if check.is_err() || !check.unwrap().status.success() {
        // Remote branch doesn't exist - cannot determine unpushed commits
        return Ok(UnpushedCommits {
            has_unpushed: false,
            count: 0,
        });
    }

    let output = Command::new("git")
        .args(["log", "--oneline", &format!("{}..HEAD", remote_ref)])
        .current_dir(worktree_path)
        .output()?;

    parse_log_output(&output.stdout)
}

fn parse_log_output(bytes: &[u8]) -> Result<UnpushedCommits> {
    let lines = parse_output_lines(bytes);
    let count = lines.len();
    Ok(UnpushedCommits {
        has_unpushed: count > 0,
        count,
    })
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

    #[test]
    fn test_parse_worktree_list_single() {
        let output = b"worktree /home/user/repo\nHEAD abc1234\nbranch refs/heads/main\n\n";
        let result = parse_worktree_list(output).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(result[0].head, "abc1234");
        assert_eq!(result[0].branch, Some("refs/heads/main".to_string()));
        assert!(result[0].is_main);
        assert!(!result[0].is_locked);
    }

    #[test]
    fn test_parse_worktree_list_multiple() {
        let output = b"worktree /home/user/repo\nHEAD abc1234\nbranch refs/heads/main\n\nworktree /home/user/feature\nHEAD def5678\nbranch refs/heads/feature\n\n";
        let result = parse_worktree_list(output).unwrap();

        assert_eq!(result.len(), 2);
        assert!(result[0].is_main);
        assert!(!result[1].is_main);
        assert_eq!(result[1].path, PathBuf::from("/home/user/feature"));
        assert_eq!(result[1].head, "def5678");
        assert_eq!(result[1].branch, Some("refs/heads/feature".to_string()));
    }

    #[test]
    fn test_parse_worktree_list_detached() {
        let output = b"worktree /home/user/repo\nHEAD abc1234\nbranch refs/heads/main\n\nworktree /home/user/detached\nHEAD def5678\ndetached\n\n";
        let result = parse_worktree_list(output).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[1].branch, None);
    }

    #[test]
    fn test_parse_worktree_list_locked() {
        let output = b"worktree /home/user/repo\nHEAD abc1234\nbranch refs/heads/main\n\nworktree /home/user/locked\nHEAD def5678\nbranch refs/heads/feature\nlocked\n\n";
        let result = parse_worktree_list(output).unwrap();

        assert_eq!(result.len(), 2);
        assert!(!result[0].is_locked);
        assert!(result[1].is_locked);
    }

    #[test]
    fn test_parse_status_output_empty() {
        let output = b"";
        let result = parse_status_output(output).unwrap();

        assert!(!result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 0);
        assert_eq!(result.deleted_count, 0);
        assert_eq!(result.untracked_count, 0);
    }

    #[test]
    fn test_parse_status_output_modified() {
        let output = b" M file1.txt\nM  file2.txt\n";
        let result = parse_status_output(output).unwrap();

        assert!(result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 2);
        assert_eq!(result.deleted_count, 0);
        assert_eq!(result.untracked_count, 0);
    }

    #[test]
    fn test_parse_status_output_deleted() {
        let output = b" D file1.txt\nD  file2.txt\n";
        let result = parse_status_output(output).unwrap();

        assert!(result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 0);
        assert_eq!(result.deleted_count, 2);
        assert_eq!(result.untracked_count, 0);
    }

    #[test]
    fn test_parse_status_output_untracked() {
        let output = b"?? file1.txt\n?? file2.txt\n";
        let result = parse_status_output(output).unwrap();

        assert!(result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 0);
        assert_eq!(result.deleted_count, 0);
        assert_eq!(result.untracked_count, 2);
    }

    #[test]
    fn test_parse_status_output_added() {
        let output = b"A  file1.txt\n";
        let result = parse_status_output(output).unwrap();

        assert!(result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 1);
        assert_eq!(result.deleted_count, 0);
        assert_eq!(result.untracked_count, 0);
    }

    #[test]
    fn test_parse_status_output_mixed() {
        let output = b" M file1.txt\nD  file2.txt\n?? file3.txt\nA  file4.txt\n";
        let result = parse_status_output(output).unwrap();

        assert!(result.has_uncommitted_changes);
        assert_eq!(result.modified_count, 2);
        assert_eq!(result.deleted_count, 1);
        assert_eq!(result.untracked_count, 1);
    }

    #[test]
    fn test_parse_log_output_empty() {
        let output = b"";
        let result = parse_log_output(output).unwrap();

        assert!(!result.has_unpushed);
        assert_eq!(result.count, 0);
    }

    #[test]
    fn test_parse_log_output_single() {
        let output = b"abc1234 Commit message\n";
        let result = parse_log_output(output).unwrap();

        assert!(result.has_unpushed);
        assert_eq!(result.count, 1);
    }

    #[test]
    fn test_parse_log_output_multiple() {
        let output = b"abc1234 Commit 1\ndef5678 Commit 2\nghi9012 Commit 3\n";
        let result = parse_log_output(output).unwrap();

        assert!(result.has_unpushed);
        assert_eq!(result.count, 3);
    }
}
