use crate::config::OnConflict;
use crate::error::{Error, Result};

use std::io::{self, BufReader, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use skim::prelude::*;
use termion::clear;
use termion::cursor;
use termion::raw::IntoRawMode;

/// Manages terminal screen for interactive prompts.
pub(crate) struct InteractiveScreen;

impl InteractiveScreen {
    /// Create a new session and clear the screen.
    pub fn new() -> Result<Self> {
        // Clear screen
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        io::stdout().flush()?;
        Ok(Self)
    }

    /// Clear and redraw the screen.
    pub fn redraw(&mut self) -> Result<()> {
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        io::stdout().flush()?;
        Ok(())
    }
}

impl Drop for InteractiveScreen {
    fn drop(&mut self) {
        // Clear screen when done
        print!("{}{}", clear::All, cursor::Goto(1, 1));
        let _ = io::stdout().flush();
    }
}

/// Check if stdin and stdout are connected to a terminal.
pub(crate) fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

/// A simple skim item
struct SimpleItem {
    text: String,
}

impl SkimItem for SimpleItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.text)
    }
}

/// Run skim selector with given items (with fuzzy search)
fn run_skim_select(
    screen: &mut InteractiveScreen,
    prompt: &str,
    items: Vec<String>,
) -> Result<Option<String>> {
    run_skim_select_full(screen, prompt, None, items, false)
}

/// Run skim selector without fuzzy search (simple selection)
fn run_skim_simple(
    screen: &mut InteractiveScreen,
    prompt: &str,
    items: Vec<String>,
) -> Result<Option<String>> {
    run_skim_select_full(screen, prompt, None, items, true)
}

/// Run skim selector with header message
fn run_skim_with_header(
    screen: &mut InteractiveScreen,
    prompt: &str,
    header: &str,
    items: Vec<String>,
    disable_search: bool,
) -> Result<Option<String>> {
    run_skim_select_full(screen, prompt, Some(header), items, disable_search)
}

/// Run skim selector with given items (full version with all options)
fn run_skim_select_full(
    screen: &mut InteractiveScreen,
    prompt: &str,
    header: Option<&str>,
    items: Vec<String>,
    disable_search: bool,
) -> Result<Option<String>> {
    // Calculate height: items + 2 lines for prompt/info (+ 1 if header), capped at 20
    let header_lines = if header.is_some() { 1 } else { 0 };
    let height = (items.len() + 2 + header_lines).min(20);

    let mut builder = SkimOptionsBuilder::default();
    builder
        .prompt(prompt.to_string())
        .height(format!("{height}"))
        .reverse(true)
        .multi(false);

    if let Some(h) = header {
        builder.header(Some(h.to_string()));
    }

    if disable_search {
        // Disable fuzzy search and hide item count for fixed choice lists
        builder.exact(true).no_sort(true).no_info(true);
    }

    let options = builder.build().map_err(|e| Error::Selector {
        message: e.to_string(),
    })?;

    let (tx, rx): (SkimItemSender, SkimItemReceiver) = unbounded();

    for item in items {
        let _ = tx.send(Arc::new(SimpleItem { text: item }));
    }
    drop(tx);

    let output = Skim::run_with(&options, Some(rx)).ok_or(Error::Aborted)?;

    // Redraw screen after skim finishes
    screen.redraw()?;

    if output.is_abort {
        return Err(Error::Aborted);
    }

    Ok(output
        .selected_items
        .first()
        .map(|item| item.output().to_string()))
}

