use crate::cli::RemoveArgs;
use crate::color::{self, ColorConfig};
use crate::command::trust_check::{TrustHint, load_config_with_trust_check};
use crate::error::{Error, Result};
use crate::git::{self, WorktreeInfo};
use crate::hook::{self, HookEnv};
use crate::output::Output;
use crate::prompt::{self, SafetyWarning};

use std::path::PathBuf;

pub(crate) fn run(args: RemoveArgs, color: ColorConfig) -> Result<()> {
    let output = Output::new(args.quiet, color);

    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repository_root()?;

    // Get main worktree path for trust operations
    let main_worktree_path = git::main_worktree_path_for(&repo_root)?;

    let config =
        load_config_with_trust_check(&repo_root, &main_worktree_path, true, TrustHint::None)?;
    color::set_cli_theme(&config.ui.colors);

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

        let hook_shell = {
            #[cfg(windows)]
            {
                args.hook_shell
                    .clone()
                    .or_else(|| config.hooks.hook_shell.clone())
            }
            #[cfg(not(windows))]
            {
                None
            }
        };

        let hook_env = HookEnv {
            worktree_path: path.to_string_lossy().to_string(),
            worktree_name,
            branch: None, // Branch info not available for remove
            repo_root: repo_root.to_string_lossy().to_string(),
            hook_shell,
        };

        // Run pre_remove hooks
        if !config.hooks.pre_remove.is_empty() {
            if args.dry_run {
                if !args.quiet {
                    hook::dry_run_hooks("pre_remove", &config.hooks.pre_remove, &output);
                }
            } else {
                hook::run_pre_remove(&config.hooks, &hook_env, path, &output)?;
            }
        }

        if args.dry_run {
            output.dry_run(&format!("Would remove: {}", path.display()));
        } else {
            let use_force = args.force || !warnings.is_empty();
            git::worktree_remove_checked(path, use_force)?;
            output.remove(path);
        }

        // Run post_remove hooks
        if !config.hooks.post_remove.is_empty() {
            if args.dry_run {
                if !args.quiet {
                    hook::dry_run_hooks("post_remove", &config.hooks.post_remove, &output);
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
    // Clear screen before entering interactive mode
    prompt::clear_screen_interactive()?;

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
