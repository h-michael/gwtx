use crate::error::{Error, Result};
use crate::git;
use crate::interactive::run_path_interactive;

pub(crate) fn run() -> Result<()> {
    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let worktrees = git::list_worktrees()?;
    let selected = run_path_interactive(&worktrees)?;
    println!("{}", selected.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_run_not_in_repo() {
        // This test would need to mock git::is_inside_repo() to return false
        // For now, it's a documentation test showing the error case
        // In a real environment, this would need a test setup with mocking
    }
}
