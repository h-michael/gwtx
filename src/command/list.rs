use crate::cli::ListArgs;
use crate::error::{Error, Result};
use crate::git::{self, WorktreeInfo};
use crate::output::Output;

pub(crate) fn run(args: ListArgs) -> Result<()> {
    let output = Output::new(false);

    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let worktrees = git::list_worktrees()?;

    if args.path_only {
        // Path-only mode: just print paths
        if args.header {
            output.list("path");
        }
        for wt in worktrees {
            output.list(&wt.path.display().to_string());
        }
    } else {
        // Calculate max path length and max branch length for alignment in single pass
        let (max_path_len, max_branch_len) = worktrees.iter().fold((0, 0), |(max_p, max_b), wt| {
            let path_len = wt.path.display().to_string().len();
            let branch_len = branch_display(&wt.branch).len();
            (max_p.max(path_len), max_b.max(branch_len))
        });

        if args.header {
            print_header(&output, max_path_len, max_branch_len);
        }

        for wt in worktrees {
            display_worktree(&wt, &output, max_path_len, max_branch_len)?;
        }
    }

    Ok(())
}

fn print_header(output: &Output, max_path_len: usize, max_branch_len: usize) {
    let mut parts = vec![format!("{:width$}", "path", width = max_path_len)];
    parts.push(format!("{:width$}", "branch", width = max_branch_len));
    // Commit hash is 7 chars (short hash)
    parts.push(format!("{:width$}", "commit", width = 7));
    parts.push("status".to_string());
    output.list(&parts.join(" "));
}

fn display_worktree(
    wt: &WorktreeInfo,
    output: &Output,
    max_path_len: usize,
    max_branch_len: usize,
) -> Result<()> {
    let path_str = wt.path.display().to_string();
    let mut parts = vec![format!("{:width$}", path_str, width = max_path_len)];

    // Locked tag
    if wt.is_locked {
        parts.push("[locked]".to_string());
    }

    // Branch name (aligned)
    let branch = branch_display(&wt.branch);
    parts.push(format!("{:width$}", branch, width = max_branch_len));

    // Commit hash (first 7 chars)
    let short_hash = wt.head.chars().take(7).collect::<String>();
    parts.push(format!("{:width$}", short_hash, width = 7));

    // Get status (best effort - don't fail if worktree is missing)
    if let Ok(status) = git::worktree_status(&wt.path) {
        if status.has_uncommitted_changes {
            parts.push(uncommitted_display(&status));
        }
    }

    // Get unpushed commits (best effort)
    if let Ok(unpushed) = git::worktree_unpushed_commits(&wt.path) {
        if unpushed.has_unpushed {
            parts.push(format!("[unpushed: {} commits]", unpushed.count));
        }
    }

    output.list(&parts.join(" "));
    Ok(())
}

fn branch_display(branch: &Option<String>) -> &str {
    branch
        .as_ref()
        .and_then(|b| b.strip_prefix("refs/heads/"))
        .unwrap_or("detached")
}

fn uncommitted_display(status: &crate::git::WorktreeStatus) -> String {
    let mut parts = vec![];
    if status.modified_count > 0 {
        parts.push(format!("{} modified", status.modified_count));
    }
    if status.deleted_count > 0 {
        parts.push(format!("{} deleted", status.deleted_count));
    }
    if status.untracked_count > 0 {
        parts.push(format!("{} untracked", status.untracked_count));
    }
    format!("[uncommitted: {}]", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_display_regular() {
        let branch = Some("refs/heads/main".to_string());
        assert_eq!(branch_display(&branch), "main");
    }

    #[test]
    fn test_branch_display_detached() {
        assert_eq!(branch_display(&None), "detached");
    }

    #[test]
    fn test_uncommitted_display() {
        use crate::git::WorktreeStatus;

        let status = WorktreeStatus {
            has_uncommitted_changes: true,
            modified_count: 2,
            deleted_count: 1,
            untracked_count: 3,
        };
        let result = uncommitted_display(&status);
        assert!(result.contains("2 modified"));
        assert!(result.contains("1 deleted"));
        assert!(result.contains("3 untracked"));
    }
}
