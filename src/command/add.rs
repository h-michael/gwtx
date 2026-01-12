use crate::cli::AddArgs;
use crate::config::{self, Config, Link, OnConflict};
use crate::error::{Error, Result};
use crate::git;
use crate::hook::{self, HookEnv};
use crate::operation::{self, ConflictAction, check_conflict, create_directory, resolve_conflict};
use crate::output::Output;
use crate::prompt::{self, ConflictChoice};
use crate::trust;

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Execute the `add` subcommand.
pub(crate) fn run(mut args: AddArgs) -> Result<()> {
    let output = Output::new(args.quiet);

    // Check if we're in a git repository
    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    // Get repository root
    let repo_root = git::repo_root()?;

    // Initial config load for trust check
    let initial_config = config::load(&repo_root)?.unwrap_or_default();

    // Trust check for hooks (before interactive mode)
    if initial_config.hooks.has_hooks()
        && !args.no_setup
        && !trust::is_trusted(&repo_root, &initial_config.hooks)?
    {
        // Display hooks that need trust
        hook::display_hooks_for_review(&initial_config.hooks);

        eprintln!("\nError: Hooks are not trusted.");
        eprintln!("The .gwtx.toml file contains hooks that can execute arbitrary commands.");
        eprintln!("For security, you must explicitly review and trust these hooks.");
        eprintln!();
        eprintln!("To trust these hooks, run:");
        eprintln!("  gwtx trust           # Trust the hooks above");
        eprintln!();
        eprintln!("Or skip hooks entirely:");
        eprintln!("  gwtx add --no-setup <path>");
        return Err(Error::HooksNotTrusted);
    }

    // TOCTOU protection: reload config immediately before use
    // This prevents attacks where .gwtx.toml is modified between trust check and execution
    let config = config::load(&repo_root)?.unwrap_or_default();
    if config.hooks.has_hooks() && !args.no_setup && !trust::is_trusted(&repo_root, &config.hooks)?
    {
        eprintln!("\nError: .gwtx.toml was modified after trust check.");
        eprintln!("For security, hooks must be re-trusted after any changes.");
        eprintln!("Run: gwtx trust");
        return Err(Error::HooksNotTrusted);
    }

    // Handle interactive mode
    let worktree_path = if args.interactive {
        run_interactive(&mut args)?
    } else {
        // Non-interactive: path is required
        let path = args.path.clone().ok_or(Error::PathRequired)?;
        if path.is_absolute() {
            path
        } else {
            std::env::current_dir()?.join(&path)
        }
    };

    // Skip setup if requested - just run git worktree add
    if args.no_setup {
        if !args.dry_run {
            git::worktree_add(&args, &worktree_path)?;
        } else {
            output.dry_run(&format!(
                "Would run: git worktree add {}",
                worktree_path.display()
            ));
        }
        return Ok(());
    }

    // Pre-validate: Check all source files exist BEFORE creating worktree
    for link in &config.link {
        // Skip validation for glob patterns - they will be expanded later
        if contains_glob_pattern(&link.source) {
            continue;
        }
        let source = repo_root.join(&link.source);
        if !source.exists() {
            return Err(Error::SourceNotFound {
                path: link.source.clone(),
            });
        }
    }
    for copy in &config.copy {
        let source = repo_root.join(&copy.source);
        if !source.exists() {
            return Err(Error::SourceNotFound {
                path: copy.source.clone(),
            });
        }
    }

    // Create hook environment
    let worktree_name = worktree_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Get branch name and strip refs/heads/ prefix if present
    let branch = args
        .new_branch
        .clone()
        .or(args.commitish.clone())
        .or(args.new_branch_force.clone())
        .map(|b| b.strip_prefix("refs/heads/").unwrap_or(&b).to_string());

    let hook_env = HookEnv {
        worktree_path: worktree_path.to_string_lossy().to_string(),
        worktree_name,
        branch,
        repo_root: repo_root.to_string_lossy().to_string(),
    };

    // Run pre_add hooks
    if !config.hooks.pre_add.is_empty() {
        if args.dry_run {
            if !args.quiet {
                for cmd in &config.hooks.pre_add {
                    output.dry_run(&format!("Would run pre_add hook: {}", cmd));
                }
            }
        } else {
            hook::run_pre_add(&config.hooks, &hook_env, &repo_root, args.quiet)?;
        }
    }

    // Run git worktree add
    if !args.dry_run {
        git::worktree_add(&args, &worktree_path)?;
    } else {
        output.dry_run(&format!(
            "Would run: git worktree add {}",
            worktree_path.display()
        ));
    }

    // Process links and copies with rollback on failure
    if let Err(e) = run_setup(&args, &config, &repo_root, &worktree_path, &output) {
        // Rollback: remove the worktree on failure
        if !args.dry_run {
            eprintln!("Setup failed, rolling back worktree creation...");
            let _ = git::worktree_remove(&worktree_path, true);
        }
        return Err(e);
    }

    // Run post_add hooks
    if !config.hooks.post_add.is_empty() {
        if args.dry_run {
            if !args.quiet {
                for cmd in &config.hooks.post_add {
                    output.dry_run(&format!("Would run post_add hook: {}", cmd));
                }
            }
        } else if let Err(e) =
            hook::run_post_add(&config.hooks, &hook_env, &worktree_path, args.quiet)
        {
            eprintln!("Warning: post_add hook failed: {}", e);
            eprintln!("Worktree was created but post-setup may be incomplete.");
        }
    }

    output.success(&format!("Worktree created: {}", worktree_path.display()));

    Ok(())
}

