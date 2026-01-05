use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

/// CLI arguments.
#[derive(Parser, Debug)]
#[command(name = "gwtx")]
#[command(about = "git worktree extra - enhance git worktree with automated setup")]
#[command(version)]
#[command(after_help = "\
CONFIGURATION:
    gwtx reads .gwtx.toml from the repository root for setup instructions.
    See https://github.com/h-michael/git-setup-worktree for config format.

EXAMPLES:
    gwtx add ../feature-branch
        Create worktree and run setup from .gwtx.toml

    gwtx add -b new-feature ../new-feature
        Create new branch and worktree with setup

    gwtx add --interactive
        Select branch and path interactively

    gwtx add --dry-run ../test
        Preview what would be done without executing

    gwtx add --no-setup ../quick
        Create worktree without running setup")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Add a new worktree with setup
    Add(AddArgs),

    /// Manage .gwtx.toml configuration
    Config(ConfigArgs),

    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },

    /// Generate man page
    Man,
}

/// Arguments for the `config` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
CONFIG FORMAT:
    [options]
    on_conflict = \"backup\"    # Optional, see CONFLICT MODES below

    [[mkdir]]
    path = \"tmp/cache\"        # Required, relative to worktree
    description = \"...\"       # Optional

    [[link]]
    source = \".env.local\"     # Required, relative to repo root
    target = \".env.local\"     # Optional, defaults to source
    on_conflict = \"skip\"      # Optional, overrides global
    description = \"...\"       # Optional

    [[copy]]
    source = \".env.example\"   # Required, relative to repo root
    target = \".env\"           # Optional, defaults to source
    on_conflict = \"backup\"    # Optional, overrides global
    description = \"...\"       # Optional

CONFLICT MODES:
    abort      Stop immediately when a conflict is found
    skip       Skip the conflicting file and continue
    overwrite  Replace the existing file
    backup     Rename existing file with .bak suffix before creating new one

    Default: prompt interactively (error if non-interactive, use --on-conflict)")]
pub(crate) struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

/// Config subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum ConfigCommand {
    /// Validate .gwtx.toml configuration
    Validate,
}

/// Arguments for the `add` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
INTERACTIVE MODE KEYBINDINGS:
    Navigation    ↑/↓, Ctrl+n/p
    Select        Enter, Ctrl+j
    Cancel        Esc, Ctrl+c

CONFLICT MODES:
    abort      Stop immediately when a conflict is found (default in non-interactive)
    skip       Skip the conflicting file and continue
    overwrite  Replace the existing file
    backup     Rename existing file with .bak suffix before creating new one")]
pub(crate) struct AddArgs {
    /// Path for the new worktree (required unless --interactive)
    pub path: Option<PathBuf>,

    /// Branch or commit to checkout
    pub commitish: Option<String>,

    // --- gwtx Options ---
    /// Interactive mode: select branch and path interactively
    #[arg(short, long, help_heading = "gwtx Options")]
    pub interactive: bool,

    /// How to handle conflicts: abort, skip, overwrite, backup
    #[arg(long, value_name = "MODE", help_heading = "gwtx Options")]
    pub on_conflict: Option<OnConflictArg>,

    /// Preview actions without executing
    #[arg(long, help_heading = "gwtx Options")]
    pub dry_run: bool,

    /// Skip .gwtx.toml setup, run git worktree add only
    #[arg(long, help_heading = "gwtx Options")]
    pub no_setup: bool,

    // --- git worktree Options ---
    /// Create a new branch <name> starting at <commitish>
    #[arg(
        short = 'b',
        value_name = "name",
        help_heading = "git worktree Options"
    )]
    pub new_branch: Option<String>,

    /// Create or reset branch <name> to <commitish>
    #[arg(
        short = 'B',
        value_name = "name",
        help_heading = "git worktree Options"
    )]
    pub new_branch_force: Option<String>,

    /// Force creation even if branch is checked out elsewhere
    #[arg(short, long, help_heading = "git worktree Options")]
    pub force: bool,

    /// Detach HEAD in the new worktree
    #[arg(short, long, help_heading = "git worktree Options")]
    pub detach: bool,

    /// Do not checkout after creation
    #[arg(long, help_heading = "git worktree Options")]
    pub no_checkout: bool,

    /// Lock the worktree after creation
    #[arg(long, help_heading = "git worktree Options")]
    pub lock: bool,

    /// Set up tracking for the branch
    #[arg(long, help_heading = "git worktree Options")]
    pub track: bool,

    /// Do not set up tracking
    #[arg(long, help_heading = "git worktree Options")]
    pub no_track: bool,

    /// Guess remote for tracking
    #[arg(long, help_heading = "git worktree Options")]
    pub guess_remote: bool,

    /// Do not guess remote
    #[arg(long, help_heading = "git worktree Options")]
    pub no_guess_remote: bool,

    // --- Shared Options ---
    /// Suppress output from both gwtx and git worktree
    #[arg(short, long, help_heading = "Shared Options")]
    pub quiet: bool,
}

/// Conflict resolution mode from CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OnConflictArg {
    Abort,
    Skip,
    Overwrite,
    Backup,
}

impl std::str::FromStr for OnConflictArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "abort" => Ok(Self::Abort),
            "skip" => Ok(Self::Skip),
            "overwrite" => Ok(Self::Overwrite),
            "backup" => Ok(Self::Backup),
            _ => Err(format!(
                "Invalid on-conflict mode: {s}. Valid values: abort, skip, overwrite, backup"
            )),
        }
    }
}

/// Parse CLI arguments.
pub(crate) fn parse() -> Cli {
    Cli::parse()
}

/// Build CLI for completion/man generation.
pub(crate) fn build() -> clap::Command {
    Cli::command()
}
