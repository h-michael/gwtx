use crate::config::OnConflict;
use crate::error::{Error, Result};

use std::borrow::Cow;
use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

// inquire is used for all platforms (Text, Confirm, and Windows fallback)
use inquire::{InquireError, Text};

// Unix: Use skim for fuzzy finder
#[cfg(unix)]
use skim::prelude::*;
#[cfg(unix)]
use std::io::Cursor;
#[cfg(unix)]
use std::sync::Arc;

// Windows: Use inquire Select/MultiSelect as fallback
#[cfg(windows)]
use inquire::{MultiSelect, Select};

/// Check if stdin and stdout are connected to a terminal.
pub(crate) fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// Convert inquire error to our error type.
fn convert_inquire_error(e: InquireError) -> Error {
    match e {
        InquireError::OperationCanceled | InquireError::OperationInterrupted => Error::Aborted,
        _ => Error::Selector {
            message: e.to_string(),
        },
    }
}

/// Clear screen (equivalent to termion's clear::All + cursor::Goto(1, 1)).
#[cfg(unix)]
fn clear_screen() {
    use std::io::Write;
    // ANSI escape sequences:
    // \x1B[2J = clear entire screen
    // \x1B[H = move cursor to home position (1, 1)
    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush().ok();
}

// Unix: skim-based selector (fuzzy search enabled)
#[cfg(unix)]
fn skim_select(prompt: &str, items: Vec<String>) -> Result<String> {
    // Calculate height: item count + 2 (prompt/info lines), capped at 20
    let height = (items.len() + 2).min(20);

    let options = SkimOptionsBuilder::default()
        .prompt(prompt.to_string())
        .height(format!("{}", height))
        .multi(false)
        .reverse(true)
        .build()
        .map_err(|e| Error::Selector {
            message: format!("Failed to build skim options: {}", e),
        })?;

    let input = items.join("\n");
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));

    let output = Skim::run_with(&options, Some(items));

    // Clear screen after selection (same as InteractiveScreen.redraw())
    clear_screen();

    match output {
        Some(out) if out.is_abort => Err(Error::Aborted),
        Some(out) if !out.selected_items.is_empty() => {
            Ok(out.selected_items[0].output().to_string())
        }
        _ => Err(Error::Aborted),
    }
}

// Unix: skim-based selector (fuzzy search disabled for fixed choices)
#[cfg(unix)]
fn skim_select_simple(prompt: &str, items: Vec<String>) -> Result<String> {
    // Calculate height: item count + 2 (prompt/info lines), capped at 20
    let height = (items.len() + 2).min(20);

    let options = SkimOptionsBuilder::default()
        .prompt(prompt.to_string())
        .height(format!("{}", height))
        .multi(false)
        .reverse(true)
        .exact(true) // Disable fuzzy search
        .no_sort(true) // Keep original order
        .build()
        .map_err(|e| Error::Selector {
            message: format!("Failed to build skim options: {}", e),
        })?;

    let input = items.join("\n");
    let item_reader = SkimItemReader::default();
    let items = item_reader.of_bufread(Cursor::new(input));

    let output = Skim::run_with(&options, Some(items));

    // Clear screen after selection (same as InteractiveScreen.redraw())
    clear_screen();

    match output {
        Some(out) if out.is_abort => Err(Error::Aborted),
        Some(out) if !out.selected_items.is_empty() => {
            Ok(out.selected_items[0].output().to_string())
        }
        _ => Err(Error::Aborted),
    }
}

// Windows: inquire Select as fallback
#[cfg(windows)]
fn inquire_select(prompt: &str, items: Vec<String>) -> Result<String> {
    Select::new(prompt, items)
        .without_help_message()
        .prompt()
        .map_err(convert_inquire_error)
}

/// Run selector with given items (fuzzy search enabled by default).
fn run_select(prompt: &str, items: Vec<String>) -> Result<String> {
    #[cfg(unix)]
    {
        // Use simple mode (no fuzzy search) for small fixed lists
        if items.len() <= 5 {
            skim_select_simple(prompt, items)
        } else {
            skim_select(prompt, items)
        }
    }
    #[cfg(windows)]
    {
        inquire_select(prompt, items)
    }
}

/// Run selector with header message.
fn run_select_with_message(prompt: &str, message: &str, items: Vec<String>) -> Result<String> {
    println!("{message}");
    run_select(prompt, items)
}

