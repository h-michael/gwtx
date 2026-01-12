use crate::cli::ListArgs;
use crate::color::{ColorConfig, ColorScheme};
use crate::error::{Error, Result};
use crate::git::{self, WorktreeInfo};
use crate::output::Output;

pub(crate) fn run(args: ListArgs, color: ColorConfig) -> Result<()> {
    let output = Output::new(false, color);

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
            display_worktree(&wt, &output, max_path_len, max_branch_len, color)?;
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
    color: ColorConfig,
) -> Result<()> {
    let path_str = wt.path.display().to_string();
    let branch = branch_display(&wt.branch);
    let short_hash = wt.head.chars().take(7).collect::<String>();

    // Format fields with padding first, then apply colors
    let path_field = format!("{:width$}", path_str, width = max_path_len);
    let branch_field = format!("{:width$}", branch, width = max_branch_len);
    let hash_field = format!("{:width$}", short_hash, width = 7);

    let mut parts = Vec::new();

    // Path (white/default color)
    parts.push(path_field);

    // Locked tag
    if wt.is_locked {
        if color.is_enabled() {
            parts.push(ColorScheme::locked("[locked]"));
        } else {
            parts.push("[locked]".to_string());
        }
    }

    // Branch name
    if color.is_enabled() {
        parts.push(ColorScheme::branch(&branch_field));
    } else {
        parts.push(branch_field);
    }

    // Commit hash
    if color.is_enabled() {
        parts.push(ColorScheme::hash(&hash_field));
    } else {
        parts.push(hash_field);
    }

    // Get status (best effort - don't fail if worktree is missing)
    if let Ok(status) = git::worktree_status(&wt.path) {
        if status.has_uncommitted_changes {
            let status_str = uncommitted_display(&status);
            if color.is_enabled() {
                parts.push(ColorScheme::status(&status_str));
            } else {
                parts.push(status_str);
            }
        }
    }

    // Get unpushed commits (best effort)
    if let Ok(unpushed) = git::worktree_unpushed_commits(&wt.path) {
        if unpushed.has_unpushed {
            let unpushed_str = format!("[unpushed: {} commits]", unpushed.count);
            if color.is_enabled() {
                parts.push(ColorScheme::unpushed(&unpushed_str));
            } else {
                parts.push(unpushed_str);
            }
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

fn uncommitted_display(_status: &crate::git::WorktreeStatus) -> String {
    "*".to_string()
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
        assert_eq!(result, "*");
    }
}
