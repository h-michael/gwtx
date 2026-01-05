use crate::cli::AddArgs;
use crate::config::{self, Config, OnConflict};
use crate::error::{Error, Result};
use crate::git;
use crate::operation::{self, ConflictAction, check_conflict, create_directory, resolve_conflict};
use crate::output::Output;
use crate::prompt::{self, ConflictChoice};

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

    // Load config (optional - if not found, just run git worktree add)
    let config = config::load(&repo_root)?.unwrap_or_default();

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

    // Process symlinks
    for link in &config.link {
        let source = repo_root.join(&link.source);
        let target = worktree_path.join(&link.target);

        process_operation(
            &source,
            &target,
            link.on_conflict.or(config.options.on_conflict),
            &mut conflict_mode_override,
            args.dry_run,
            operation::create_symlink,
            link.description.as_deref(),
            output,
            true, // is_link
        )?;
    }

    // Process copies
    for copy in &config.copy {
        let source = repo_root.join(&copy.source);
        let target = worktree_path.join(&copy.target);

        process_operation(
            &source,
            &target,
            copy.on_conflict.or(config.options.on_conflict),
            &mut conflict_mode_override,
            args.dry_run,
            operation::copy_file,
            copy.description.as_deref(),
            output,
            false, // is_link
        )?;
    }

    Ok(())
}

/// Process a single operation (symlink or copy) with conflict handling
#[allow(clippy::too_many_arguments)]
fn process_operation<F>(
    source: &Path,
    target: &Path,
    config_mode: Option<OnConflict>,
    override_mode: &mut Option<OnConflict>,
    dry_run: bool,
    operation: F,
    description: Option<&str>,
    output: &Output,
    is_link: bool,
) -> Result<()>
where
    F: FnOnce(&Path, &Path) -> Result<()>,
{
    // Check for conflict
    if check_conflict(target) {
        // Determine conflict mode
        let mode = if let Some(mode) = *override_mode {
            mode
        } else if let Some(mode) = config_mode {
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
        let op_name = if is_link { "link" } else { "copy" };
        output.dry_run(&format!(
            "Would {}: {} -> {}",
            op_name,
            source.display(),
            target.display()
        ));
    } else {
        operation(source, target)?;
        if is_link {
            output.link(source, target, description);
        } else {
            output.copy(source, target, description);
        }
    }

    Ok(())
}