/// Simple text input with optional default value.
fn read_line(prompt: &str, default: Option<&str>) -> Result<String> {
    let mut text = Text::new(prompt);
    if let Some(d) = default {
        text = text.with_default(d);
    }
    text.prompt().map_err(convert_inquire_error)
}

/// User's conflict resolution choice.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ConflictChoice {
    pub mode: OnConflict,
    pub apply_to_all: bool,
}

/// Conflict resolution option for interactive prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictOption {
    Abort,
    Skip,
    SkipAll,
    Overwrite,
    OverwriteAll,
    Backup,
    BackupAll,
}

impl ConflictOption {
    const ALL: &[Self] = &[
        Self::Abort,
        Self::Skip,
        Self::SkipAll,
        Self::Overwrite,
        Self::OverwriteAll,
        Self::Backup,
        Self::BackupAll,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Abort => "abort - stop the entire operation",
            Self::Skip => "skip - skip this file",
            Self::SkipAll => "skip all - skip all conflicts",
            Self::Overwrite => "overwrite - overwrite this file",
            Self::OverwriteAll => "overwrite all - overwrite all conflicts",
            Self::Backup => "backup - backup existing and overwrite",
            Self::BackupAll => "backup all - backup all conflicts",
        }
    }

    fn from_label(s: &str) -> Option<Self> {
        Self::ALL.iter().find(|opt| opt.label() == s).copied()
    }

    fn to_choice(self) -> ConflictChoice {
        match self {
            Self::Abort => ConflictChoice {
                mode: OnConflict::Abort,
                apply_to_all: false,
            },
            Self::Skip => ConflictChoice {
                mode: OnConflict::Skip,
                apply_to_all: false,
            },
            Self::SkipAll => ConflictChoice {
                mode: OnConflict::Skip,
                apply_to_all: true,
            },
            Self::Overwrite => ConflictChoice {
                mode: OnConflict::Overwrite,
                apply_to_all: false,
            },
            Self::OverwriteAll => ConflictChoice {
                mode: OnConflict::Overwrite,
                apply_to_all: true,
            },
            Self::Backup => ConflictChoice {
                mode: OnConflict::Backup,
                apply_to_all: false,
            },
            Self::BackupAll => ConflictChoice {
                mode: OnConflict::Backup,
                apply_to_all: true,
            },
        }
    }
}

/// Prompt user for conflict resolution with "apply to all" option.
pub(crate) fn prompt_conflict_with_all(target: &Path) -> Result<ConflictChoice> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let choices: Vec<String> = ConflictOption::ALL
        .iter()
        .map(|o| o.label().to_string())
        .collect();

    let message = format!("{} already exists in worktree.", target.display());
    let selection = run_select_with_message("Choose how to proceed:", &message, choices)?;

    let option = ConflictOption::from_label(&selection).unwrap_or(ConflictOption::Abort);

    Ok(option.to_choice())
}

/// User's branch selection result.
#[derive(Debug, Clone)]
pub(crate) struct BranchChoice {
    pub branch: String,
    pub create_new: bool,
    /// Base for new branch (commit hash or branch name).
    pub base_commitish: Option<String>,
}

/// Prompt user to select or create a branch.
pub(crate) fn prompt_branch_selection(
    local_branches: &[String],
    remote_branches: &[String],
) -> Result<BranchChoice> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    // First: ask whether to create new or select existing
    let choices = vec![
        "Create new branch".to_string(),
        "Select existing branch".to_string(),
    ];
    let mode = run_select("Branch:", choices)?;

    if mode == "Create new branch" {
        prompt_new_branch_creation(local_branches, remote_branches)
    } else {
        // Select existing branch with fuzzy search
        if local_branches.is_empty() {
            return Err(Error::Selector {
                message: "No existing branches found".to_string(),
            });
        }

        let selection = run_select("Select branch:", local_branches.to_vec())?;

        Ok(BranchChoice {
            branch: selection,
            create_new: false,
            base_commitish: None,
        })
    }
}

