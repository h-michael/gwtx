use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

/// CLI arguments.
#[derive(Parser, Debug)]
#[command(name = "gwtx")]
#[command(
    about = "git/jj worktree extra - enhance git worktree and jj workspace with automated setup"
)]
#[command(version = VERSION_STRING)]
#[command(after_help = "\
VCS SUPPORT:
    gwtx automatically detects and supports:
    - Git repositories (uses git worktree commands)
    - jj (Jujutsu) repositories (uses jj workspace commands)
    - Colocated repositories (jj on top of git)

CONFIGURATION:
    gwtx reads .gwtx/config.yaml from the repository root for setup instructions.

COLOR OUTPUT:
    gwtx uses colored output for better readability. Control with:

    --color=always    Always use colors (useful when piping: gwtx list --color=always | less -R)
    --color=never     Never use colors (or use --no-color)
    --color=auto      Auto-detect terminal (default)

    Environment:
    NO_COLOR          When set to non-empty value, disables colors (https://no-color.org/)

    Priority: --color flag > NO_COLOR env > terminal detection

EXAMPLES:
    gwtx add ../new-worktree-path
        Create worktree and run setup from .gwtx/config.yaml

    gwtx add -b new-branch-name ../new-worktree-path
        Create new branch and worktree with setup

    gwtx add --i
        Select branch and path interactively

    gwtx add --dry-run ../test
        Preview what would be done without executing

    gwtx add --no-setup ../quick
        Create worktree without running setup

    gwtx remove ../worktree-path
        Remove worktree with safety checks

    gwtx remove --i
        Select worktrees to remove interactively

    gwtx remove --dry-run ../test
        Preview what would be removed without executing

    gwtx trust
        Trust hooks in .gwtx/config.yaml (required for hook execution)

    gwtx untrust --list
        List all trusted repositories

    gwtx cd
        Select a worktree and cd to it (requires shell integration)

    cd \"$(gwtx path)\"
        Select a worktree and print its path (works without shell integration)

    eval \"$(gwtx init bash)\"
        Enable shell completions and trust warnings")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

const VERSION_STRING: &str = env!("GWTX_VERSION_LABEL");

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    /// Add a new worktree/workspace with setup
    Add(AddArgs),

    /// Remove worktrees/workspaces with safety checks
    #[command(visible_alias = "rm")]
    Remove(RemoveArgs),

    /// List all worktrees/workspaces
    #[command(visible_alias = "ls")]
    List(ListArgs),

    /// Select a worktree/workspace and print its path
    Path(PathArgs),

    /// Change directory to a selected worktree/workspace (requires shell integration)
    Cd,

    /// Manage .gwtx/config.yaml configuration
    Config(ConfigArgs),

    /// Trust hooks in .gwtx/config.yaml for the current repository
    Trust(TrustArgs),

    /// Revoke trust for hooks in .gwtx/config.yaml
    Untrust(UntrustArgs),

    /// Generate shell completion script
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },

    /// Print shell init script (completions + trust warning hook)
    Init(InitArgs),

    /// Generate man page
    Man,
}

/// Arguments for the `config` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
CONFIG FORMAT:
    defaults:
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
        ignore_tracked: true # Optional, skip git-tracked files (for glob patterns)
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

    With ignore_tracked: true, only git-ignored files are linked, while
    git-tracked files (like .gitkeep) are skipped. This keeps git status clean.

HOOKS:
    Hooks run custom commands before/after worktree operations.
    Require explicit trust via 'gwtx trust' before execution.

    Platform support: Unix-like systems (Linux, macOS) and Windows.
    On Windows, hooks run via an auto-detected shell (pwsh, powershell,
    Git Bash, or cmd). Override with --hook-shell or GWTHOOK_SHELL.

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
    /// Validate .gwtx/config.yaml configuration
    Validate,
    /// Generate JSON Schema for configuration
    Schema,
    /// Create a new configuration file
    New {
        /// Create global config at:
        ///   - $XDG_CONFIG_HOME/gwtx/config.yaml (if XDG_CONFIG_HOME is set)
        ///   - ~/.config/gwtx/config.yaml (Linux)
        ///   - ~/Library/Application Support/gwtx/config.yaml (macOS)
        ///   - %APPDATA%\gwtx\config.yaml (Windows)
        #[arg(short, long, verbatim_doc_comment)]
        global: bool,

        /// Custom path for the config file
        #[arg(short, long)]
        path: Option<std::path::PathBuf>,

        /// Overwrite existing config file
        #[arg(short = 'O', long = "override")]
        override_existing: bool,

        /// Create .gwtx/.gitignore to exclude config from git
        #[arg(long)]
        with_gitignore: bool,

        /// Do not create .gwtx/.gitignore (skip prompt)
        #[arg(long, conflicts_with = "with_gitignore")]
        without_gitignore: bool,
    },
    /// Get a configuration value
    Get {
        /// Configuration key (e.g., auto_cd.after_remove)
        key: String,
    },
}