/// Run interactive mode to select branch and path.
fn run_interactive(args: &mut AddArgs) -> Result<PathBuf> {
    // Get list of local and remote branches
    let local_branches = git::list_branches()?;
    let remote_branches = git::list_remote_branches()?;

    // Prompt for branch selection
    let branch_choice = prompt::prompt_branch_selection(&local_branches, &remote_branches)?;

    // Set branch in args
    if branch_choice.create_new {
        args.new_branch = Some(branch_choice.branch.clone());
        // Set base commitish if specified
        if let Some(base) = &branch_choice.base_commitish {
            args.commitish = Some(base.clone());
        }
    } else {
        args.commitish = Some(branch_choice.branch.clone());
    }

    // Suggest worktree path based on branch name
    let suggested_path = format!("../{}", branch_choice.branch.replace('/', "-"));

    // Prompt for worktree path
    let path = prompt::prompt_worktree_path(&suggested_path)?;

    // Update args.path for display purposes
    args.path = Some(path.clone());

    // Convert to absolute path
    let worktree_path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(&path)
    };

    Ok(worktree_path)
}

/// Run the setup operations (mkdir, symlinks and copies)
fn run_setup(
    args: &AddArgs,
    config: &Config,
    repo_root: &Path,
    worktree_path: &Path,
    output: &Output,
) -> Result<()> {
    let mut conflict_mode_override: Option<OnConflict> = args.on_conflict.map(|m| match m {
        crate::cli::OnConflictArg::Abort => OnConflict::Abort,
        crate::cli::OnConflictArg::Skip => OnConflict::Skip,
        crate::cli::OnConflictArg::Overwrite => OnConflict::Overwrite,
        crate::cli::OnConflictArg::Backup => OnConflict::Backup,
    });

    // Process mkdir
    for mkdir in &config.mkdir {
        let target = worktree_path.join(&mkdir.path);

        if args.dry_run {
            output.dry_run(&format!("Would create directory: {}", target.display()));
        } else {
            create_directory(&target)?;
            output.mkdir(&target, mkdir.description.as_deref());
        }
    }

    // Process symlinks (expand glob patterns first)
    for link in &config.link {
        let expanded_links = expand_link(link, repo_root)?;
        for expanded_link in expanded_links {
            let params = OperationParams {
                source: &repo_root.join(&expanded_link.source),
                target: &worktree_path.join(&expanded_link.target),
                op_type: FileOp::Link,
                config_mode: expanded_link.on_conflict.or(config.options.on_conflict),
                description: expanded_link.description.as_deref(),
            };
            process_operation(&params, &mut conflict_mode_override, args.dry_run, output)?;
        }
    }

    // Process copies
    for copy in &config.copy {
        let params = OperationParams {
            source: &repo_root.join(&copy.source),
            target: &worktree_path.join(&copy.target),
            op_type: FileOp::Copy,
            config_mode: copy.on_conflict.or(config.options.on_conflict),
            description: copy.description.as_deref(),
        };
        process_operation(&params, &mut conflict_mode_override, args.dry_run, output)?;
    }

    Ok(())
}

/// File operation type.
enum FileOp {
    Link,
    Copy,
}

