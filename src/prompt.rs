use crate::color::ColorConfig;
use crate::config::OnConflict;
use crate::error::{Error, Result};
use crate::interactive;
use crate::interactive::{STEP_CONFIRM, STEP_SELECT, SelectMode, WorktreeEntry};
use crate::{config, git};

use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Check if stdin is connected to a terminal.
/// Note: We only check stdin because interactive UI writes to /dev/tty or stdout directly.
pub(crate) fn is_interactive() -> bool {
    io::stdin().is_terminal()
}

/// Clear screen (equivalent to termion's clear::All + cursor::Goto(1, 1)).
#[cfg(unix)]
fn clear_screen() -> Result<()> {
    use std::fs::OpenOptions;
    use std::io::Write;

    // Write to /dev/tty instead of stdout to avoid interfering with command output
    // This allows `gwtx path` to work correctly in command substitution
    let mut tty = OpenOptions::new()
        .write(true)
        .open("/dev/tty")
        .map_err(|e| Error::Internal(format!("Failed to open /dev/tty: {}", e)))?;

    // ANSI escape sequences:
    // \x1B[2J = clear entire screen
    // \x1B[H = move cursor to home position (1, 1)
    write!(tty, "\x1B[2J\x1B[H")
        .map_err(|e| Error::Internal(format!("Failed to write to /dev/tty: {}", e)))?;

    tty.flush()
        .map_err(|e| Error::Internal(format!("Failed to flush /dev/tty: {}", e)))?;

    Ok(())
}

/// Clear screen before entering interactive mode.
#[cfg(unix)]
pub(crate) fn clear_screen_interactive() -> Result<()> {
    clear_screen()
}

