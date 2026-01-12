use std::path::PathBuf;

use thiserror::Error;

/// Application errors.
#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Failed to parse config: {message}")]
    ConfigParse { message: String },

    #[error("Invalid config:\n{message}")]
    ConfigValidation { message: String },

    #[error("Not inside a git repository. Run this command from within a git repository.")]
    NotInGitRepo,

    #[error("Path is required. Use -i for interactive mode or provide a path.")]
    PathRequired,

    #[error("git worktree add failed:\n{stderr}")]
    GitWorktreeAddFailed { stderr: String },

    #[error("Source not found: {path}\n  Check if the file exists in the repository root.")]
    SourceNotFound { path: PathBuf },

    #[error("Failed to create symlink: {source} -> {target}")]
    SymlinkFailed {
        source: PathBuf,
        target: PathBuf,
        #[source]
        cause: std::io::Error,
    },

    #[error("Failed to copy: {source} -> {target}")]
    CopyFailed {
        source: PathBuf,
        target: PathBuf,
        #[source]
        cause: std::io::Error,
    },

    #[error("Operation aborted by user")]
    Aborted,

    #[error(
        "Interactive prompt required but running in non-interactive mode\n  Use --on-conflict to specify how to handle conflicts."
    )]
    NonInteractive,

    #[error("Selector error: {message}")]
    Selector { message: String },

    #[error("Cannot remove main worktree: {path}")]
    CannotRemoveMainWorktree { path: PathBuf },

    #[error("No worktrees available to remove")]
    NoWorktreesToRemove,

    #[error("Worktree has uncommitted changes: {path}\n  Use --force to remove anyway.")]
    WorktreeHasUncommittedChanges { path: PathBuf },

    #[error("Worktree has unpushed commits: {path}\n  Use --force to remove anyway.")]
    WorktreeHasUnpushedCommits { path: PathBuf },

    #[error("Worktree not found: {path}")]
    WorktreeNotFound { path: PathBuf },

    #[error("git worktree remove failed:\n{stderr}")]
    GitWorktreeRemoveFailed { stderr: String },

    #[cfg(windows)]
    #[error(
        "Failed to create symlink: permission denied\n  Enable Developer Mode in Windows Settings or run as administrator."
    )]
    WindowsSymlinkPermission,

    #[error("")]
    HooksNotTrusted,

    #[error("Hook execution failed: {command}\n  {cause}")]
    HookExecutionFailed { command: String, cause: String },

    #[error("Hook failed: {command}")]
    HookFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Trust storage directory not found")]
    TrustStorageNotFound,

    #[error("Trust file corrupted: {message}")]
    TrustFileCorrupted { message: String },

    #[error("Trust file serialization failed: {message}")]
    TrustFileSerialization { message: String },

    #[error("Config file not found: {path}")]
    ConfigNotFound { path: PathBuf },

    #[cfg(windows)]
    #[error(
        "Hooks are not supported on Windows yet.\n\
        \n\
        For Windows users:\n\
        - Use Git Bash or WSL for hooks functionality\n\
        - Or use --no-setup to skip hooks"
    )]
    WindowsHooksNotSupported,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result type alias for this crate.
pub(crate) type Result<T> = std::result::Result<T, Error>;
