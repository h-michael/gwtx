use crate::cli::TrustArgs;
use crate::{config, error::Error, error::Result, git, prompt, trust};

pub(crate) fn run(args: TrustArgs) -> Result<()> {
    let repo_root = match args.path {
        Some(p) => p.canonicalize()?,
        None => git::repo_root()?,
    };

    let config = config::load(&repo_root)?.ok_or_else(|| Error::ConfigNotFound {
        path: repo_root.clone(),
    })?;

    if !config.hooks.has_hooks() {
        println!("No hooks defined in .gwtx.toml");
        return Ok(());
    }

    if args.show {
        println!("Hooks in {}:", repo_root.display());
        if !config.hooks.pre_add.is_empty() {
            println!("\npre_add:");
            for cmd in &config.hooks.pre_add {
                println!("  - {}", cmd);
            }
        }
        if !config.hooks.post_add.is_empty() {
            println!("\npost_add:");
            for cmd in &config.hooks.post_add {
                println!("  - {}", cmd);
            }
        }
        if !config.hooks.pre_remove.is_empty() {
            println!("\npre_remove:");
            for cmd in &config.hooks.pre_remove {
                println!("  - {}", cmd);
            }
        }
        if !config.hooks.post_remove.is_empty() {
            println!("\npost_remove:");
            for cmd in &config.hooks.post_remove {
                println!("  - {}", cmd);
            }
        }

        let is_trusted = trust::is_trusted(&repo_root, &config.hooks)?;
        println!(
            "\nTrust status: {}",
            if is_trusted { "trusted" } else { "not trusted" }
        );
        return Ok(());
    }

    // Display hooks
    println!();
    println!("WARNING: Review these commands before trusting");
    println!();
    println!("Repository: {}", repo_root.display());

    if !config.hooks.pre_add.is_empty() {
        println!("pre_add (before worktree creation):");
        for cmd in &config.hooks.pre_add {
            println!("  $ {}", cmd);
        }
        println!();
    }
    if !config.hooks.post_add.is_empty() {
        println!("post_add (after worktree creation):");
        for cmd in &config.hooks.post_add {
            println!("  $ {}", cmd);
        }
        println!();
    }
    if !config.hooks.pre_remove.is_empty() {
        println!("pre_remove (before worktree removal):");
        for cmd in &config.hooks.pre_remove {
            println!("  $ {}", cmd);
        }
        println!();
    }
    if !config.hooks.post_remove.is_empty() {
        println!("post_remove (after worktree removal):");
        for cmd in &config.hooks.post_remove {
            println!("  $ {}", cmd);
        }
        println!();
    }

    // Prompt for confirmation
    if prompt::is_interactive() {
        if prompt::prompt_trust_hooks(&repo_root)? {
            trust::trust(&repo_root, &config.hooks)?;
            println!("\n✓ Hooks trusted for: {}", repo_root.display());
            println!("These hooks will now run automatically on gwtx add/remove commands.");
        } else {
            println!("\nHooks were not trusted.");
            return Err(Error::Aborted);
        }
    } else {
        // Non-interactive: cannot prompt
        eprintln!("\nError: Cannot prompt for confirmation in non-interactive mode.");
        eprintln!("Run this command in an interactive terminal to trust hooks.");
        return Err(Error::NonInteractive);
    }

    Ok(())
}
