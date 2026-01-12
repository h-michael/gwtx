use crate::config::OnConflict;
use crate::error::{Error, Result};

use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use inquire::{InquireError, Select, Text};

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

/// Run selector with given items (fuzzy search enabled by default).
fn run_select(prompt: &str, items: Vec<String>) -> Result<String> {
    Select::new(prompt, items)
        .without_help_message()
        .prompt()
        .map_err(convert_inquire_error)
}

/// Run selector with header message.
fn run_select_with_message(prompt: &str, message: &str, items: Vec<String>) -> Result<String> {
    println!("{message}");
    Select::new(prompt, items)
        .without_help_message()
        .prompt()
        .map_err(convert_inquire_error)
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
}

impl std::fmt::Display for WorktreeItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display)
    }
}

/// Prompt user to select worktrees to remove.
pub(crate) fn prompt_worktree_selection(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<Vec<PathBuf>> {
    use inquire::MultiSelect;

    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let items: Vec<WorktreeItem> = worktrees
        .iter()
        .filter(|wt| !wt.is_main)
        .map(|wt| {
            let branch_info = wt
                .branch
                .as_ref()
                .and_then(|b| b.strip_prefix("refs/heads/"))
                .unwrap_or("(detached)");
            let lock_info = if wt.is_locked { " [locked]" } else { "" };
            WorktreeItem {
                display: format!("{} ({}){}", wt.path.display(), branch_info, lock_info),
                path: wt.path.clone(),
            }
        })
        .collect();

    if items.is_empty() {
        return Err(Error::NoWorktreesToRemove);
    }

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