/// Arguments for the `add` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
VCS SUPPORT:
    Works with both git worktree and jj workspace:
    - Git: Creates worktree using `git worktree add`
    - jj:  Creates workspace using `jj workspace add`

EXAMPLES:
    gwtx add ../new-worktree-path
        Create worktree/workspace and run setup from .gwtx/config.yaml

    gwtx add -b new-branch-name ../new-worktree-path
        Create new branch (git) or bookmark (jj) with worktree/workspace

    gwtx add --interactive
        Select branch and path interactively

    gwtx add --dry-run ../test
        Preview what would be done without executing

    gwtx add --no-setup ../quick
        Create worktree/workspace without running setup

CONFLICT MODES:
    abort      Stop immediately when a conflict is found (default in non-interactive)
    skip       Skip the conflicting file and continue
    overwrite  Replace the existing file
    backup     Rename existing file with .bak suffix before creating new one

ENVIRONMENT VARIABLES:
    GWTX_ON_CONFLICT    Default conflict resolution mode (e.g., GWTX_ON_CONFLICT=backup)
    GWTHOOK_SHELL       Windows-only hook shell override (pwsh, powershell, bash, cmd, wsl)")]
pub(crate) struct AddArgs {
    /// Path for the new worktree/workspace (required unless --interactive)
    pub path: Option<PathBuf>,

    /// Branch or commit to checkout (git) / revision (jj)
    pub commitish: Option<String>,

    // --- gwtx Options ---
    /// Interactive mode: select branch and path interactively
    #[arg(short, long, help_heading = "gwtx Options")]
    pub interactive: bool,

    /// How to handle conflicts: abort, skip, overwrite, backup
    #[arg(
        long,
        value_name = "MODE",
        help_heading = "gwtx Options",
        env = "GWTX_ON_CONFLICT"
    )]
    pub on_conflict: Option<OnConflictArg>,

    /// Preview actions without executing
    #[arg(long, help_heading = "gwtx Options")]
    pub dry_run: bool,

    /// Skip .gwtx/config.yaml setup, run worktree/workspace add only
    #[arg(long, help_heading = "gwtx Options")]
    pub no_setup: bool,

    /// Windows-only: select hook shell (pwsh, powershell, bash, cmd, wsl)
    #[cfg(windows)]
    #[arg(
        long,
        value_name = "SHELL",
        help_heading = "gwtx Options",
        value_parser = [
            "pwsh",
            "powershell",
            "bash",
            "git-bash",
            "gitbash",
            "cmd",
            "cmd.exe",
            "wsl"
        ]
    )]
    pub hook_shell: Option<String>,

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

    /// Detach HEAD in the new worktree/workspace
    #[arg(short, long, help_heading = "git worktree Options")]
    pub detach: bool,

    /// Do not checkout after creation
    #[arg(long, help_heading = "git worktree Options")]
    pub no_checkout: bool,

    /// Lock the worktree after creation (git only)
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
    /// Suppress output
    #[arg(short, long, help_heading = "Shared Options")]
    pub quiet: bool,

    /// When to use colored output (always, auto, never)
    #[arg(
        long,
        value_name = "WHEN",
        default_value = "auto",
        conflicts_with = "no_color",
        help_heading = "Shared Options"
    )]
    pub color: clap::ColorChoice,

    /// Disable colored output (equivalent to --color=never)
    #[arg(long, help_heading = "Shared Options")]
    pub no_color: bool,
}