/// Clear screen before entering interactive mode (no-op on Windows).
#[cfg(windows)]
pub(crate) fn clear_screen_interactive() -> Result<()> {
    Ok(())
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

    fn label(&self) -> &'static str {
        match self {
            Self::Abort => "abort (cancel the entire operation)",
            Self::Skip => "skip (do not touch the existing file)",
            Self::SkipAll => "skip all (skip all future conflicts)",
            Self::Overwrite => "overwrite (deletes the existing file)",
            Self::OverwriteAll => "overwrite all (overwrite all future conflicts)",
            Self::Backup => "backup (renames existing to *.bak)",
            Self::BackupAll => "backup all (backup all future conflicts)",
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

    let message = format!("Conflict: '{}' already exists.", target.display());
    let theme = resolve_ui_theme()?;
    let selection =
        interactive::select_from_list("How should gwtx proceed?", Some(&message), &choices, theme)?;

    let option = ConflictOption::from_label(&selection).unwrap_or(ConflictOption::Abort);

    Ok(option.to_choice())
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

/// Build WorktreeItem list from WorktreeInfo (shared logic).
fn build_worktree_items(worktrees: &[crate::git::WorktreeInfo]) -> Result<Vec<WorktreeItem>> {
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

    Ok(items)
}

/// Build WorktreeItem list for single selection (includes main worktree).
fn build_worktree_items_for_cd(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<Vec<WorktreeItem>> {
    let items: Vec<WorktreeItem> = worktrees
        .iter()
        .map(|wt| {
            let branch_info = wt
                .branch
                .as_ref()
                .and_then(|b| b.strip_prefix("refs/heads/"))
                .unwrap_or("(detached)");
            let main_info = if wt.is_main { " [main]" } else { "" };
            let lock_info = if wt.is_locked { " [locked]" } else { "" };
            WorktreeItem {
                display: format!(
                    "{} ({}){}{}",
                    wt.path.display(),
                    branch_info,
                    main_info,
                    lock_info
                ),
                path: wt.path.clone(),
            }
        })
        .collect();

    if items.is_empty() {
        return Err(Error::NoWorktreesFound);
    }

    Ok(items)
}

pub(crate) fn prompt_worktree_single_selection(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<PathBuf> {
    if !is_interactive() {
        return Err(Error::InteractiveRequired {
            command: "gwtx path",
        });
    }

    let items = build_worktree_items_for_cd(worktrees)?;
    let entries = items
        .into_iter()
        .map(|item| WorktreeEntry {
            display: item.display,
            path: item.path,
        })
        .collect::<Vec<_>>();

    let theme = resolve_ui_theme()?;
    let selected =
        interactive::select_worktrees(&entries, SelectMode::Single, "Path", &[STEP_SELECT], theme)?;
    selected.into_iter().next().ok_or(Error::Aborted)
}

/// Prompt user to select worktrees to remove.
pub(crate) fn prompt_worktree_selection(
    worktrees: &[crate::git::WorktreeInfo],
) -> Result<Vec<PathBuf>> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let items = build_worktree_items(worktrees)?;
    let entries = items
        .into_iter()
        .map(|item| WorktreeEntry {
            display: item.display,
            path: item.path,
        })
        .collect::<Vec<_>>();

    let theme = resolve_ui_theme()?;
    interactive::select_worktrees(&entries, SelectMode::Multi, "Remove", &[STEP_SELECT], theme)
}

fn resolve_ui_theme() -> Result<interactive::UiTheme> {
    let repo_root = git::repository_root()?;
    let config = config::load_merged(&repo_root)?;
    Ok(interactive::UiTheme::from_colors(&config.ui.colors))
}

/// Prompt for confirmation when safety warnings exist.
pub(crate) fn prompt_remove_confirmation(
    warnings: &[SafetyWarning],
    _color: ColorConfig,
) -> Result<bool> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
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
    interactive::confirm(
        "Remove",
        &[STEP_SELECT, STEP_CONFIRM],
        "Do you want to proceed with removal?",
        &details,
        theme,
    )
}

/// Prompt user to trust hooks
pub(crate) fn prompt_trust_hooks(repo_root: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        use std::fs::OpenOptions;
        use std::io::BufRead;

        // Try to open /dev/tty for interactive prompts
        // This works even when stdin is redirected (e.g., in command substitution)
        let mut tty = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty")
            .map_err(|e| {
                // Check raw errno for non-interactive conditions:
                // - ENOENT: /dev/tty doesn't exist
                // - ENXIO: No controlling terminal
                // - ENOTTY: Not a terminal device
                // Other errors (PermissionDenied, etc.): report as Internal for visibility
                match e.raw_os_error() {
                    Some(libc::ENOENT | libc::ENXIO | libc::ENOTTY) => Error::NonInteractive,
                    _ => Error::Internal(format!("Failed to open /dev/tty: {e}")),
                }
            })?;

        writeln!(tty, "Trust these hooks for {}?", repo_root.display())
            .map_err(|e| Error::Internal(format!("Failed to write to /dev/tty: {e}")))?;
        writeln!(
            tty,
            "Once trusted, hooks will run automatically on future `gwtx add/remove` commands"
        )
        .map_err(|e| Error::Internal(format!("Failed to write to /dev/tty: {e}")))?;
        write!(tty, "Proceed? [y/N]: ")
            .map_err(|e| Error::Internal(format!("Failed to write to /dev/tty: {e}")))?;
        tty.flush()
            .map_err(|e| Error::Internal(format!("Failed to flush /dev/tty: {e}")))?;

        let mut input = String::new();
        let mut reader = io::BufReader::new(tty);
        reader
            .read_line(&mut input)
            .map_err(|e| Error::Internal(format!("Failed to read input: {e}")))?;
        let input = input.trim().to_ascii_lowercase();

        Ok(matches!(input.as_str(), "y" | "yes"))
    }
    #[cfg(windows)]
    {
        if !is_interactive() {
            return Err(Error::NonInteractive);
        }

        println!("Trust these hooks for {}?", repo_root.display());
        println!("Once trusted, hooks will run automatically on future `gwtx add/remove` commands");
        print!("Proceed? [y/N]: ");
        io::stdout()
            .flush()
            .map_err(|e| Error::Internal(format!("Failed to flush stdout: {e}")))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| Error::Internal(format!("Failed to read input: {e}")))?;
        let input = input.trim().to_ascii_lowercase();

        Ok(matches!(input.as_str(), "y" | "yes"))
    }
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
    fn test_build_worktree_items_preserves_order() {
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
        assert_eq!(result[0].path, PathBuf::from("/repo/feature-1"));
        assert_eq!(result[1].path, PathBuf::from("/repo/feature-2"));
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

    #[test]
    fn test_build_worktree_items_for_cd_includes_main() {
        let worktrees = vec![WorktreeInfo {
            path: PathBuf::from("/repo/.git"),
            head: "abc123".to_string(),
            branch: Some("refs/heads/main".to_string()),
            is_main: true,
            is_locked: false,
        }];

        let result = build_worktree_items_for_cd(&worktrees).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].display.contains("[main]"));
    }

    #[test]
    fn test_build_worktree_items_for_cd_empty_list() {
        let worktrees: Vec<WorktreeInfo> = vec![];

        let result = build_worktree_items_for_cd(&worktrees);

        assert!(matches!(result.unwrap_err(), Error::NoWorktreesFound));
    }

    #[test]
    fn test_build_worktree_items_for_cd_multiple_worktrees() {
        let worktrees = vec![
            WorktreeInfo {
                path: PathBuf::from("/repo/.git"),
                head: "abc123".to_string(),
                branch: Some("refs/heads/main".to_string()),
                is_main: true,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/feature-branch"),
                head: "def456".to_string(),
                branch: Some("refs/heads/feature".to_string()),
                is_main: false,
                is_locked: false,
            },
            WorktreeInfo {
                path: PathBuf::from("/repo/locked-branch"),
                head: "ghi789".to_string(),
                branch: Some("refs/heads/locked".to_string()),
                is_main: false,
                is_locked: true,
            },
        ];

        let result = build_worktree_items_for_cd(&worktrees).unwrap();

        assert_eq!(result.len(), 3);
        assert!(result[0].display.contains("[main]"));
        assert!(!result[1].display.contains("[main]"));
        assert!(!result[1].display.contains("[locked]"));
        assert!(result[2].display.contains("[locked]"));
    }

    #[test]
    fn test_build_worktree_items_for_cd_detached_head() {
        let worktrees = vec![WorktreeInfo {
            path: PathBuf::from("/repo/detached"),
            head: "abc123".to_string(),
            branch: None,
            is_main: false,
            is_locked: false,
        }];

        let result = build_worktree_items_for_cd(&worktrees).unwrap();

        assert_eq!(result.len(), 1);
        assert!(result[0].display.contains("(detached)"));
    }

    // ConflictOption tests
    #[test]
    fn test_conflict_option_all_count() {
        assert_eq!(ConflictOption::ALL.len(), 7);
    }

    #[test]
    fn test_conflict_option_label_abort() {
        assert_eq!(
            ConflictOption::Abort.label(),
            "abort (cancel the entire operation)"
        );
    }

    #[test]
    fn test_conflict_option_label_skip() {
        assert_eq!(
            ConflictOption::Skip.label(),
            "skip (do not touch the existing file)"
        );
    }

    #[test]
    fn test_conflict_option_label_skip_all() {
        assert_eq!(
            ConflictOption::SkipAll.label(),
            "skip all (skip all future conflicts)"
        );
    }

    #[test]
    fn test_conflict_option_label_overwrite() {
        assert_eq!(
            ConflictOption::Overwrite.label(),
            "overwrite (deletes the existing file)"
        );
    }

    #[test]
    fn test_conflict_option_label_overwrite_all() {
        assert_eq!(
            ConflictOption::OverwriteAll.label(),
            "overwrite all (overwrite all future conflicts)"
        );
    }

    #[test]
    fn test_conflict_option_label_backup() {
        assert_eq!(
            ConflictOption::Backup.label(),
            "backup (renames existing to *.bak)"
        );
    }

    #[test]
    fn test_conflict_option_label_backup_all() {
        assert_eq!(
            ConflictOption::BackupAll.label(),
            "backup all (backup all future conflicts)"
        );
    }

    #[test]
    fn test_conflict_option_from_label_abort() {
        let result = ConflictOption::from_label("abort (cancel the entire operation)");
        assert_eq!(result, Some(ConflictOption::Abort));
    }

    #[test]
    fn test_conflict_option_from_label_skip() {
        let result = ConflictOption::from_label("skip (do not touch the existing file)");
        assert_eq!(result, Some(ConflictOption::Skip));
    }

    #[test]
    fn test_conflict_option_from_label_skip_all() {
        let result = ConflictOption::from_label("skip all (skip all future conflicts)");
        assert_eq!(result, Some(ConflictOption::SkipAll));
    }

    #[test]
    fn test_conflict_option_from_label_overwrite() {
        let result = ConflictOption::from_label("overwrite (deletes the existing file)");
        assert_eq!(result, Some(ConflictOption::Overwrite));
    }

    #[test]
    fn test_conflict_option_from_label_overwrite_all() {
        let result = ConflictOption::from_label("overwrite all (overwrite all future conflicts)");
        assert_eq!(result, Some(ConflictOption::OverwriteAll));
    }

    #[test]
    fn test_conflict_option_from_label_backup() {
        let result = ConflictOption::from_label("backup (renames existing to *.bak)");
        assert_eq!(result, Some(ConflictOption::Backup));
    }

    #[test]
    fn test_conflict_option_from_label_backup_all() {
        let result = ConflictOption::from_label("backup all (backup all future conflicts)");
        assert_eq!(result, Some(ConflictOption::BackupAll));
    }

    #[test]
    fn test_conflict_option_from_label_invalid() {
        let result = ConflictOption::from_label("invalid option");
        assert_eq!(result, None);
    }

    #[test]
    fn test_conflict_option_to_choice_abort() {
        let choice = ConflictOption::Abort.to_choice();
        assert!(matches!(choice.mode, OnConflict::Abort));
        assert!(!choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_skip() {
        let choice = ConflictOption::Skip.to_choice();
        assert!(matches!(choice.mode, OnConflict::Skip));
        assert!(!choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_skip_all() {
        let choice = ConflictOption::SkipAll.to_choice();
        assert!(matches!(choice.mode, OnConflict::Skip));
        assert!(choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_overwrite() {
        let choice = ConflictOption::Overwrite.to_choice();
        assert!(matches!(choice.mode, OnConflict::Overwrite));
        assert!(!choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_overwrite_all() {
        let choice = ConflictOption::OverwriteAll.to_choice();
        assert!(matches!(choice.mode, OnConflict::Overwrite));
        assert!(choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_backup() {
        let choice = ConflictOption::Backup.to_choice();
        assert!(matches!(choice.mode, OnConflict::Backup));
        assert!(!choice.apply_to_all);
    }

    #[test]
    fn test_conflict_option_to_choice_backup_all() {
        let choice = ConflictOption::BackupAll.to_choice();
        assert!(matches!(choice.mode, OnConflict::Backup));
        assert!(choice.apply_to_all);
    }

    // WorktreeItem tests
    #[test]
    fn test_worktree_item_display() {
        let item = WorktreeItem {
            display: "test display".to_string(),
            path: PathBuf::from("/test/path"),
        };
        assert_eq!(format!("{}", item), "test display");
    }

    // SafetyWarning tests
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

    // ConflictChoice tests
    #[test]
    fn test_conflict_choice_debug() {
        let choice = ConflictChoice {
            mode: OnConflict::Skip,
            apply_to_all: true,
        };
        let debug_str = format!("{:?}", choice);
        assert!(debug_str.contains("Skip"));
        assert!(debug_str.contains("true"));
    }

    #[test]
    fn test_conflict_choice_clone() {
        let choice = ConflictChoice {
            mode: OnConflict::Backup,
            apply_to_all: false,
        };
        let cloned = choice;
        assert!(matches!(cloned.mode, OnConflict::Backup));
        assert!(!cloned.apply_to_all);
    }
}
