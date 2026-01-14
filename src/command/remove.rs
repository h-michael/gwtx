use crate::cli::RemoveArgs;
use crate::color::ColorConfig;
use crate::config;
use crate::error::{Error, Result};
use crate::git::{self, WorktreeInfo};
use crate::hook::{self, HookEnv};
use crate::output::Output;
use crate::prompt::{self, SafetyWarning};
use crate::trust;

use std::path::PathBuf;

pub(crate) fn run(args: RemoveArgs, color: ColorConfig) -> Result<()> {
    let output = Output::new(args.quiet, color);

    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repository_root()?;

    // Initial config load for trust check
    let initial_config = config::load(&repo_root)?.unwrap_or_default();

    // Trust check for hooks
    if initial_config.hooks.has_hooks() && !trust::is_trusted(&repo_root, &initial_config.hooks)? {
        // Display hooks that need trust
        hook::display_hooks_for_review(&initial_config.hooks);

        eprintln!();
        eprintln!("Error: Hooks are not trusted.");
        eprintln!("The .gwtx.toml file contains hooks that can execute arbitrary commands.");
        eprintln!("For security, you must explicitly review and trust these hooks.");
        eprintln!();
        eprintln!("To trust these hooks, run:");
        eprintln!("  gwtx trust");
        return Err(Error::HooksNotTrusted);
    }

    // TOCTOU protection: reload config immediately before use
    let config = config::load(&repo_root)?.unwrap_or_default();
    if config.hooks.has_hooks() && !trust::is_trusted(&repo_root, &config.hooks)? {
        eprintln!("\nError: .gwtx.toml was modified after trust check.");
        eprintln!("For security, hooks must be re-trusted after any changes.");
        eprintln!("Run: gwtx trust");
        return Err(Error::HooksNotTrusted);
    }

    let worktrees = git::list_worktrees()?;

    let targets = if args.interactive {
        select_worktrees_interactively(&worktrees)?
    } else if args.paths.is_empty() {
        return Err(Error::PathRequired);
    } else {
        resolve_worktree_paths(&args.paths, &worktrees)?
    };

    for path in &targets {
        if is_main_worktree(path, &worktrees) {
            return Err(Error::CannotRemoveMainWorktree { path: path.clone() });
        }
    }

    let warnings = if !args.force {
        collect_safety_warnings(&targets)?
    } else {
        vec![]
    };

    if !warnings.is_empty() {
        if args.dry_run {
            for warning in &warnings {
                display_warning(&output, warning);
            }
        } else if prompt::is_interactive() {
            if !prompt::prompt_remove_confirmation(&warnings, color)? {
                return Err(Error::Aborted);
            }
        } else {
            let first_warning = &warnings[0];
            if first_warning.has_uncommitted {
                return Err(Error::WorktreeHasUncommittedChanges {
                    path: first_warning.path.clone(),
                });
            } else {
                return Err(Error::WorktreeHasUnpushedCommits {
                    path: first_warning.path.clone(),
                });
            }
        }
    }

    for path in &targets {
        // Create hook environment
        let worktree_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let hook_env = HookEnv {
            worktree_path: path.to_string_lossy().to_string(),
            worktree_name,
            branch: None, // Branch info not available for remove
            repo_root: repo_root.to_string_lossy().to_string(),
        };

        // Run pre_remove hooks
        if !config.hooks.pre_remove.is_empty() {
            if args.dry_run {
                if !args.quiet {
                    for entry in &config.hooks.pre_remove {
                        let display = entry.description.as_deref().unwrap_or(&entry.command);
                        output.dry_run(&format!("Would run pre_remove hook: {}", display));
                    }
                }
            } else {
                hook::run_pre_remove(&config.hooks, &hook_env, path, &output)?;
            }
        }

        if args.dry_run {
            output.dry_run(&format!("Would remove: {}", path.display()));
        } else {
            let use_force = args.force || !warnings.is_empty();
            remove_worktree(path, use_force)?;
            output.remove(path);
        }

        // Run post_remove hooks
        if !config.hooks.post_remove.is_empty() {
            if args.dry_run {
                if !args.quiet {
                    for entry in &config.hooks.post_remove {
                        let display = entry.description.as_deref().unwrap_or(&entry.command);
                        output.dry_run(&format!("Would run post_remove hook: {}", display));
                    }
                }
            } else if let Err(e) =
                hook::run_post_remove(&config.hooks, &hook_env, &repo_root, &output)
            {
                // Extract exit code from error if available
                let exit_code = match &e {
                    Error::HookFailed { exit_code, .. } => *exit_code,
                    _ => None,
                };
                output.hook_warning("post_remove", &e.to_string(), exit_code);
                output.hook_note("Worktree was removed but post-cleanup may be incomplete.");
            }
        }
    }

    Ok(())
}

