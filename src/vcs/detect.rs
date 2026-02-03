//! VCS detection logic.
//!
//! Detects which VCS is in use at a given path by checking for
//! `.git` (Git) and `.jj` (jj) directories.

use crate::error::{Error, Result};
use std::path::Path;
use std::process::Command;

/// Type of VCS detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VcsType {
    /// Pure Git repository
    Git,
    /// Pure jj repository (non-colocated)
    Jj,
    /// Colocated jj repository (both .git and .jj exist)
    JjColocated,
}

/// Detect the VCS type in the current directory.
pub(crate) fn detect_vcs() -> Result<VcsType> {
    let current_dir = std::env::current_dir()?;
    detect_vcs_at(&current_dir)
}

/// Detect the VCS type at a specific path.
///
/// Detection order:
/// 1. Check for `.jj/` directory (jj is checked first because it can colocate with git)
/// 2. If `.jj/` exists, check for `.git/` to determine if colocated
/// 3. If only `.git/` exists, it's a pure Git repository
/// 4. Walk up parent directories if not found
pub(crate) fn detect_vcs_at(path: &Path) -> Result<VcsType> {
    // First, try to find VCS root by walking up directories
    if let Some(vcs_type) = find_vcs_root(path) {
        return Ok(vcs_type);
    }

    // If directory traversal didn't find anything, try VCS commands
    // This handles edge cases where .git might be a file (worktree) or
    // the repo structure is non-standard

    // Try jj first (higher priority for colocated repos)
    if is_jj_repo_by_command(path) {
        if is_git_repo_by_command(path) {
            return Ok(VcsType::JjColocated);
        }
        return Ok(VcsType::Jj);
    }

    // Then try git
    if is_git_repo_by_command(path) {
        return Ok(VcsType::Git);
    }

    Err(Error::NotInAnyRepo)
}

/// Walk up the directory tree to find VCS markers.
fn find_vcs_root(start: &Path) -> Option<VcsType> {
    let mut current = start.to_path_buf();

    loop {
        let has_jj = current.join(".jj").is_dir();
        let has_git = current.join(".git").exists(); // .git can be a file (worktree) or dir

        if has_jj {
            if has_git {
                return Some(VcsType::JjColocated);
            }
            return Some(VcsType::Jj);
        }

        if has_git {
            return Some(VcsType::Git);
        }

        // Move to parent directory
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    None
}

/// Check if the path is inside a jj repository using the jj command.
fn is_jj_repo_by_command(path: &Path) -> bool {
    Command::new("jj")
        .args(["root"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if the path is inside a git repository using the git command.
fn is_git_repo_by_command(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    impl VcsType {
        fn is_jj(&self) -> bool {
            matches!(self, VcsType::Jj | VcsType::JjColocated)
        }
    }

    #[test]
    fn test_vcs_type_is_jj() {
        assert!(!VcsType::Git.is_jj());
        assert!(VcsType::Jj.is_jj());
        assert!(VcsType::JjColocated.is_jj());
    }
}

#[cfg(all(test, feature = "impure-test"))]
mod impure_tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detect_git_repo() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        // Initialize git repo
        let init_result = Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output();

        if init_result.is_err() {
            eprintln!("Skipping test: git not available");
            return;
        }

        let result = detect_vcs_at(path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), VcsType::Git);
    }

    #[test]
    fn test_detect_not_in_repo() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path();

        let result = detect_vcs_at(path);
        assert!(result.is_err());
    }
}
