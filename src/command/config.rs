use crate::cli::ConfigCommand;
use crate::config;
use crate::error::{Error, Result};
use crate::vcs;

use std::fs;
use std::path::Path;

/// Execute the `config` subcommand.
pub(crate) fn run(command: Option<ConfigCommand>) -> Result<()> {
    match command {
        Some(ConfigCommand::Validate) => validate(),
        Some(ConfigCommand::Schema) => crate::command::schema(),
        Some(ConfigCommand::New {
            global,
            path,
            override_existing,
            with_gitignore,
            without_gitignore,
        }) => new_config(
            global,
            path,
            override_existing,
            with_gitignore,
            without_gitignore,
        ),
        Some(ConfigCommand::Get { key }) => get_config_value(&key),
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

/// Validate .kabu/config.yaml configuration.
fn validate() -> Result<()> {
    let provider = vcs::get_provider()?;
    if !provider.is_inside_repo() {
        return Err(Error::NotInAnyRepo);
    }

    let repo_root = provider.repository_root()?;
    config::load(&repo_root)?;

    println!("Config is valid");
    Ok(())
}

/// Get a configuration value by key.
fn get_config_value(key: &str) -> Result<()> {
    let provider = vcs::get_provider()?;
    if !provider.is_inside_repo() {
        return Err(Error::NotInAnyRepo);
    }

    let repo_root = provider.repository_root()?;
    let cfg = config::load_merged(&repo_root)?;

    match key {
        "auto_cd.after_remove" => match cfg.auto_cd.after_remove() {
            config::AfterRemove::Main => println!("main"),
            config::AfterRemove::Select => println!("select"),
        },
        "auto_cd.after_add" => {
            if cfg.auto_cd.after_add() {
                println!("true");
            } else {
                println!("false");
            }
        }
        "on_conflict" => {
            if let Some(value) = cfg.on_conflict {
                match value {
                    config::OnConflict::Abort => println!("abort"),
                    config::OnConflict::Skip => println!("skip"),
                    config::OnConflict::Overwrite => println!("overwrite"),
                    config::OnConflict::Backup => println!("backup"),
                }
            }
        }
        _ => {
            return Err(Error::Internal(format!("Unknown config key: {}", key)));
        }
    }

    Ok(())
}

fn new_config(
    global: bool,
    custom_path: Option<std::path::PathBuf>,
    override_existing: bool,
    with_gitignore: bool,
    without_gitignore: bool,
) -> Result<()> {
    if let Some(path) = custom_path {
        let template = if global {
            global_config_template()
        } else {
            repo_config_template()
        };
        write_new_config(&path, &template, override_existing)?;
        return Ok(());
    }

    if global {
        let path = config::global_config_path().ok_or(Error::GlobalConfigDirNotFound)?;
        let template = global_config_template();
        write_new_config(&path, &template, override_existing)?;
        return Ok(());
    }

    let provider = vcs::get_provider()?;
    if !provider.is_inside_repo() {
        return Err(Error::NotInAnyRepo);
    }

    let repo_root = provider.repository_root()?;
    let kabu_dir = repo_root.join(config::CONFIG_DIR_NAME);
    let config_path = kabu_dir.join(config::CONFIG_FILE_NAME);

    // Create .kabu/ directory
    fs::create_dir_all(&kabu_dir)?;

    // Write config.yaml
    let template = repo_config_template();
    write_new_config(&config_path, &template, override_existing)?;

    // Handle .gitignore
    if with_gitignore {
        write_gitignore(&kabu_dir)?;
    } else if !without_gitignore {
        // Interactive prompt
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            if prompt_create_gitignore() {
                write_gitignore(&kabu_dir)?;
            }
        } else {
            // Non-interactive: default to creating .gitignore
            write_gitignore(&kabu_dir)?;
        }
    }

    Ok(())
}

fn prompt_create_gitignore() -> bool {
    use std::io::{BufRead, Write};

    print!("Create .kabu/.gitignore to exclude from git? [Y/n] ");
    let _ = std::io::stdout().flush();

    let stdin = std::io::stdin();
    let mut line = String::new();
    if stdin.lock().read_line(&mut line).is_err() {
        return true; // Default to yes on error
    }

    let answer = line.trim().to_lowercase();
    answer.is_empty() || answer == "y" || answer == "yes"
}

fn write_gitignore(kabu_dir: &Path) -> Result<()> {
    let path = kabu_dir.join(".gitignore");
    fs::write(&path, "/*\n")?;
    println!("Created: {}", path.display());
    Ok(())
}

fn write_new_config(path: &Path, template: &str, override_existing: bool) -> Result<()> {
    if path.exists() && !override_existing {
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
    "https://raw.githubusercontent.com/h-michael/kabu/main/schema/kabu.schema.json".to_string()
}

fn repo_config_template() -> String {
    format!(
        r#"# yaml-language-server: $schema={}

# kabu configuration
# See: https://github.com/h-michael/kabu

# Conflict handling for file operations
# on_conflict: backup  # abort, skip, overwrite, backup

# Auto cd settings (requires shell integration)
# auto_cd:
#   after_add: true    # cd to new worktree after creation (default: true)
#   after_remove: main # cd target after removing current worktree (default: main)

# Worktree path/branch templates
# worktree:
#   # path_template supports: {{{{branch}}}}, {{{{repository}}}}
#   # branch_template supports: {{{{commitish}}}}, {{{{repository}}}}, {{{{strftime(...)}}}} (e.g., {{{{strftime(%Y%m%d)}}}})
#   path_template: "../worktrees/{{{{branch}}}}"
#   branch_template: "{{{{commitish}}}}"

# Create directories in new worktree
# mkdir:
#   - path: build
#   - path: tmp
#     description: Temporary files

# Create symlinks from repo root to worktree
# link:
#   - source: .env.local
#   - source: "fixtures/*"
#     skip_tracked: true
#     description: Link untracked test fixtures

# Copy files from repo root to worktree
# copy:
#   - source: config.template.json
#     target: config.json

# Hooks (requires trust via `kabu trust`)
# hooks:
#   pre_add:
#     - command: echo "Creating {{{{worktree_name}}}}"
#   post_add:
#     - command: npm install
#       description: Install dependencies
#   pre_remove:
#     - command: echo "Removing {{{{worktree_name}}}}"
#   post_remove:
#     - command: ./scripts/cleanup.sh
"#,
        schema_url()
    )
}

#[cfg(windows)]
fn global_config_template() -> String {
    format!(
        r#"# yaml-language-server: $schema={}

# Global kabu configuration
# This file applies to all repositories and can be overridden by .kabu/config.yaml.

# Allowed keys: on_conflict, auto_cd, worktree, ui, hooks.hook_shell

# on_conflict: backup  # abort, skip, overwrite, backup

# Auto cd settings (requires shell integration)
# auto_cd:
#   after_add: true    # cd to new worktree after creation (default: true)
#   after_remove: main # cd target after removing current worktree (default: main)

# worktree:
#   # path_template supports: {{{{branch}}}}, {{{{repository}}}}
#   # branch_template supports: {{{{commitish}}}}, {{{{repository}}}}, {{{{strftime(...)}}}} (e.g., {{{{strftime(%Y%m%d)}}}})
#   path_template: "../worktrees/{{{{repository}}}}-{{{{branch}}}}"
#   branch_template: "review/{{{{commitish}}}}"

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

# Global kabu configuration
# This file applies to all repositories and can be overridden by .kabu/config.yaml.

# Allowed keys: on_conflict, auto_cd, worktree, ui

# on_conflict: backup  # abort, skip, overwrite, backup

# Auto cd settings (requires shell integration)
# auto_cd:
#   after_add: true    # cd to new worktree after creation (default: true)
#   after_remove: main # cd target after removing current worktree (default: main)

# worktree:
#   # path_template supports: {{{{branch}}}}, {{{{repository}}}}
#   # branch_template supports: {{{{commitish}}}}, {{{{repository}}}}, {{{{strftime(...)}}}} (e.g., {{{{strftime(%Y%m%d)}}}})
#   path_template: "../worktrees/{{{{repository}}}}-{{{{branch}}}}"
#   branch_template: "review/{{{{commitish}}}}"

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
