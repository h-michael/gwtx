use crate::color::ColorScheme;
use crate::config::{HookEntry, Hooks};
use crate::error::{Error, Result};
use crate::output::Output;

use std::io::IsTerminal;
use std::path::Path;
use std::process::{Command, Stdio};

/// Template variables for hooks.
///
/// Provides context information that can be used in hook commands.
/// All variables are automatically shell-escaped when expanded.
pub(crate) struct HookEnv {
    /// Worktree path
    pub worktree_path: String,
    /// Worktree directory name
    pub worktree_name: String,
    /// Branch name (if applicable)
    pub branch: Option<String>,
    /// Repository root path
    pub repo_root: String,
}

impl HookEnv {
    /// Expand template variables in a command string.
    ///
    /// # Supported Variables
    /// - `{{worktree_path}}`: Full path to the worktree
    /// - `{{worktree_name}}`: Worktree directory name
    /// - `{{branch}}`: Branch name (empty if detached or not available)
    /// - `{{repo_root}}`: Repository root path
    ///
    /// All values are automatically shell-escaped to prevent command injection.
    fn expand_template(&self, cmd: &str) -> String {
        let mut result = cmd.to_string();

        // Replace template variables with shell-escaped values
        result = result.replace("{{worktree_path}}", &shell_escape(&self.worktree_path));
        result = result.replace("{{worktree_name}}", &shell_escape(&self.worktree_name));
        result = result.replace("{{repo_root}}", &shell_escape(&self.repo_root));

        if let Some(branch) = &self.branch {
            result = result.replace("{{branch}}", &shell_escape(branch));
        } else {
            result = result.replace("{{branch}}", "");
        }

        result
    }
}

