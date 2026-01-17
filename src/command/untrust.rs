use crate::cli::UntrustArgs;
use crate::{config, error::Error, error::Result, git, trust};

pub(crate) fn run(args: UntrustArgs) -> Result<()> {
    if args.list {
        let entries = trust::list_trusted()?;
        if entries.is_empty() {
            println!("No trusted repositories");
        } else {
            println!("Trusted repositories:");
            for entry in entries {
                println!("  {}", entry.main_worktree_path.display());
                println!("    Trusted at: {}", entry.trusted_at);
            }
        }
        return Ok(());
    }

    let repo_root = match args.path {
        Some(p) => p.canonicalize()?,
        None => git::repository_root()?,
    };

    let main_worktree_path = git::main_worktree_path()?;

    let config = config::load(&repo_root)?.ok_or_else(|| Error::ConfigNotFound {
        path: repo_root.clone(),
    })?;

    if trust::untrust(&main_worktree_path, &config.hooks)? {
        println!("Untrusted hooks for: {}", repo_root.display());
    } else {
        println!("Hooks were not trusted: {}", repo_root.display());
    }

    Ok(())
}
