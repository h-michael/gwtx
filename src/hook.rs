use crate::config::Hooks;
use crate::error::{Error, Result};

use std::path::Path;
use std::process::{Command, Stdio};

/// Template variables for hooks
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
    /// Expand template variables in a command string
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

/// Escape a string for safe use in shell commands
/// Uses POSIX sh single quote escaping
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
    quiet: bool,
) -> Result<()> {
    for (i, cmd) in hooks.pre_add.iter().enumerate() {
        if !quiet {
            println!(
                "Running pre_add hook [{}/{}]: {}",
                i + 1,
                hooks.pre_add.len(),
                cmd
            );
        }

        execute_hook(cmd, env, working_dir)?;
    }
    Ok(())
}

/// Execute post_add hooks
pub(crate) fn run_post_add(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    quiet: bool,
) -> Result<()> {
    for (i, cmd) in hooks.post_add.iter().enumerate() {
        if !quiet {
            println!(
                "Running post_add hook [{}/{}]: {}",
                i + 1,
                hooks.post_add.len(),
                cmd
            );
        }

        execute_hook(cmd, env, working_dir)?;
    }
    Ok(())
}

/// Execute pre_remove hooks
pub(crate) fn run_pre_remove(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    quiet: bool,
) -> Result<()> {
    for (i, cmd) in hooks.pre_remove.iter().enumerate() {
        if !quiet {
            println!(
                "Running pre_remove hook [{}/{}]: {}",
                i + 1,
                hooks.pre_remove.len(),
                cmd
            );
        }

        execute_hook(cmd, env, working_dir)?;
    }
    Ok(())
}

/// Execute post_remove hooks
pub(crate) fn run_post_remove(
    hooks: &Hooks,
    env: &HookEnv,
    working_dir: &Path,
    quiet: bool,
) -> Result<()> {
    for (i, cmd) in hooks.post_remove.iter().enumerate() {
        if !quiet {
            println!(
                "Running post_remove hook [{}/{}]: {}",
                i + 1,
                hooks.post_remove.len(),
                cmd
            );
        }

        execute_hook(cmd, env, working_dir)?;
    }
    Ok(())
}

/// Display hooks for user review before trusting
pub(crate) fn display_hooks_for_review(hooks: &Hooks) {
    eprintln!();
    eprintln!("WARNING: Untrusted hooks detected in .gwtx.toml");
    eprintln!();
    eprintln!("Trusting will allow ALL hooks in this file to execute:");

    if !hooks.pre_add.is_empty() {
        eprintln!();
        eprintln!("pre_add (before worktree creation):");
        for cmd in &hooks.pre_add {
            eprintln!("  $ {}", cmd);
        }
    }

    if !hooks.post_add.is_empty() {
        eprintln!();
        eprintln!("post_add (after worktree creation):");
        for cmd in &hooks.post_add {
            eprintln!("  $ {}", cmd);
        }
    }

    if !hooks.pre_remove.is_empty() {
        eprintln!();
        eprintln!("pre_remove (before worktree removal):");
        for cmd in &hooks.pre_remove {
            eprintln!("  $ {}", cmd);
        }
    }

    if !hooks.post_remove.is_empty() {
        eprintln!();
        eprintln!("post_remove (after worktree removal):");
        for cmd in &hooks.post_remove {
            eprintln!("  $ {}", cmd);
        }
    }
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
