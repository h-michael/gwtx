use crate::cli::ConfigCommand;
use crate::config;
use crate::error::{Error, Result};
use crate::git;

/// Execute the `config` subcommand.
pub(crate) fn run(command: Option<ConfigCommand>) -> Result<()> {
    match command {
        Some(ConfigCommand::Validate) => validate(),
        Some(ConfigCommand::Schema) => crate::command::schema(),
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

/// Validate .gwtx.toml configuration.
fn validate() -> Result<()> {
    if !git::is_inside_repo() {
        return Err(Error::NotInGitRepo);
    }

    let repo_root = git::repository_root()?;
    config::load(&repo_root)?;

    println!("Config is valid");
    Ok(())
}