/// Prompt the user to create a new branch with base selection.
fn prompt_new_branch_creation(
    local_branches: &[String],
    remote_branches: &[String],
) -> Result<BranchChoice> {
    // Ask for the base of the new branch
    let choices = vec![
        "From local branch".to_string(),
        "From remote branch".to_string(),
        "From commit hash".to_string(),
    ];
    let base_mode = run_select("Create from:", choices)?;

    let base_commitish = if base_mode == "From local branch" {
        if local_branches.is_empty() {
            return Err(Error::Selector {
                message: "No local branches found".to_string(),
            });
        }
        let selection = run_select("Select base branch:", local_branches.to_vec())?;
        Some(selection)
    } else if base_mode == "From remote branch" {
        if remote_branches.is_empty() {
            return Err(Error::Selector {
                message: "No remote branches found. Run `git fetch` first.".to_string(),
            });
        }
        let selection = run_select("Select base branch:", remote_branches.to_vec())?;
        Some(selection)
    } else {
        // From commit hash
        let hash = read_line("Commit hash:", None)?;
        Some(hash)
    };

    // Finally, ask for the new branch name
    let branch_name = read_line("New branch name:", None)?;

    Ok(BranchChoice {
        branch: branch_name,
        create_new: true,
        base_commitish,
    })
}

/// Prompt user for worktree path.
pub(crate) fn prompt_worktree_path(suggested: &str) -> Result<PathBuf> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let path = read_line("Worktree path:", Some(suggested))?;
    Ok(PathBuf::from(path))
}

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

/// Worktree item for display in selector.
#[derive(Debug, Clone)]
struct WorktreeItem {
    display: String,
    path: PathBuf,
    index: usize,
}

impl std::fmt::Display for WorktreeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

// Unix: Implement SkimItem for WorktreeItem
#[cfg(unix)]
impl SkimItem for WorktreeItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display)
    }

    // IMPORTANT: get_index() is required in skim 0.14.0+
    // Without this, Tab key toggles all items instead of current item
    // See: https://github.com/skim-rs/skim/issues/848
    fn get_index(&self) -> usize {
        self.index
    }
}

/// Build WorktreeItem list from WorktreeInfo (shared logic).
fn build_worktree_items(worktrees: &[crate::git::WorktreeInfo]) -> Result<Vec<WorktreeItem>> {
    let items: Vec<WorktreeItem> = worktrees
        .iter()
        .filter(|wt| !wt.is_main)
        .enumerate()
        .map(|(index, wt)| {
            let branch_info = wt
                .branch
                .as_ref()
                .and_then(|b| b.strip_prefix("refs/heads/"))
                .unwrap_or("(detached)");
            let lock_info = if wt.is_locked { " [locked]" } else { "" };
            WorktreeItem {
                display: format!("{} ({}){}", wt.path.display(), branch_info, lock_info),
                path: wt.path.clone(),
                index,
            }
        })
        .collect();

    if items.is_empty() {
        return Err(Error::NoWorktreesToRemove);
    }

    Ok(items)
}

/// Prompt user to select worktrees to remove (Unix: skim).
#[cfg(unix)]
pub(crate) fn prompt_worktree_selection(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<Vec<PathBuf>> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let items = build_worktree_items(worktrees)?;

    // Calculate height: item count + 2 (prompt/info lines), capped at 20
    let height = (items.len() + 2).min(20);

    let options = SkimOptionsBuilder::default()
        .prompt("Select worktrees to remove: ".to_string())
        .height(format!("{}", height))
        .multi(true)
        .reverse(true)
        .no_multi(false) // Ensure multi-select is enabled
        .build()
        .map_err(|e| Error::Selector {
            message: format!("Failed to build skim options: {}", e),
        })?;

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();
    for item in &items {
        let _ = tx.send(Arc::new(item.clone()));
    }
    drop(tx);

    let output = Skim::run_with(&options, Some(rx));

    // Clear screen after selection (same as InteractiveScreen.redraw())
    clear_screen();

    match output {
        Some(out) if out.is_abort => Err(Error::Aborted),
        Some(out) if out.selected_items.is_empty() => Err(Error::Aborted),
        Some(out) => {
            // Get selected display strings
            let selected_displays: Vec<String> = out
                .selected_items
                .iter()
                .map(|item| item.output().to_string())
                .collect();

            // Match display strings to original items to get paths
            let paths: Vec<PathBuf> = selected_displays
                .iter()
                .filter_map(|display| {
                    items
                        .iter()
                        .find(|item| &item.display == display)
                        .map(|item| item.path.clone())
                })
                .collect();

            Ok(paths)
        }
        None => Err(Error::Aborted),
    }
}

