use crate::cli::ListArgs;
use crate::color::{ColorConfig, ColorScheme};
use crate::error::{Error, Result};
use crate::git::{self, UnpushedCommits, WorktreeInfo, WorktreeStatus};
use crate::output::Output;

/// Enriched worktree info for display purposes.
struct DisplayWorktree {
    path: String,
    branch: String,
    head: String,
    status: WorktreeStatus,
    unpushed: UnpushedCommits,
    upstream: Option<String>,
    is_locked: bool,
}

pub(crate) fn run(args: ListArgs, color: ColorConfig) -> Result<()> {
    let output = Output::new(false, color);

    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let worktrees = git::list_worktrees()?;

    if args.path_only {
        print_path_only(&worktrees, &output, args.header);
        return Ok(());
    }

    // Pre-fetch all data before printing
    let display_data = enrich_worktrees(&worktrees)?;

    // Calculate max path length and max branch length for alignment
    let (max_path, max_branch) = display_data.iter().fold((0, 0), |(max_p, max_b), wt| {
        (max_p.max(wt.path.len()), max_b.max(wt.branch.len()))
    });

    if args.header {
        print_header(&output, max_path, max_branch, color);
    }

    for wt in display_data {
        display_worktree(&wt, &output, max_path, max_branch, color);
    }

    Ok(())
}

fn print_path_only(worktrees: &[WorktreeInfo], output: &Output, header: bool) {
    if header {
        output.list("PATH");
    }
    for wt in worktrees {
        output.list(&wt.path.display().to_string());
    }
}

fn enrich_worktrees(worktrees: &[WorktreeInfo]) -> Result<Vec<DisplayWorktree>> {
    let mut display_data = Vec::new();
    for wt in worktrees {
        // Get status and unpushed commits (best effort)
        let status = git::worktree_status(&wt.path).unwrap_or_default();
        let unpushed = git::worktree_unpushed_commits(&wt.path).unwrap_or_default();
        let upstream = git::get_upstream_branch(&wt.path).unwrap_or(None);

        display_data.push(DisplayWorktree {
            path: wt.path.display().to_string(),
            branch: branch_display(&wt.branch),
            head: wt.head.clone(),
            status,
            unpushed,
            upstream,
            is_locked: wt.is_locked,
        });
    }
    Ok(display_data)
}

fn print_header(output: &Output, max_path: usize, max_branch: usize, color: ColorConfig) {
    let path = "PATH";
    let branch = "BRANCH";
    let commit = "COMMIT";
    let status = "STATUS";

    let line = format!(
        "{path:<p_width$} {branch:<b_width$} {commit:<c_width$} {status}",
        p_width = max_path,
        b_width = max_branch,
        c_width = 7,
    );
    if color.is_enabled() {
        output.list(&ColorScheme::header(&line));
    } else {
        output.list(&line);
    }
}

fn display_worktree(
    wt: &DisplayWorktree,
    output: &Output,
    max_path: usize,
    max_branch: usize,
    color: ColorConfig,
) {
    let short_hash = wt.head.chars().take(7).collect::<String>();
    let status_str = format_status(&wt.status, &wt.unpushed, &wt.upstream, wt.is_locked);

    let line = if color.is_enabled() {
        format!(
            "{path:<p_width$} {branch} {hash} {status}",
            path = wt.path,
            p_width = max_path,
            branch = ColorScheme::branch(&format!("{:<width$}", wt.branch, width = max_branch)),
            hash = ColorScheme::hash(&format!("{:<width$}", short_hash, width = 7)),
            status = ColorScheme::dimmed(&status_str),
        )
    } else {
        format!(
            "{path:<p_width$} {branch:<b_width$} {hash:<c_width$} {status}",
            path = wt.path,
            p_width = max_path,
            branch = wt.branch,
            b_width = max_branch,
            hash = short_hash,
            c_width = 7,
            status = status_str,
        )
    };

    output.list(&line);
}

fn branch_display(branch: &Option<String>) -> String {
    branch
        .as_ref()
        .and_then(|b| b.strip_prefix("refs/heads/"))
        .unwrap_or("(detached)")
        .to_string()
}

fn format_status(
    status: &crate::git::WorktreeStatus,
    unpushed: &crate::git::UnpushedCommits,
    upstream: &Option<String>,
    is_locked: bool,
) -> String {
    let mut parts = Vec::new();
    if status.has_uncommitted_changes {
        let mut changes = Vec::new();
        if status.modified_count > 0 {
            changes.push("modified");
        }
        if status.deleted_count > 0 {
            changes.push("deleted");
        }
        if status.untracked_count > 0 {
            changes.push("untracked");
        }
        if !changes.is_empty() {
            parts.push(changes.join(", "));
        }
    }

    if unpushed.has_unpushed {
        let unpushed_str = if let Some(upstream_name) = upstream {
            format!("unpushed: {} (vs {})", unpushed.count, upstream_name)
        } else {
            format!("unpushed: {}", unpushed.count)
        };
        parts.push(unpushed_str);
    }

    if is_locked {
        parts.push("locked".to_string());
    }

    if parts.is_empty() {
        "up to date".to_string()
    } else {
        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::UnpushedCommits;

    #[test]
    fn test_branch_display_regular() {
        let branch = Some("refs/heads/main".to_string());
        assert_eq!(branch_display(&branch), "main");
    }

    #[test]
    fn test_branch_display_detached() {
        assert_eq!(branch_display(&None), "(detached)");
    }

    #[test]
    fn test_format_status_all() {
        use crate::git::WorktreeStatus;

        let status = WorktreeStatus {
            has_uncommitted_changes: true,
            modified_count: 2,
            deleted_count: 1,
            untracked_count: 3,
        };
        let unpushed = UnpushedCommits {
            has_unpushed: true,
            count: 5,
        };
        let upstream = Some("origin/main".to_string());
        let result = format_status(&status, &unpushed, &upstream, true);
        assert_eq!(
            result,
            "modified, deleted, untracked | unpushed: 5 (vs origin/main) | locked"
        );
    }

    #[test]
    fn test_format_status_clean() {
        use crate::git::WorktreeStatus;

        let status = WorktreeStatus {
            has_uncommitted_changes: false,
            modified_count: 0,
            deleted_count: 0,
            untracked_count: 0,
        };
        let unpushed = UnpushedCommits {
            has_unpushed: false,
            count: 0,
        };
        let result = format_status(&status, &unpushed, &None, false);
        assert_eq!(result, "up to date");
    }
}
