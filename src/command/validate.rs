use crate::config;
use crate::error::{Error, Result};
use crate::git;

/// Execute the `validate` subcommand.
pub(crate) fn run() -> Result<()> {
    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repo_root()?;
    config::load(&repo_root)?;

    println!("Config is valid");
    Ok(())
}
