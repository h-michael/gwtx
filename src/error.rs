use std::path::PathBuf;

use thiserror::Error;

/// Application errors.
#[derive(Debug, Error)]
pub(crate) enum Error {
    #[error("Failed to parse config: {message}")]
    ConfigParse { message: String },

    #[error("Invalid config:\n{message}")]
    ConfigValidation { message: String },

    #[error("Not inside a git repository")]
    NotInGitRepo,

    #[error("Path is required. Use -i for interactive mode or provide a path.")]
    PathRequired,

    #[cfg(feature = "libgit2")]
    #[error("git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[cfg(not(feature = "libgit2"))]
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

    #[cfg(windows)]
    #[error(
        "Failed to create symlink: permission denied\n  Enable Developer Mode in Windows Settings or run as administrator."
    )]
    WindowsSymlinkPermission,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Result type alias for this crate.
pub(crate) type Result<T> = std::result::Result<T, Error>;
