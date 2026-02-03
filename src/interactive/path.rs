use crate::error::{Error, Result};
use crate::prompt;
use crate::vcs::WorkspaceInfo;

use std::path::PathBuf;

use super::STEP_SELECT_WORKTREE;
use super::resolve_ui_theme;
use super::worktree_list::{SelectMode, build_worktree_entries, select_worktrees};

pub(crate) fn run_path_interactive(workspaces: &[WorkspaceInfo]) -> Result<PathBuf> {
    if !prompt::is_interactive() {
        return Err(Error::InteractiveRequired {
            command: "kabu path",
        });
    }

    let entries = build_worktree_entries(workspaces, true, None);
    if entries.is_empty() {
        return Err(Error::NoWorktreesFound);
    }

    let theme = resolve_ui_theme()?;
    let selected = select_worktrees(
        &entries,
        SelectMode::Single,
        "Path",
        &[STEP_SELECT_WORKTREE],
        theme,
    )?;
    selected.into_iter().next().ok_or(Error::Aborted)
}
