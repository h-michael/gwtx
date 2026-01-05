use crate::cli;
use crate::error::Result;

use std::io;

use clap_mangen::Man;

/// Execute the `man` subcommand.
pub(crate) fn run() -> Result<()> {
    let cmd = cli::build();
    Man::new(cmd).render(&mut io::stdout())?;
    Ok(())
}