/// Escape a string for safe use in shell commands.
///
/// Uses POSIX sh single quote escaping. This method wraps the entire
/// string in single quotes and escapes any single quotes within the string.
///
/// # Safety
/// This prevents command injection by ensuring special shell characters
/// (like `$`, `` ` ``, `&`, `|`, etc.) are treated as literal text.
///
/// # Platform Support
/// - **Unix/Linux**: Uses POSIX sh-compatible escaping
/// - **Windows**: Hooks are not supported on Windows
fn shell_escape(s: &str) -> String {
    // Use single quotes and escape any single quotes in the string
    // This works by: ending quote, escaped quote, start quote
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Execute a single hook command
#[cfg(unix)]
fn execute_hook(command: &str, env: &HookEnv, working_dir: &Path) -> Result<()> {
    let shell = "sh";
    let shell_arg = "-c";

    // Expand template variables
    let expanded_command = env.expand_template(command);

    let mut cmd = Command::new(shell);
    cmd.arg(shell_arg)
        .arg(&expanded_command)
        .current_dir(working_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status().map_err(|e| Error::HookExecutionFailed {
        command: command.to_string(),
        cause: e.to_string(),
    })?;

    if !status.success() {
        return Err(Error::HookFailed {
            command: command.to_string(),
            exit_code: status.code(),
            stderr: String::new(), // stderr is already displayed
        });
    }

    Ok(())
}

/// Execute a single hook command (Windows - not supported)
#[cfg(windows)]
fn execute_hook(_command: &str, _env: &HookEnv, _working_dir: &Path) -> Result<()> {
    Err(Error::WindowsHooksNotSupported)
}

/// Execute pre_add hooks
pub(crate) fn run_pre_add(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    output: &Output,
) -> Result<()> {
    for (i, entry) in hooks.pre_add.iter().enumerate() {
        output.hook_running(
            "pre_add",
            i + 1,
            hooks.pre_add.len(),
            &entry.command,
            entry.description.as_deref(),
        );
        execute_hook(&entry.command, env, working_dir)?;
        output.hook_separator();
    }
    Ok(())
}

/// Execute post_add hooks
pub(crate) fn run_post_add(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    output: &Output,
) -> Result<()> {
    for (i, entry) in hooks.post_add.iter().enumerate() {
        output.hook_running(
            "post_add",
            i + 1,
            hooks.post_add.len(),
            &entry.command,
            entry.description.as_deref(),
        );
        execute_hook(&entry.command, env, working_dir)?;
        output.hook_separator();
    }
    Ok(())
}

/// Execute pre_remove hooks
pub(crate) fn run_pre_remove(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    output: &Output,
) -> Result<()> {
    for (i, entry) in hooks.pre_remove.iter().enumerate() {
        output.hook_running(
            "pre_remove",
            i + 1,
            hooks.pre_remove.len(),
            &entry.command,
            entry.description.as_deref(),
        );
        execute_hook(&entry.command, env, working_dir)?;
        output.hook_separator();
    }
    Ok(())
}

/// Execute post_remove hooks
pub(crate) fn run_post_remove(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    output: &Output,
) -> Result<()> {
    for (i, entry) in hooks.post_remove.iter().enumerate() {
        output.hook_running(
            "post_remove",
            i + 1,
            hooks.post_remove.len(),
            &entry.command,
            entry.description.as_deref(),
        );
        execute_hook(&entry.command, env, working_dir)?;
        output.hook_separator();
    }
    Ok(())
}

/// Display dry-run output for hook entries.
pub(crate) fn dry_run_hooks(hook_type: &str, entries: &[HookEntry], output: &Output) {
    for entry in entries {
        let display = entry.description.as_deref().unwrap_or(&entry.command);
        output.dry_run(&format!("Would run {} hook: {}", hook_type, display));
    }
}

/// Display a list of hook entries with optional color formatting.
fn display_hook_entries(entries: &[HookEntry], hook_type: &str, use_color: bool) {
    if entries.is_empty() {
        return;
    }

    eprintln!();
    if use_color {
        eprintln!("{}", ColorScheme::hook_type(&format!("{}:", hook_type)));
    } else {
        eprintln!("{}:", hook_type);
    }

    for entry in entries {
        eprintln!("  {}", entry.command);
        if let Some(desc) = &entry.description {
            if use_color {
                eprintln!(
                    "  {} {}",
                    ColorScheme::hook_arrow("->"),
                    ColorScheme::hook_description(desc)
                );
            } else {
                eprintln!("  -> {}", desc);
            }
        } else if use_color {
            eprintln!(
                "  {} {}",
                ColorScheme::hook_arrow("->"),
                ColorScheme::dimmed("no description")
            );
        } else {
            eprintln!("  -> no description");
        }
    }
}

/// Display hooks for user review before trusting
pub(crate) fn display_hooks_for_review(hooks: &Hooks) {
    let use_color = std::io::stderr().is_terminal();

    if use_color {
        eprintln!(
            "{}",
            ColorScheme::warning("WARNING: Untrusted hooks detected in .gwtx.toml")
        );
    } else {
        eprintln!("WARNING: Untrusted hooks detected in .gwtx.toml");
    }
    eprintln!();
    eprintln!("Trusting will allow ALL hooks in this file to execute:");

    display_hook_entries(&hooks.pre_add, "pre_add", use_color);
    display_hook_entries(&hooks.post_add, "post_add", use_color);
    display_hook_entries(&hooks.pre_remove, "pre_remove", use_color);
    display_hook_entries(&hooks.post_remove, "post_remove", use_color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_escape_basic() {
        assert_eq!(shell_escape("hello"), "'hello'");
        assert_eq!(shell_escape("hello world"), "'hello world'");
    }

    #[test]
    fn test_shell_escape_single_quote() {
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
        assert_eq!(shell_escape("a'b'c"), "'a'\\''b'\\''c'");
    }

    #[test]
    fn test_shell_escape_special_chars() {
        // Test shell metacharacters are properly quoted
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
        assert_eq!(shell_escape("`date`"), "'`date`'");
        assert_eq!(shell_escape("test & pause"), "'test & pause'");
        assert_eq!(shell_escape("foo | bar"), "'foo | bar'");
        assert_eq!(shell_escape("test > file"), "'test > file'");
    }

    #[test]
    fn test_shell_escape_empty() {
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn test_expand_template_basic() {
        let env = HookEnv {
            worktree_path: "/path/to/worktree".to_string(),
            worktree_name: "my-feature".to_string(),
            branch: Some("feature/test".to_string()),
            repo_root: "/path/to/repo".to_string(),
        };

        let result = env.expand_template("echo {{worktree_name}}");
        assert!(result.contains("my-feature"));
        assert!(result.contains("echo"));
    }

    #[test]
    fn test_expand_template_with_special_chars() {
        let env = HookEnv {
            worktree_path: "/path/with spaces/worktree".to_string(),
            worktree_name: "feat'ure".to_string(),
            branch: Some("fix/$bug".to_string()),
            repo_root: "/repo".to_string(),
        };

        let result = env.expand_template("cd {{worktree_path}}");
        // Should be properly escaped
        assert!(result.contains('\''));

        let result = env.expand_template("echo {{worktree_name}}");
        assert!(result.contains('\''));

        let result = env.expand_template("git checkout {{branch}}");
        assert!(result.contains('\''));
    }

    #[test]
    fn test_expand_template_no_branch() {
        let env = HookEnv {
            worktree_path: "/path/to/worktree".to_string(),
            worktree_name: "detached".to_string(),
            branch: None,
            repo_root: "/repo".to_string(),
        };

        let result = env.expand_template("echo {{branch}}");
        assert_eq!(result, "echo ");
    }

    #[test]
    fn test_expand_template_multiple_variables() {
        let env = HookEnv {
            worktree_path: "/worktree".to_string(),
            worktree_name: "test".to_string(),
            branch: Some("main".to_string()),
            repo_root: "/repo".to_string(),
        };

        let result = env.expand_template("cd {{worktree_path}} && git checkout {{branch}} && pwd");
        assert!(result.contains("/worktree"));
        assert!(result.contains("main"));
    }
}
