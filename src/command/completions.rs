use crate::cli;
use crate::error::Result;

use std::io;

use clap_complete::Shell;

/// Execute the `completions` subcommand.
pub(crate) fn run(shell: Shell) -> Result<()> {
    let mut cmd = cli::build();
    clap_complete::generate(shell, &mut cmd, "gwtx", &mut io::stdout());
    Ok(())
}
