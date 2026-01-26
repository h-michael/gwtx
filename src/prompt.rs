use crate::error::{Error, Result};

use std::io::{self, IsTerminal, Write};
use std::path::Path;

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

/// Prompt user to trust hooks (simple y/N prompt, not TUI-based)
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