/// Arguments for the `remove` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
VCS SUPPORT:
    Works with both git worktree and jj workspace:
    - Git: Removes worktree using `git worktree remove`
    - jj:  Forgets workspace using `jj workspace forget`

EXAMPLES:
    gwtx remove ../target-worktree-path
        Remove worktree/workspace with safety checks

    gwtx remove --i
        Select worktrees/workspaces to remove interactively

    gwtx remove --current
        Remove the worktree/workspace containing the current directory

    gwtx remove --dry-run ../target-worktree-path
        Preview what would be removed without executing

    gwtx remove --force ../target-worktree-path
        Force removal (skip safety checks and confirmation)

SAFETY CHECKS:
    By default, gwtx remove warns about:
    - Uncommitted changes (modified/staged files)
    - Unpushed commits (git) or commits not on remote bookmarks (jj)

    Use --force to skip safety checks and force removal.")]
pub(crate) struct RemoveArgs {
    /// Worktree/workspace paths to remove (required unless --interactive or --current)
    pub paths: Vec<PathBuf>,

    // --- gwtx Options ---
    /// Interactive mode: select worktrees/workspaces to remove
    #[arg(short, long, help_heading = "gwtx Options")]
    pub interactive: bool,

    /// Remove the worktree/workspace containing the current directory
    #[arg(short = 'c', long, help_heading = "gwtx Options")]
    pub current: bool,

    /// Preview actions without executing
    #[arg(long, help_heading = "gwtx Options")]
    pub dry_run: bool,

    /// Windows-only: select hook shell (pwsh, powershell, bash, cmd, wsl)
    #[cfg(windows)]
    #[arg(
        long,
        value_name = "SHELL",
        help_heading = "gwtx Options",
        value_parser = [
            "pwsh",
            "powershell",
            "bash",
            "git-bash",
            "gitbash",
            "cmd",
            "cmd.exe",
            "wsl"
        ]
    )]
    pub hook_shell: Option<String>,

    // --- git worktree Options ---
    /// Force removal even if worktree/workspace is dirty or locked
    #[arg(short, long, help_heading = "git worktree Options")]
    pub force: bool,

    // --- Shared Options ---
    /// Suppress output
    #[arg(short, long, help_heading = "Shared Options")]
    pub quiet: bool,

    /// When to use colored output (always, auto, never)
    #[arg(
        long,
        value_name = "WHEN",
        default_value = "auto",
        conflicts_with = "no_color",
        help_heading = "Shared Options"
    )]
    pub color: clap::ColorChoice,

    /// Disable colored output (equivalent to --color=never)
    #[arg(long, help_heading = "Shared Options")]
    pub no_color: bool,
}

/// Arguments for the `list` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
VCS SUPPORT:
    Works with both git worktree and jj workspace:
    - Git: Lists worktrees using `git worktree list`
    - jj:  Lists workspaces using `jj workspace list`

STATUS SYMBOLS:
    *  Uncommitted changes (modified, deleted, or untracked files)

COLUMNS:
    - Path: Worktree/workspace directory path
    - Branch: Branch name (git) or bookmark/workspace name (jj)
    - Commit: Short commit hash or change ID (jj)
    - Status: Uncommitted changes indicator

EXAMPLES:
    gwtx list
        List all worktrees/workspaces with detailed information

    gwtx list --header
        Show header row with column names

    gwtx list --path-only
        List only paths (useful for scripting)

    gwtx ls -p --header
        Combine options using the short alias")]
pub(crate) struct ListArgs {
    /// Show only worktree/workspace paths
    #[arg(short, long)]
    pub path_only: bool,

    /// Show header row
    #[arg(long)]
    pub header: bool,

    /// When to use colored output (always, auto, never)
    #[arg(
        long,
        value_name = "WHEN",
        default_value = "auto",
        conflicts_with = "no_color",
        help_heading = "Shared Options"
    )]
    pub color: clap::ColorChoice,
    /// Disable colored output (equivalent to --color=never)
    #[arg(long, help_heading = "Shared Options")]
    pub no_color: bool,
}

