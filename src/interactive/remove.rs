use crate::error::{Error, Result};
use crate::git::WorktreeInfo;
use crate::prompt;

use std::path::PathBuf;

use super::resolve_ui_theme;
use super::select::confirm;
use super::worktree_list::{SelectMode, build_worktree_entries, select_worktrees};
use super::{STEP_CONFIRM, STEP_SELECT_WORKTREE};

/// Safety warning information for a worktree.
#[derive(Debug, Clone)]
pub(crate) struct SafetyWarning {
    pub path: PathBuf,
    pub has_uncommitted: bool,
    pub modified_count: usize,
    pub deleted_count: usize,
    pub untracked_count: usize,
    pub has_unpushed: bool,
    pub unpushed_count: usize,
}

pub(crate) fn run_remove_selection(worktrees: &[WorktreeInfo]) -> Result<Vec<PathBuf>> {
    if !prompt::is_interactive() {
        return Err(Error::InteractiveRequired {
            command: "gwtx remove -i",
        });
    }

    let current_dir = std::env::current_dir().ok();
    let entries = build_worktree_entries(worktrees, false, current_dir.as_deref());
    if entries.is_empty() {
        return Err(Error::NoWorktreesToRemove);
    }

    let theme = resolve_ui_theme()?;
    select_worktrees(
        &entries,
        SelectMode::Multi,
        "Remove",
        &[STEP_SELECT_WORKTREE],
        theme,
    )
}

pub(crate) fn run_remove_confirmation(warnings: &[SafetyWarning]) -> Result<bool> {
    if !prompt::is_interactive() {
        return Err(Error::InteractiveRequired {
            command: "gwtx remove -i",
        });
    }
    let mut details = Vec::new();
    details.push("Warning: The following worktrees have unsaved work:".to_string());
    for warning in warnings {
        details.push(format!("  {}", warning.path.display()));
        if warning.modified_count > 0 {
            details.push(format!("    - {} modified file(s)", warning.modified_count));
        }
        if warning.deleted_count > 0 {
            details.push(format!("    - {} deleted file(s)", warning.deleted_count));
        }
        if warning.untracked_count > 0 {
            details.push(format!(
                "    - {} untracked file(s)",
                warning.untracked_count
            ));
        }
        if warning.has_unpushed {
            details.push(format!(
                "    - {} unpushed commit(s)",
                warning.unpushed_count
            ));
        }
        details.push(String::new());
    }
    while matches!(details.last(), Some(line) if line.is_empty()) {
        details.pop();
    }

    let theme = resolve_ui_theme()?;
    confirm(
        "Remove",
        &[STEP_SELECT_WORKTREE, STEP_CONFIRM],
        "Do you want to proceed with removal?",
        &details,
        theme,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safety_warning_creation() {
        let warning = SafetyWarning {
            path: PathBuf::from("/test/worktree"),
            has_uncommitted: true,
            modified_count: 3,
            deleted_count: 1,
            untracked_count: 2,
            has_unpushed: true,
            unpushed_count: 5,
        };

        assert_eq!(warning.path, PathBuf::from("/test/worktree"));
        assert!(warning.has_uncommitted);
        assert_eq!(warning.modified_count, 3);
        assert_eq!(warning.deleted_count, 1);
        assert_eq!(warning.untracked_count, 2);
        assert!(warning.has_unpushed);
        assert_eq!(warning.unpushed_count, 5);
    }

    #[test]
    fn test_safety_warning_clean() {
        let warning = SafetyWarning {
            path: PathBuf::from("/test/clean"),
            has_uncommitted: false,
            modified_count: 0,
            deleted_count: 0,
            untracked_count: 0,
            has_unpushed: false,
            unpushed_count: 0,
        };

        assert!(!warning.has_uncommitted);
        assert_eq!(warning.modified_count, 0);
        assert!(!warning.has_unpushed);
    }
}