/// Simple text input from stdin with Ctrl+C support.
fn read_line(prompt: &str, default: Option<&str>) -> Result<String> {
    let prompt_with_default = if let Some(d) = default {
        format!("{prompt} [{d}]: ")
    } else {
        format!("{prompt}: ")
    };

    print!("{prompt_with_default}");
    io::stdout().flush()?;

    // Use raw mode to detect Ctrl+C
    let mut stdout = io::stdout().into_raw_mode()?;
    let stdin = BufReader::new(io::stdin());
    let mut input = String::new();

    for byte in stdin.bytes() {
        let byte = byte?;
        match byte {
            // Ctrl+C (ETX)
            3 => {
                // Restore terminal and return abort
                drop(stdout);
                println!();
                return Err(Error::Aborted);
            }
            // Enter (CR or LF)
            13 | 10 => {
                // Restore terminal
                drop(stdout);
                println!();
                break;
            }
            // Backspace or Delete
            127 | 8 => {
                if !input.is_empty() {
                    input.pop();
                    // Erase character on screen
                    write!(stdout, "\x08 \x08")?;
                    stdout.flush()?;
                }
            }
            // Ctrl+U - clear line
            21 => {
                for _ in 0..input.len() {
                    write!(stdout, "\x08 \x08")?;
                }
                stdout.flush()?;
                input.clear();
            }
            // Escape
            27 => {
                drop(stdout);
                println!();
                return Err(Error::Aborted);
            }
            // Printable ASCII
            32..=126 => {
                input.push(byte as char);
                write!(stdout, "{}", byte as char)?;
                stdout.flush()?;
            }
            // Ignore other control characters
            _ => {}
        }
    }

    let input = input.trim().to_string();

    if input.is_empty() {
        if let Some(d) = default {
            return Ok(d.to_string());
        }
    }

    if input.is_empty() {
        return Err(Error::Aborted);
    }

    Ok(input)
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

    let mut screen = InteractiveScreen::new()?;

    let choices: Vec<String> = ConflictOption::ALL
        .iter()
        .map(|o| o.label().to_string())
        .collect();

    let prompt = format!("{} already exists in worktree.", target.display());
    let header = "Choose how to proceed:".to_string();

    let selection = run_skim_with_header(&mut screen, &prompt, &header, choices, true)?
        .ok_or(Error::Aborted)?;

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
    screen: &mut InteractiveScreen,
    local_branches: &[String],
    remote_branches: &[String],
) -> Result<BranchChoice> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    // First: ask whether to create new or select existing (no fuzzy search)
    let choices = vec![
        "Create new branch".to_string(),
        "Select existing branch".to_string(),
    ];
    let mode = run_skim_simple(screen, "Branch > ", choices)?.ok_or(Error::Aborted)?;

    if mode == "Create new branch" {
        prompt_new_branch_creation(screen, local_branches, remote_branches)
    } else {
        // Select existing branch with fuzzy search
        if local_branches.is_empty() {
            return Err(Error::Selector {
                message: "No existing branches found".to_string(),
            });
        }

        let selection = run_skim_select(screen, "Select branch > ", local_branches.to_vec())?
            .ok_or(Error::Aborted)?;

        Ok(BranchChoice {
            branch: selection,
            create_new: false,
            base_commitish: None,
        })
    }
}

/// Prompt the user to create a new branch with base selection
fn prompt_new_branch_creation(
    screen: &mut InteractiveScreen,
    local_branches: &[String],
    remote_branches: &[String],
) -> Result<BranchChoice> {
    // Ask for the base of the new branch
    let choices = vec![
        "From local branch".to_string(),
        "From remote branch".to_string(),
        "From commit hash".to_string(),
    ];
    let base_mode = run_skim_simple(screen, "Create from > ", choices)?.ok_or(Error::Aborted)?;

    let base_commitish = if base_mode == "From local branch" {
        if local_branches.is_empty() {
            return Err(Error::Selector {
                message: "No local branches found".to_string(),
            });
        }
        let selection = run_skim_select(screen, "Select base branch > ", local_branches.to_vec())?
            .ok_or(Error::Aborted)?;
        Some(selection)
    } else if base_mode == "From remote branch" {
        if remote_branches.is_empty() {
            return Err(Error::Selector {
                message: "No remote branches found. Run `git fetch` first.".to_string(),
            });
        }
        let selection = run_skim_select(screen, "Select base branch > ", remote_branches.to_vec())?
            .ok_or(Error::Aborted)?;
        Some(selection)
    } else {
        // From commit hash
        let hash = read_line("Commit hash", None)?;
        Some(hash)
    };

    // Finally, ask for the new branch name
    let branch_name = read_line("New branch name", None)?;

    Ok(BranchChoice {
        branch: branch_name,
        create_new: true,
        base_commitish,
    })
}

/// Prompt user for worktree path.
pub(crate) fn prompt_worktree_path(
    _screen: &mut InteractiveScreen,
    suggested: &str,
) -> Result<PathBuf> {
    if !is_interactive() {
        return Err(Error::NonInteractive);
    }

    let path = read_line("Worktree path", Some(suggested))?;
    Ok(PathBuf::from(path))
}