/// Prompt user to select worktrees to remove (Windows: inquire fallback).
#[cfg(windows)]
pub(crate) fn prompt_worktree_selection(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<Vec<PathBuf>> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let items = build_worktree_items(worktrees)?;

    let selected = MultiSelect::new("Select worktrees to remove:", items)
        .with_help_message("Space to toggle, Enter to confirm, Esc to cancel")
        .prompt()
        .map_err(convert_inquire_error)?;

    if selected.is_empty() {
        return Err(Error::Aborted);
    }

    Ok(selected.into_iter().map(|item| item.path).collect())
}

/// Prompt for confirmation when safety warnings exist.
pub(crate) fn prompt_remove_confirmation(warnings: &[SafetyWarning]) -> Result<bool> {
    use inquire::Confirm;

    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    println!("\nWarning: The following worktrees have unsaved work:");
    for warning in warnings {
        println!("\n  {}", warning.path.display());
        if warning.modified_count > 0 {
            println!("    - {} modified file(s)", warning.modified_count);
        }
        if warning.deleted_count > 0 {
            println!("    - {} deleted file(s)", warning.deleted_count);
        }
        if warning.untracked_count > 0 {
            println!("    - {} untracked file(s)", warning.untracked_count);
        }
        if warning.has_unpushed {
            println!("    - {} unpushed commit(s)", warning.unpushed_count);
        }
    }
    println!();

    Confirm::new("Do you want to proceed with removal?")
        .with_default(false)
        .prompt()
        .map_err(convert_inquire_error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::WorktreeInfo;
    use std::path::PathBuf;

    #[test]
    fn test_build_worktree_items_filters_main() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/repo/.git"),
                head: "abc123".to_string(),
                branch: Some("refs/heads/main".to_string()),
                is_main: true,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-1"),
                head: "def456".to_string(),
                branch: Some("refs/heads/feature-1".to_string()),
                is_main: false,
                is_locked: false,
            },
        ];

        let result = build_worktree_items(&worktrees).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, PathBuf::from("/repo/feature-1"));
    }

    #[test]
    fn test_build_worktree_items_assigns_index() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/repo/.git"),
                head: "abc123".to_string(),
                branch: Some("refs/heads/main".to_string()),
                is_main: true,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-1"),
                head: "def456".to_string(),
                branch: Some("refs/heads/feature-1".to_string()),
                is_main: false,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-2"),
                head: "ghi789".to_string(),
                branch: Some("refs/heads/feature-2".to_string()),
                is_main: false,
                is_locked: false,
            },
        ];

        let result = build_worktree_items(&worktrees).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].index, 0);
        assert_eq!(result[1].index, 1);
    }

    #[test]
    fn test_build_worktree_items_formats_display() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/repo/.git"),
                head: "abc123".to_string(),
                branch: Some("refs/heads/main".to_string()),
                is_main: true,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-1"),
                head: "def456".to_string(),
                branch: Some("refs/heads/feature-1".to_string()),
                is_main: false,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-2"),
                head: "ghi789".to_string(),
                branch: None,
                is_main: false,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-3"),
                head: "jkl012".to_string(),
                branch: Some("refs/heads/feature-3".to_string()),
                is_main: false,
                is_locked: true,
            },
        ];

        let result = build_worktree_items(&worktrees).unwrap();

        assert_eq!(result.len(), 3);
        assert!(result[0].display.contains("feature-1"));
        assert!(result[1].display.contains("(detached)"));
        assert!(result[2].display.contains("[locked]"));
    }

    #[test]
    fn test_build_worktree_items_empty() {
        let worktrees = vec![WorktreeInfo {
            path: PathBuf::from("/repo/.git"),
            head: "abc123".to_string(),
            branch: Some("refs/heads/main".to_string()),
            is_main: true,
            is_locked: false,
        }];

        let result = build_worktree_items(&worktrees);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoWorktreesToRemove));
    }

    #[cfg(unix)]
    #[test]
    fn test_worktree_item_get_index() {
        let item = WorktreeItem {
            display: "test".to_string(),
            path: PathBuf::from("/test"),
            index: 42,
        };

        assert_eq!(item.get_index(), 42);
    }
}

/// Prompt user to trust hooks
pub(crate) fn prompt_trust_hooks(repo_root: &Path) -> Result<bool> {
    use inquire::Confirm;

    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    Confirm::new(&format!("Trust these hooks for {}?", repo_root.display()))
        .with_default(false)
        .with_help_message(
            "Once trusted, hooks will run automatically on future `gwtx add/remove` commands",
        )
        .prompt()
        .map_err(convert_inquire_error)
}