/// Parameters for a file operation.
struct OperationParams<'a> {
    source: &'a Path,
    target: &'a Path,
    op_type: FileOp,
    config_mode: Option<OnConflict>,
    description: Option<&'a str>,
}

/// Process a single operation (symlink or copy) with conflict handling.
fn process_operation(
    params: &OperationParams,
    override_mode: &mut Option<OnConflict>,
    dry_run: bool,
    output: &Output,
) -> Result<()> {
    let OperationParams {
        source,
        target,
        op_type,
        config_mode,
        description,
    } = params;
    // Check for conflict
    if check_conflict(target) {
        // Determine conflict mode
        let mode = if let Some(mode) = *override_mode {
            mode
        } else if let Some(mode) = *config_mode {
            mode
        } else {
            // Prompt user
            let choice: ConflictChoice = prompt::prompt_conflict_with_all(target)?;
            if choice.apply_to_all {
                *override_mode = Some(choice.mode);
            }
            choice.mode
        };

        // Resolve conflict
        let action = resolve_conflict(target, mode)?;
        match action {
            ConflictAction::Abort => return Err(Error::Aborted),
            ConflictAction::Skip => {
                output.skip(target);
                return Ok(());
            }
            ConflictAction::Proceed => {
                // Continue with operation
            }
        }
    }

    // Perform operation
    if dry_run {
        let op_name = match op_type {
            FileOp::Link => "link",
            FileOp::Copy => "copy",
        };
        output.dry_run(&format!(
            "Would {}: {} -> {}",
            op_name,
            source.display(),
            target.display()
        ));
    } else {
        match op_type {
            FileOp::Link => {
                operation::create_symlink(source, target)?;
                output.link(source, target, *description);
            }
            FileOp::Copy => {
                operation::copy_file(source, target)?;
                output.copy(source, target, *description);
            }
        }
    }

    Ok(())
}

/// Check if a path contains glob patterns.
fn contains_glob_pattern(path: &Path) -> bool {
    path.to_str()
        .map(|s| s.contains('*') || s.contains('?') || s.contains('['))
        .unwrap_or(false)
}

/// Expand a link entry with glob patterns into multiple concrete link entries.
/// If skip_tracked is true, filter out git-tracked files.
fn expand_link(link: &Link, repo_root: &Path) -> Result<Vec<Link>> {
    let source_str = link.source.to_string_lossy();

    if !contains_glob_pattern(&link.source) {
        // No glob pattern, return as-is
        return Ok(vec![link.clone()]);
    }

    // Build glob matcher
    let glob = globset::GlobBuilder::new(&source_str)
        .literal_separator(true)
        .build()
        .map_err(|e| Error::ConfigValidation {
            message: format!("Invalid glob pattern '{}': {}", source_str, e),
        })?;
    let matcher = glob.compile_matcher();

    // Get git-tracked files if needed
    let tracked_files: HashSet<PathBuf> = if link.skip_tracked {
        git::list_tracked_files()?.into_iter().collect()
    } else {
        HashSet::new()
    };

    // Walk the repository and find matching files
    let mut results = Vec::new();
    for entry in walkdir::WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.file_name() != ".git")
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip if it's a directory
        if path.is_dir() {
            continue;
        }

        // Get relative path from repo root
        let rel_path = match path.strip_prefix(repo_root) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check if it matches the glob pattern
        if !matcher.is_match(rel_path) {
            continue;
        }

        // Skip if it's tracked and skip_tracked is true
        if link.skip_tracked && tracked_files.contains(rel_path) {
            continue;
        }

        // Create a link entry for this file
        let mut file_link = link.clone();
        file_link.source = rel_path.to_path_buf();
        file_link.target = rel_path.to_path_buf();
        file_link.skip_tracked = false; // Already filtered, no need to check again
        results.push(file_link);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_glob_pattern_with_asterisk() {
        assert!(contains_glob_pattern(Path::new("secrets/*")));
    }

    #[test]
    fn test_contains_glob_pattern_with_question() {
        assert!(contains_glob_pattern(Path::new("file?.txt")));
    }

    #[test]
    fn test_contains_glob_pattern_with_bracket() {
        assert!(contains_glob_pattern(Path::new("file[0-9].txt")));
    }

    #[test]
    fn test_contains_glob_pattern_none() {
        assert!(!contains_glob_pattern(Path::new("secrets/config.json")));
    }
}
