use crate::cli::ConfigCommand;
use crate::config;
use crate::error::{Error, Result};
use crate::git;

use std::fs;
use std::path::Path;

/// Execute the `config` subcommand.
pub(crate) fn run(command: Option<ConfigCommand>) -> Result<()> {
    match command {
        Some(ConfigCommand::Validate) => validate(),
        Some(ConfigCommand::Schema) => crate::command::schema(),
        Some(ConfigCommand::New { global }) => new_config(global),
        None => {
            // Show help when no subcommand is given
            use clap::CommandFactory;
            let mut cmd = crate::cli::Cli::command();
            let config_cmd = cmd
                .find_subcommand_mut("config")
                .ok_or_else(|| Error::Internal("config subcommand not found".to_string()))?;
            config_cmd.print_help()?;
            println!();
            Ok(())
        }
    }
}

/// Validate .gwtx.yaml configuration.
fn validate() -> Result<()> {
    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repository_root()?;
    config::load(&repo_root)?;

    println!("Config is valid");
    Ok(())
}

fn new_config(global: bool) -> Result<()> {
    if global {
        let path = config::global_config_path().ok_or(Error::GlobalConfigDirNotFound)?;
        let template = global_config_template();
        write_new_config(&path, &template)?;
        return Ok(());
    }

    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repository_root()?;
    let path = repo_root.join(config::CONFIG_FILE_NAME);
    let template = repo_config_template();
    write_new_config(&path, &template)?;
    Ok(())
}

fn write_new_config(path: &Path, template: &str) -> Result<()> {
    if path.exists() {
        return Err(Error::ConfigAlreadyExists {
            path: path.to_path_buf(),
        });
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, template)?;
    println!("Created config: {}", path.display());
    Ok(())
}

fn schema_url() -> String {
    "https://raw.githubusercontent.com/h-michael/gwtx/main/schema/gwtx.schema.json".to_string()
}

fn repo_config_template() -> String {
    format!(
        r#"# yaml-language-server: $schema={}

# Minimal gwtx configuration
# Copy this file to .gwtx.yaml in your repository root to get started.

# This is the simplest possible configuration with no operations.
# gwtx will create worktrees without any additional setup.
# You can add operations as needed.

# Uncomment to set global conflict handling
# defaults:
#   on_conflict: backup  # abort, skip, overwrite, backup
"#,
        schema_url()
    )
}

#[cfg(windows)]
fn global_config_template() -> String {
    format!(
        r#"# yaml-language-server: $schema={}

# Global gwtx configuration
# This file applies to all repositories and can be overridden by .gwtx.yaml.

# Allowed keys: defaults, worktree, ui, hooks.hook_shell

# defaults:
#   on_conflict: backup  # abort, skip, overwrite, backup

# worktree:
#   # path_template supports: {{branch}}, {{repository}}
#   # branch_template supports: {{commitish}}, {{repository}}, {{strftime(...)}} (e.g., {{strftime(%Y%m%d)}})
#   path_template: "../worktrees/{{repository}}-{{branch}}"
#   branch_template: "review/{{commitish}}"

# ui:
#   colors:
#     # Supported color values:
#     # - named: default, black, red, green, yellow, blue, magenta, cyan, gray,
#     #          darkgray, lightred, lightgreen, lightyellow, lightblue,
#     #          lightmagenta, lightcyan, white
#     # - RGB: "rgb(255, 85, 0)"
#     border: default
#     text: default
#     accent: cyan
#     header: default
#     footer: default
#     title: default
#     label: default
#     muted: default
#     disabled: default
#     search: default
#     preview: default
#     selection_bg: default
#     selection_fg: default
#     warning: default
#     error: default

# hooks:
#   hook_shell: "pwsh"  # Windows-only: pwsh, powershell, bash, cmd, wsl
 "#,
        schema_url()
    )
}

#[cfg(not(windows))]
fn global_config_template() -> String {
    format!(
        r#"# yaml-language-server: $schema={}

# Global gwtx configuration
# This file applies to all repositories and can be overridden by .gwtx.yaml.

# Allowed keys: defaults, worktree, ui

# defaults:
#   on_conflict: backup  # abort, skip, overwrite, backup

# worktree:
#   # path_template supports: {{branch}}, {{repository}}
#   # branch_template supports: {{commitish}}, {{repository}}, {{strftime(...)}} (e.g., {{strftime(%Y%m%d)}})
#   path_template: "../worktrees/{{repository}}-{{branch}}"
#   branch_template: "review/{{commitish}}"

# ui:
#   colors:
#     # Supported color values:
#     # - named: default, black, red, green, yellow, blue, magenta, cyan, gray,
#     #          darkgray, lightred, lightgreen, lightyellow, lightblue,
#     #          lightmagenta, lightcyan, white
#     # - RGB: "rgb(255, 85, 0)"
#     border: default
#     text: default
#     accent: cyan
#     header: default
#     footer: default
#     title: default
#     label: default
#     muted: default
#     disabled: default
#     search: default
#     preview: default
#     selection_bg: default
#     selection_fg: default
#     warning: default
#     error: default
 "#,
        schema_url()
    )
}