/// Arguments for the `trust` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
SECURITY:
    Hooks allow running custom commands before/after worktree operations.
    For security, hooks require explicit trust before execution.

    Trust is stored in: ~/.local/share/gwtx/trusted/
    Each repository's hooks are identified by a SHA256 hash.

    If hooks are modified in .gwtx/config.yaml, you must re-trust them.

HOOKS:
    Platform support: Unix-like systems (Linux, macOS) and Windows.
    On Windows, hooks run via an auto-detected shell (pwsh, powershell,
    Git Bash, or cmd). Override with --hook-shell or GWTHOOK_SHELL.

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
        Trust hooks in .gwtx/config.yaml for the current repository

    gwtx trust --yes
        Trust hooks without prompting

    gwtx trust --show
        Show hooks and trust status without trusting

    gwtx trust --check
        Exit 0 if hooks are trusted, 1 if trust is required

    gwtx trust /path/to/repo
        Trust hooks for a specific repository")]
pub(crate) struct TrustArgs {
    /// Path to repository (defaults to current directory)
    pub path: Option<PathBuf>,

    /// Trust without confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Show trusted hooks without trusting
    #[arg(long)]
    pub show: bool,

    /// Check trust status (exit 0 if trusted, 1 if untrusted)
    #[arg(long, conflicts_with = "show")]
    pub check: bool,

    /// When to use colored output (always, auto, never)
    #[arg(
        long,
        value_name = "WHEN",
        default_value = "auto",
        conflicts_with = "no_color"
    )]
    pub color: clap::ColorChoice,

    /// Disable colored output (equivalent to --color=never)
    #[arg(long)]
    pub no_color: bool,
}

/// Arguments for the `init` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
SHELL INTEGRATION:
    gwtx init enables shell integration features:
    - Shell completions for all gwtx commands and options
    - gwtx cd command for interactive worktree changing
    - Automatic trust warnings when entering directories with untrusted hooks

INSTALLATION:
    Add to your shell configuration file:

    Bash (~/.bashrc or ~/.bash_profile):
      eval \"$(gwtx init bash)\"

    Zsh (~/.zshrc):
      eval \"$(gwtx init zsh)\"

    Fish (~/.config/fish/config.fish):
      gwtx init fish | source

    PowerShell (profile, open with: $PROFILE):
      Invoke-Expression (& gwtx init powershell | Out-String)

    Elvish (~/.config/elvish/rc.elv):
      eval (gwtx init elvish | slurp)

FEATURES:
    Shell Completions
        Provides intelligent tab completion for gwtx commands and options.
        Works across all supported shells.

    gwtx cd Command
        Interactive fuzzy finder to select and cd to a worktree.
        Uses ratatui-based UI across platforms

        Note: gwtx cd requires shell integration.
        Without shell integration, use: cd \"$(gwtx path)\"

    Trust Warnings
        When entering a directory with untrusted hooks, gwtx displays a warning.
        Review and trust hooks using: gwtx trust

EXAMPLES:
    # Show init script for bash (add to ~/.bashrc)
    gwtx init bash

    # Show init script for zsh (add to ~/.zshrc)
    gwtx init zsh

    # Show full init script (used by shell config, not for manual viewing)
    gwtx init bash --print-full-init

TROUBLESHOOTING:
    If shell integration doesn't work after installation:
    1. Restart your shell or source the config file manually
    2. Verify the command is in your shell PATH: command -v gwtx
    3. Check shell config file was updated correctly
    4. Review shell-specific documentation: gwtx help init")]
pub(crate) struct InitArgs {
    /// Shell to generate init script for
    pub shell: Shell,

    /// Print the full init script instead of the stub
    #[arg(long)]
    pub print_full_init: bool,
}

/// Arguments for the `path` subcommand.
#[derive(Parser, Debug)]
#[command(after_help = "\
VCS SUPPORT:
    Works with both git worktree and jj workspace.

EXAMPLES:
    gwtx path
        Select a worktree/workspace interactively and print its path

    gwtx path --main
        Print the main worktree/workspace path (useful for shell integration)

    cd \"$(gwtx path)\"
        Select a worktree/workspace and change to it

    cd \"$(gwtx path --main)\"
        Change to the main worktree/workspace")]
pub(crate) struct PathArgs {
    /// Print the main worktree/workspace path instead of interactive selection
    #[arg(long)]
    pub main: bool,
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