fn select_worktrees_interactively(worktrees: &[WorktreeInfo]) -> Result<Vec<PathBuf>> {
    let paths = prompt::prompt_worktree_selection(worktrees)?;
    Ok(paths)
}

fn resolve_worktree_paths(paths: &[PathBuf], worktrees: &[WorktreeInfo]) -> Result<Vec<PathBuf>> {
    let mut resolved = Vec::new();

    for path in paths {
        let abs_path = if path.is_absolute() {
            path.clone()
        } else {
            std::env::current_dir()?.join(path)
        };

        let canonical = abs_path
            .canonicalize()
            .map_err(|_| Error::WorktreeNotFound { path: path.clone() })?;

        if !worktrees.iter().any(|wt| wt.path == canonical) {
            return Err(Error::WorktreeNotFound { path: path.clone() });
        }

        resolved.push(canonical);
    }

    Ok(resolved)
}

fn is_main_worktree(path: &PathBuf, worktrees: &[WorktreeInfo]) -> bool {
    worktrees
        .iter()
        .find(|wt| &wt.path == path)
        .map(|wt| wt.is_main)
        .unwrap_or(false)
}

fn collect_safety_warnings(targets: &[PathBuf]) -> Result<Vec<SafetyWarning>> {
    let mut warnings = Vec::new();

    for path in targets {
        let status = git::worktree_status(path)?;
        let unpushed = git::worktree_unpushed_commits(path)?;

        if status.has_uncommitted_changes || unpushed.has_unpushed {
            warnings.push(SafetyWarning {
                path: path.clone(),
                has_uncommitted: status.has_uncommitted_changes,
                modified_count: status.modified_count,
                deleted_count: status.deleted_count,
                untracked_count: status.untracked_count,
                has_unpushed: unpushed.has_unpushed,
                unpushed_count: unpushed.count,
            });
        }
    }

    Ok(warnings)
}

fn display_warning(output: &Output, warning: &SafetyWarning) {
    if warning.modified_count > 0 {
        output.safety_warning(
            &warning.path,
            &format!("{} modified file(s)", warning.modified_count),
        );
    }
    if warning.deleted_count > 0 {
        output.safety_warning(
            &warning.path,
            &format!("{} deleted file(s)", warning.deleted_count),
        );
    }
    if warning.untracked_count > 0 {
        output.safety_warning(
            &warning.path,
            &format!("{} untracked file(s)", warning.untracked_count),
        );
    }
    if warning.has_unpushed {
        output.safety_warning(
            &warning.path,
            &format!("{} unpushed commit(s)", warning.unpushed_count),
        );
    }
}

fn remove_worktree(path: &PathBuf, force: bool) -> Result<()> {
    use std::process::Command;

    let mut cmd = Command::new("git");
    cmd.args(["worktree", "remove"]);

    if force {
        cmd.arg("--force");
    }

    cmd.arg(path);

    let output = cmd.output()?;

    if !output.status.success() {
        return Err(Error::GitWorktreeRemoveFailed {
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}
