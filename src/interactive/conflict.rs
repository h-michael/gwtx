use crate::config::OnConflict;
use crate::error::{Error, Result};
use crate::prompt;

use std::path::Path;

use super::STEP_CONFLICT;
use super::resolve_ui_theme;
use super::select::select_from_list;

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
pub(crate) fn prompt_conflict(target: &Path) -> Result<ConflictChoice> {
    if !prompt::is_interactive() {
        return Err(Error::NonInteractive);
    }

    let choices: Vec<String> = ConflictOption::ALL
        .iter()
        .map(|o| o.label().to_string())
        .collect();

    let message = format!("Conflict: '{}' already exists.", target.display());
    let theme = resolve_ui_theme()?;
    let selection = select_from_list("Add", &[STEP_CONFLICT], Some(&message), &choices, theme)?;

    let option = ConflictOption::from_label(&selection).unwrap_or(ConflictOption::Abort);

    Ok(option.to_choice())
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_conflict_choice_copy() {
        let choice = ConflictChoice {
            mode: OnConflict::Backup,
            apply_to_all: false,
        };
        let copied = choice;
        assert!(matches!(copied.mode, OnConflict::Backup));
        assert!(!copied.apply_to_all);
    }
}
