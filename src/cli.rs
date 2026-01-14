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
    gwtx reads .gwtx.yaml from the repository root for setup instructions.
    See https://github.com/h-michael/git-setup-worktree for config format.

COLOR OUTPUT:
    gwtx uses colored output for better readability. Control with:

    --color=always    Always use colors (useful when piping: gwtx list --color=always | less -R)
    --color=never     Never use colors (or use --no-color)
    --color=auto      Auto-detect terminal (default)

    Environment:
    NO_COLOR          When set to non-empty value, disables colors (https://no-color.org/)

    Priority: --color flag > NO_COLOR env > terminal detection

EXAMPLES:
    gwtx add ../feature-branch
        Create worktree and run setup from .gwtx.yaml

    gwtx add -b new-feature ../new-feature
        Create new branch and worktree with setup

    gwtx add --interactive
        Select branch and path interactively

    gwtx add --dry-run ../test
        Preview what would be done without executing

    gwtx add --no-setup ../quick
        Create worktree without running setup

    gwtx remove ../feature-branch
        Remove worktree with safety checks

    gwtx remove --interactive
        Select worktrees to remove interactively

    gwtx remove --dry-run ../test
        Preview what would be removed without executing

    gwtx trust
        Trust hooks in .gwtx.yaml (required for hook execution)

    gwtx untrust --list
        List all trusted repositories")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// When to use colored output (always, auto, never)
    #[arg(
        long,
        global = true,
        value_name = "WHEN",
        default_value = "auto",
        conflicts_with = "no_color"
    )]
    pub color: clap::ColorChoice,

    /// Disable colored output (equivalent to --color=never)
    #[arg(long, global = true)]
    pub no_color: bool,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Add a new worktree with setup
    Add(AddArgs),

    /// Remove worktrees with safety checks
    Remove(RemoveArgs),

    /// List all worktrees
    #[command(visible_alias = "ls")]
    List(ListArgs),

    /// Manage .gwtx.yaml configuration
    Config(ConfigArgs),

    /// Trust hooks in .gwtx.yaml for the current repository
    Trust(TrustArgs),

    /// Revoke trust for hooks in .gwtx.yaml
    Untrust(UntrustArgs),

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
    options:
      on_conflict: backup    # Optional, see CONFLICT MODES below

    mkdir:
      - path: build          # Required, relative to worktree
        description: ...     # Optional

    link:
      - source: .env.local   # Required, relative to repo root (supports glob patterns)
        target: .env.local   # Optional, defaults to source
        on_conflict: skip    # Optional, overrides global
        description: ...     # Optional

      - source: .envrc
        description: ...

      - source: fixtures/*   # Glob pattern support
        skip_tracked: true   # Optional, skip git-tracked files (for glob patterns)
        description: ...

    copy:
      - source: .env.example # Required, relative to repo root
        target: .env         # Optional, defaults to source
        on_conflict: backup  # Optional, overrides global
        description: ...     # Optional

    hooks:
      pre_add:
        - command: echo 'Setting up {{worktree_name}}'

      post_add:
        - command: npm install
          description: Install dependencies  # Optional

      pre_remove:
        - command: echo 'Cleaning up {{worktree_name}}'

      post_remove:
        - command: ./scripts/cleanup.sh
          description: Run cleanup script

CONFLICT MODES:
    abort      Stop immediately when a conflict is found
    skip       Skip the conflicting file and continue
    overwrite  Replace the existing file
    backup     Rename existing file with .bak suffix before creating new one

    Default: prompt interactively (error if non-interactive, use --on-conflict)

GLOB PATTERNS:
    link entries support glob patterns in the source field:
        source: fixtures/*       Match all files in fixtures/
        source: file?.txt        Match single character
        source: file[0-9].txt    Match character ranges

    With skip_tracked: true, only git-ignored files are linked, while
    git-tracked files (like .gitkeep) are skipped. This keeps git status clean.

HOOKS:
    Hooks run custom commands before/after worktree operations.
    Require explicit trust via 'gwtx trust' before execution.

    Platform support: Unix-like systems (Linux, macOS) only.
    Windows users: Use Git Bash/WSL or --no-setup flag.

    Format:
        hooks:
          pre_add:
            - command: ...       # Required: shell command to execute
              description: ...   # Optional: human-readable description

    Execution order (gwtx add):
        1. pre_add (repo_root) → 2. git worktree add →
        3. mkdir/link/copy → 4. post_add (worktree_path)

    Execution order (gwtx remove):
        1. pre_remove (worktree_path) → 2. git worktree remove →
        3. post_remove (repo_root)

    Template variables (automatically shell-escaped):
        {{worktree_path}}    Full path to the worktree
        {{worktree_name}}    Worktree directory name
        {{branch}}           Branch name
        {{repo_root}}        Repository root path")]
pub(crate) struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

/// Config subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum ConfigCommand {
    /// Validate .gwtx.yaml configuration
    Validate,
    /// Generate JSON Schema for configuration
    Schema,
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

    /// Skip .gwtx.yaml setup, run git worktree add only
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

/// Arguments for the `remove` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
INTERACTIVE MODE KEYBINDINGS:
    Navigation    ↑/↓, Ctrl+n/p
    Toggle        Space
    Confirm       Enter
    Cancel        Esc, Ctrl+c

SAFETY CHECKS:
    By default, gwtx remove warns about:
    - Uncommitted changes (modified/staged files)
    - Unpushed commits (commits not on remote)

    Use --force to skip safety checks and force removal.")]
pub(crate) struct RemoveArgs {
    /// Worktree paths to remove (required unless --interactive)
    pub paths: Vec<PathBuf>,

    // --- gwtx Options ---
    /// Interactive mode: select worktrees to remove
    #[arg(short, long, help_heading = "gwtx Options")]
    pub interactive: bool,

    /// Preview actions without executing
    #[arg(long, help_heading = "gwtx Options")]
    pub dry_run: bool,

    // --- git worktree Options ---
    /// Force removal even if worktree is dirty or locked
    #[arg(short, long, help_heading = "git worktree Options")]
    pub force: bool,

    // --- Shared Options ---
    /// Suppress output
    #[arg(short, long, help_heading = "Shared Options")]
    pub quiet: bool,
}

/// Arguments for the `list` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
STATUS SYMBOLS:
    *  Uncommitted changes (modified, deleted, or untracked files)

EXAMPLES:
    gwtx list
        List all worktrees with detailed information (branch, commit, status)

    gwtx list --header
        Show header row with column names

    gwtx list --path-only
        List only worktree paths (useful for scripting)

    gwtx ls -p --header
        Combine options using the short alias")]
pub(crate) struct ListArgs {
    /// Show only worktree paths
    #[arg(short, long)]
    pub path_only: bool,

    /// Show header row
    #[arg(long)]
    pub header: bool,
}

/// Arguments for the `trust` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
SECURITY:
    Hooks allow running custom commands before/after worktree operations.
    For security, hooks require explicit trust before execution.

    Trust is stored in: ~/.local/share/gwtx/trusted/
    Each repository's hooks are identified by a SHA256 hash.

    If hooks are modified in .gwtx.yaml, you must re-trust them.

HOOKS:
    Platform support: Unix-like systems (Linux, macOS) only.
    Windows users: Use Git Bash/WSL or --no-setup flag.

    pre_add      Run before worktree creation (in repo_root)
    post_add     Run after worktree creation (in worktree_path)
    pre_remove   Run before worktree removal (in worktree_path)
    post_remove  Run after worktree removal (in repo_root)

    Execution order (gwtx add):
      1. pre_add → 2. git worktree add → 3. mkdir/link/copy → 4. post_add

    Execution order (gwtx remove):
      1. pre_remove → 2. git worktree remove → 3. post_remove

    Hooks can use template variables (see gwtx config for details):
    {{worktree_path}}, {{worktree_name}}, {{branch}}, {{repo_root}}

EXAMPLES:
    gwtx trust
        Trust hooks in .gwtx.yaml for the current repository

    gwtx trust --show
        Show hooks and trust status without trusting

    gwtx trust /path/to/repo
        Trust hooks for a specific repository")]
pub(crate) struct TrustArgs {
    /// Path to repository (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Show trusted hooks without trusting
    #[arg(long)]
    pub show: bool,
}

/// Arguments for the `untrust` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
EXAMPLES:
    gwtx untrust
        Revoke trust for hooks in the current repository

    gwtx untrust --list
        List all trusted repositories

    gwtx untrust /path/to/repo
        Revoke trust for a specific repository")]
pub(crate) struct UntrustArgs {
    /// Path to repository (defaults to current directory)
    pub path: Option<PathBuf>,

    /// List all trusted repositories
    #[arg(long)]
    pub list: bool,
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
